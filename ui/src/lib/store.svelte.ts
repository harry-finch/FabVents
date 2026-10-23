// État de l'application : le projet (avec annuler / rétablir), le moteur,
// et l'état de l'interface (outil, onglet, sélection, vue, unités).
//
// Toute modification du projet passe par `edit()` (une étape d'historique,
// fusionnée si la même valeur est modifiée de suite) ou par un geste
// `begin()` … `end()` (glisser une poignée, faire défiler un libellé) : une
// seule étape quelle que soit la durée du geste. Le recalcul et la
// sauvegarde automatique suivent l'état (voir App.svelte).
//
// Le canevas édite des « éléments » : les contrôleurs du canal affiché, ou
// les tracés de la disposition Chemin dans l'onglet Chemin.

import { EngineClient } from './engine';
import { t } from './i18n.svelte';
import { rasterFromFile } from './image';
import { scaleGeometry, toCircle, toEllipse, toPolygon, translate } from './geometry';
import {
  baseSpacing,
  CHANNELS,
  GOLDEN,
  type Channel,
  type Controller,
  type CtrlGeometry,
  type Curve,
  type Feature,
  type FeatureShape,
  type FrameData,
  type ImportedOutline,
  type Influence,
  type Layout,
  type LayoutMode,
  type Project,
  type Tab,
  type Tool,
  type Vec2,
} from './types';
import type { UnitSystem } from './format';

const PROJECT_KEY = 'fabvents.project.v1';
const PREFS_KEY = 'fabvents.prefs.v1';
const HISTORY_MAX = 200;
const MIN_SCALE = 0.05;
const MAX_SCALE = 400;
const MERGE_MS = 900;
/** Au-delà de ce temps de calcul (ms), le motif n'est recalculé qu'au relâchement. */
export const HEAVY_MS = 60;

interface Prefs {
  units: UnitSystem;
  tab: Tab;
  scalePattern: boolean;
}

function readPrefs(): Prefs {
  const fallback: Prefs = { units: 'mm', tab: 'size', scalePattern: false };
  try {
    const v = JSON.parse(localStorage.getItem(PREFS_KEY) ?? 'null');
    return { ...fallback, ...(v ?? {}) };
  } catch {
    return fallback;
  }
}

function savedProject(): string | null {
  try {
    return localStorage.getItem(PROJECT_KEY);
  } catch {
    return null;
  }
}

export interface View {
  /** Pixels par millimètre. */
  scale: number;
  /** Position à l'écran (px CSS) de l'origine de la plaque. */
  ox: number;
  oy: number;
}

/** Élément éditable sur le canevas. */
export type ItemKey = { kind: 'ctrl'; id: number } | { kind: 'path'; index: number } | { kind: 'feature'; id: number };
export interface Item {
  key: ItemKey;
  geometry: CtrlGeometry;
  /** Influence sur le canal affiché (null pour un tracé de chemin). */
  influence: Influence | null;
  /** Tracé fermé dont l'intérieur reçoit la cible. */
  fill: boolean;
}

const keyNum = (k: ItemKey) => (k.kind === 'path' ? k.index : k.id);
export const sameKey = (a: ItemKey | null, b: ItemKey | null) => !!a && !!b && a.kind === b.kind && keyNum(a) === keyNum(b);

const curveGeometry = (c: Curve): CtrlGeometry => ({ type: 'curve', points: c.points, closed: c.closed, fillets: c.fillets });

/** Forme d'un élément de plaque, vue comme une géométrie du canevas. */
export function featureGeometry(f: FeatureShape): CtrlGeometry {
  if (f.type === 'circle') return { type: 'circle', center: f.center, radius: f.diameter / 2 };
  if (f.type === 'rectangle') return { type: 'rect', center: f.center, width: f.width, height: f.height, corner: f.corner };
  return curveGeometry(f);
}

function toFeatureShape(g: CtrlGeometry): FeatureShape | null {
  if (g.type === 'circle') return { type: 'circle', center: g.center, diameter: g.radius * 2 };
  if (g.type === 'rect') return { type: 'rectangle', center: g.center, width: g.width, height: g.height, corner: g.corner };
  if (g.type === 'curve' && g.closed) return { type: 'curve', points: g.points, closed: true, fillets: g.fillets?.length ? g.fillets : undefined };
  return null;
}

class Store {
  // ---- Projet
  project = $state<Project | null>(null);
  #undo: string[] = [];
  #redo: string[] = [];
  canUndo = $state(false);
  canRedo = $state(false);
  #lastEdit = { key: '', at: 0 };
  #gesture: string | null = null;
  /** Un geste (glisser) est en cours. */
  gesturing = $state(false);
  /** Temps du premier calcul reçu pendant le geste en cours. */
  #gestureMs = $state<number | null>(null);

  // ---- Moteur
  engine = new EngineClient();
  engineState = $state<'loading' | 'ready' | 'failed'>('loading');
  engineVersion = $state('');
  frame = $state.raw<FrameData | null>(null);
  frameError = $state<string | null>(null);
  #example = '';
  #blank = '';

  // ---- Interface
  tool = $state<Tool>('select');
  tab = $state<Tab>(readPrefs().tab);
  units = $state<UnitSystem>(readPrefs().units);
  /** Redimensionner la plaque met aussi à l'échelle tailles, pas et portées. */
  scalePattern = $state(readPrefs().scalePattern);
  selection = $state<ItemKey | null>(null);
  /** Nœud sélectionné d'un tracé. */
  node = $state<number | null>(null);
  view = $state<View>({ scale: 3, ox: 40, oy: 500 });
  /** Taille du canevas (px CSS), tenue à jour par le canevas. */
  viewport = $state({ w: 0, h: 0 });
  /** Incrémenté pour demander au canevas de recadrer la plaque. */
  fitSeq = $state(0);
  /** Espace maintenu : glisser déplace la vue. */
  spacePan = $state(false);
  /** Vrai dès que l'utilisateur zoome ou déplace la vue : le canevas cesse
   *  alors de recadrer la plaque quand sa taille change. */
  viewTouched = false;
  cursor = $state<Vec2 | null>(null);
  help = $state(false);
  toasts = $state<{ id: number; text: string }[]>([]);
  #toastSeq = 0;

  constructor() {
    this.engine.onReady = (version, example, blank) => {
      this.engineVersion = version;
      this.engineState = 'ready';
      this.#example = example;
      this.#blank = blank;
      void this.#restore();
    };
    this.engine.onFailed = () => (this.engineState = 'failed');
    this.engine.onFrame = (f) => {
      this.frame = f;
      this.frameError = null;
      if (this.gesturing && this.#gestureMs == null) this.#gestureMs = f.ms;
    };
    this.engine.onFrameError = (code) => (this.frameError = code);
  }

  async #restore() {
    const saved = savedProject();
    if (saved) {
      try {
        this.project = JSON.parse(await this.engine.read(saved));
        this.#checkTab();
        this.fitSeq++;
        return;
      } catch {
        this.toast(t('file.restoreFailed'));
      }
    }
    this.project = JSON.parse(this.#example);
    this.#checkTab();
    this.fitSeq++;
  }

  /** Projet courant (l'interface principale n'est montrée qu'une fois chargé). */
  get p(): Project {
    return this.project!;
  }

  // ---- Historique

  #snapshot(): string {
    return JSON.stringify(this.project);
  }

  #push(snapshot: string) {
    this.#undo.push(snapshot);
    if (this.#undo.length > HISTORY_MAX) this.#undo.shift();
    this.#redo = [];
    this.canUndo = true;
    this.canRedo = false;
  }

  /** Modifie le projet en une étape d'historique. `key` : des modifications
   *  successives de la même valeur sont fusionnées. */
  edit(fn: (p: Project) => void, key = '') {
    if (!this.project) return;
    const now = performance.now();
    const merge = key !== '' && key === this.#lastEdit.key && now - this.#lastEdit.at < MERGE_MS;
    const before = this.#snapshot();
    fn(this.project);
    if (this.#snapshot() === before) return;
    if (!merge && this.#gesture == null) this.#push(before);
    this.#lastEdit = { key, at: now };
  }

  /** Début d'un geste continu (glisser) : une seule étape d'historique. */
  begin() {
    if (this.project && this.#gesture == null) {
      this.#gesture = this.#snapshot();
      this.gesturing = true;
      this.#gestureMs = null;
    }
  }

  end() {
    if (this.#gesture == null) return;
    const before = this.#gesture;
    this.#gesture = null;
    this.gesturing = false;
    if (this.#snapshot() !== before) this.#push(before);
    this.#lastEdit = { key: '', at: 0 };
  }

  /** Motif lourd : si le premier calcul du geste a été lent, les suivants
   *  attendent le relâchement (les poignées restent fluides de toute façon). */
  get deferred(): boolean {
    return this.gesturing && this.#gestureMs != null && this.#gestureMs > HEAVY_MS;
  }

  #replace(json: string) {
    this.project = JSON.parse(json);
    if (this.selection && !this.itemExists(this.selection)) this.select(null);
    this.#checkTab();
  }

  undo() {
    const prev = this.#undo.pop();
    if (prev == null) return;
    this.#redo.push(this.#snapshot());
    this.#replace(prev);
    this.canUndo = this.#undo.length > 0;
    this.canRedo = true;
  }

  redo() {
    const next = this.#redo.pop();
    if (next == null) return;
    this.#undo.push(this.#snapshot());
    this.#replace(next);
    this.canUndo = true;
    this.canRedo = this.#redo.length > 0;
  }

  // ---- Projet : nouveau, ouvrir, enregistrer

  newProject(example = false) {
    if (!this.project) return;
    this.#push(this.#snapshot());
    this.#replace(example ? this.#example : this.#blank);
    this.select(null);
    this.fitSeq++;
  }

  /** Remplace le projet par le contenu d'un fichier (JSON ou SVG FabVents). */
  async openText(text: string, name: string) {
    try {
      const json = await this.engine.read(text);
      if (this.project) this.#push(this.#snapshot());
      this.#replace(json);
      this.select(null);
      this.fitSeq++;
      this.toast(t('file.opened', { name }));
    } catch (e) {
      const code = e instanceof Error ? e.message : '';
      // Un SVG sans projet FabVents, ou un DXF : c'est un contour de plaque.
      if (code === 'project.svg_without_project' || /\.dxf$/i.test(name)) {
        await this.importOutlineText(text, name);
        return;
      }
      this.toast(t('file.openFailed', { name, reason: t(`errors.${code}` as never) }));
    }
  }

  persist() {
    if (!this.project) return;
    try {
      localStorage.setItem(PROJECT_KEY, this.#snapshot());
      localStorage.setItem(PREFS_KEY, JSON.stringify({ units: this.units, tab: this.tab, scalePattern: this.scalePattern }));
    } catch {
      /* stockage indisponible ou plein */
    }
  }

  // ---- Onglets

  /** Onglets du canevas : Taille, Espacement (si la disposition le suit),
   *  Angle, Chemin (disposition Chemin), Plaque (fixations, zones réservées). */
  get tabs(): Tab[] {
    if (!this.project) return ['size', 'angle'];
    const l = this.p.layout;
    const out: Tab[] = CHANNELS.filter((c) => c !== 'spacing' || baseSpacing(l) != null);
    if (l.mode === 'path') out.push('path');
    out.push('plate');
    return out;
  }

  /** Canal affiché (null dans les onglets Chemin et Plaque). */
  get channel(): Channel | null {
    return this.tab === 'path' || this.tab === 'plate' ? null : this.tab;
  }

  setTab(tab: Tab) {
    this.tab = tab;
    if (this.selection && !this.items.some((i) => sameKey(i.key, this.selection))) this.select(null);
  }

  #checkTab() {
    if (this.project && !this.tabs.includes(this.tab)) this.tab = 'size';
  }

  // ---- Éléments du canevas et sélection

  get items(): Item[] {
    if (!this.project) return [];
    const ch = this.channel;
    if (this.tab === 'plate') {
      return (this.p.features ?? []).map((f) => ({ key: { kind: 'feature', id: f.id }, geometry: featureGeometry(f.shape), influence: null, fill: false }));
    }
    if (!ch) {
      const l = this.p.layout;
      return l.mode === 'path'
        ? l.paths.map((c, index) => ({ key: { kind: 'path', index }, geometry: curveGeometry(c), influence: null, fill: false }))
        : [];
    }
    return this.p.controllers
      .filter((c) => c[ch])
      .map((c) => ({ key: { kind: 'ctrl', id: c.id }, geometry: c.geometry, influence: c[ch]!, fill: !!c.fill }));
  }

  itemExists(k: ItemKey): boolean {
    if (!this.project) return false;
    if (k.kind === 'ctrl') return this.p.controllers.some((c) => c.id === k.id);
    if (k.kind === 'feature') return (this.p.features ?? []).some((f) => f.id === k.id);
    const l = this.p.layout;
    return l.mode === 'path' && k.index < l.paths.length;
  }

  isSelected(k: ItemKey): boolean {
    return sameKey(this.selection, k);
  }

  select(k: ItemKey | null, node: number | null = null) {
    this.selection = k;
    this.node = node;
  }

  get selectedController(): Controller | null {
    const s = this.selection;
    return s?.kind === 'ctrl' ? (this.project?.controllers.find((c) => c.id === s.id) ?? null) : null;
  }

  get selectedFeature(): Feature | null {
    const s = this.selection;
    return s?.kind === 'feature' ? ((this.project?.features ?? []).find((f) => f.id === s.id) ?? null) : null;
  }

  /** Géométrie de l'élément sélectionné (contrôleur, tracé ou élément de plaque). */
  get selectedGeometry(): CtrlGeometry | null {
    const s = this.selection;
    if (!s || !this.project) return null;
    if (s.kind === 'ctrl') return this.selectedController?.geometry ?? null;
    if (s.kind === 'feature') {
      const f = this.selectedFeature;
      return f ? featureGeometry(f.shape) : null;
    }
    const l = this.p.layout;
    return l.mode === 'path' && l.paths[s.index] ? curveGeometry(l.paths[s.index]) : null;
  }

  /** Remplace la géométrie d'un élément (dans un `edit` ou un geste). */
  setGeometry(k: ItemKey, g: CtrlGeometry, key = '') {
    this.edit((p) => {
      if (k.kind === 'ctrl') {
        const c = p.controllers.find((x) => x.id === k.id);
        if (c) c.geometry = g;
      } else if (k.kind === 'feature') {
        const f = (p.features ?? []).find((x) => x.id === k.id);
        const shape = toFeatureShape(g);
        if (f && shape) f.shape = shape;
      } else if (p.layout.mode === 'path' && g.type === 'curve' && p.layout.paths[k.index]) {
        p.layout.paths[k.index] = { points: g.points, closed: g.closed, fillets: g.fillets?.length ? g.fillets : undefined };
      }
    }, key);
  }

  /** Ajoute un contrôleur sur le canal affiché, un tracé dans l'onglet
   *  Chemin, ou un élément dans l'onglet Plaque (point : trou de fixation ;
   *  tracé fermé : zone réservée). */
  addItem(geometry: CtrlGeometry) {
    const ch = this.channel;
    if (this.tab === 'plate') {
      const shape: FeatureShape | null =
        geometry.type === 'point' ? { type: 'circle', center: geometry.at, diameter: 4 } : toFeatureShape(geometry);
      if (!shape) {
        this.toast(t('features.closedOnly'));
        return;
      }
      const cut = geometry.type === 'point';
      this.addFeature({ shape, cut, clearance: cut ? 2 : 1 });
      return;
    }
    if (!ch) {
      if (geometry.type !== 'curve') return;
      let index = 0;
      this.edit((p) => {
        if (p.layout.mode !== 'path') return;
        index = p.layout.paths.length;
        p.layout.paths.push({ points: geometry.points, closed: geometry.closed });
      });
      this.select({ kind: 'path', index });
      return;
    }
    let id = 0;
    this.edit((p) => {
      id = p.controllers.reduce((m, c) => Math.max(m, c.id), 0) + 1;
      const c: Controller = { id, geometry };
      c[ch] = this.defaultInfluence(ch);
      if (geometry.type === 'image') c[ch]!.reach = 1;
      p.controllers.push(c);
    });
    this.select({ kind: 'ctrl', id });
  }

  addFeature(f: Omit<Feature, 'id'>) {
    let id = 0;
    this.edit((p) => {
      p.features ??= [];
      id = p.features.reduce((m, x) => Math.max(m, x.id), 0) + 1;
      p.features.push({ ...f, id });
    });
    this.select({ kind: 'feature', id });
  }

  removeSelected() {
    const s = this.selection;
    if (!s) return;
    this.edit((p) => {
      if (s.kind === 'ctrl') p.controllers = p.controllers.filter((c) => c.id !== s.id);
      else if (s.kind === 'feature') p.features = (p.features ?? []).filter((f) => f.id !== s.id);
      else if (p.layout.mode === 'path') p.layout.paths.splice(s.index, 1);
    });
    this.select(null);
  }

  duplicateSelected() {
    const s = this.selection;
    const g = this.selectedGeometry;
    if (!s || !g) return;
    const off = (v: Vec2) => ({ x: v.x + 10, y: v.y - 10 });
    const moved: CtrlGeometry = JSON.parse(JSON.stringify(g));
    if ('at' in moved) moved.at = off(moved.at);
    if ('center' in moved) moved.center = off(moved.center);
    if (moved.type === 'line') {
      moved.a = off(moved.a);
      moved.b = off(moved.b);
    }
    if (moved.type === 'curve') moved.points = moved.points.map(off);
    if (s.kind === 'path') {
      this.addItem(moved);
      return;
    }
    if (s.kind === 'feature') {
      const f = this.selectedFeature!;
      const shape = toFeatureShape(moved);
      if (shape) this.addFeature({ shape, cut: f.cut, clearance: f.clearance });
      return;
    }
    const src = this.selectedController!;
    let id = 0;
    this.edit((p) => {
      id = p.controllers.reduce((m, c) => Math.max(m, c.id), 0) + 1;
      p.controllers.push({ ...JSON.parse(JSON.stringify(src)), id, geometry: moved });
    });
    this.select({ kind: 'ctrl', id });
  }

  /** Convertit le tracé fermé sélectionné en cercle, ellipse ou polygone régulier. */
  convertSelected(to: 'circle' | 'ellipse' | 'polygon') {
    const g = this.selectedGeometry;
    const s = this.selection;
    if (!g || g.type !== 'curve' || !g.closed || s?.kind !== 'ctrl') return;
    const next = to === 'circle' ? toCircle(g) : to === 'ellipse' ? toEllipse(g) : toPolygon(g);
    this.setGeometry(s, next);
    this.node = null;
  }

  defaultInfluence(ch: Channel): Influence {
    const p = this.p;
    const ext = this.plateExtent();
    const reach = Math.max(5, Math.round(0.3 * Math.min(ext.x, ext.y)));
    if (ch === 'size') return { reach, target: Math.round(p.shape.size * 16) / 10, hard_edge: false };
    if (ch === 'spacing') return { reach, target: Math.round((baseSpacing(p.layout) ?? 8) * 5) / 10, hard_edge: false };
    return { reach, target: p.shape.rotation + 45, hard_edge: false };
  }

  /** Unifier : donne à tous les contrôleurs du canal la portée ou la cible indiquée. */
  unify(ch: Channel, field: 'reach' | 'target', value: number) {
    this.edit((p) => {
      for (const c of p.controllers) {
        const inf = c[ch];
        if (inf && c.geometry.type !== 'image') inf[field] = value;
        else if (inf && field === 'target') inf.target = value;
      }
    });
    this.toast(t(field === 'reach' ? 'ctrl.unifiedReach' : 'ctrl.unifiedTarget'));
  }

  /** Ajoute un contrôleur Image à partir d'un fichier, ajusté à la plaque. */
  async addImage(file: File) {
    try {
      const raster = await rasterFromFile(file);
      const ext = this.plateExtent();
      const k = Math.min(ext.x / raster.width, ext.y / raster.height);
      const width = Math.round(raster.width * k * 10) / 10;
      const height = Math.round(raster.height * k * 10) / 10;
      const at = { x: Math.round(((ext.x - width) / 2) * 10) / 10, y: Math.round(((ext.y - height) / 2) * 10) / 10 };
      if (!this.channel) this.setTab('size');
      this.addItem({ type: 'image', at, width, height, raster });
      this.toast(t('ctrl.imageAdded', { name: file.name }));
    } catch {
      this.toast(t('ctrl.imageFailed', { name: file.name }));
    }
  }

  // ---- Contour de la plaque, gabarits

  /** Charge un gabarit de grille de ventilateur (nouveau projet, annulable). */
  async applyTemplate(size: number) {
    try {
      const json = await this.engine.fanTemplate(size);
      if (this.project) this.#push(this.#snapshot());
      this.#replace(json);
      this.select(null);
      this.fitSeq++;
      this.toast(t('templates.loaded', { size }));
    } catch {
      this.toast(t('templates.failed'));
    }
  }

  /** Contour importé (SVG, DXF) : devient le contour de la plaque, ses
   *  contours intérieurs des découpes. */
  async importOutline(file: File) {
    await this.importOutlineText(await file.text(), file.name);
  }

  async importOutlineText(text: string, name: string) {
    const file = { name };
    try {
      const r: ImportedOutline = JSON.parse(await this.engine.importOutline(text));
      const round = (v: Vec2) => ({ x: Math.round(v.x * 1000) / 1000, y: Math.round(v.y * 1000) / 1000 });
      this.edit((p) => {
        p.plate.outline = 'custom';
        p.plate.custom = r.outer.map(round);
        p.plate.width = Math.round(r.width * 1000) / 1000;
        p.plate.height = Math.round(r.height * 1000) / 1000;
        p.features ??= [];
        let id = p.features.reduce((m, x) => Math.max(m, x.id), 0);
        for (const c of r.inner) p.features.push({ id: ++id, shape: { type: 'curve', points: c.map(round), closed: true }, cut: true, clearance: 2 });
      });
      this.fitSeq++;
      this.toast(t('plate.imported', { name: file.name, n: r.inner.length }));
    } catch (e) {
      const code = e instanceof Error ? e.message : '';
      this.toast(t('file.openFailed', { name: file.name, reason: t(`errors.${code}` as never) }));
    }
  }

  /** Le tracé fermé sélectionné (onglet Plaque) devient le contour de la plaque. */
  useAsOutline() {
    const f = this.selectedFeature;
    if (!f || f.shape.type !== 'curve') return;
    const pts = f.shape.points;
    const x0 = Math.min(...pts.map((p) => p.x));
    const y0 = Math.min(...pts.map((p) => p.y));
    const x1 = Math.max(...pts.map((p) => p.x));
    const y1 = Math.max(...pts.map((p) => p.y));
    const d = { x: -x0, y: -y0 };
    const id = f.id;
    this.edit((p) => {
      p.features = (p.features ?? []).filter((x) => x.id !== id);
      // Tout le projet suit, pour que le contour parte de l'origine.
      for (const c of p.controllers) c.geometry = translate(c.geometry, d);
      for (const x of p.features) {
        const s = toFeatureShape(translate(featureGeometry(x.shape), d));
        if (s) x.shape = s;
      }
      if (p.layout.mode === 'path') {
        p.layout.paths = p.layout.paths.map((c) => ({ ...c, points: c.points.map((q) => ({ x: q.x + d.x, y: q.y + d.y })) }));
      }
      if (p.plate.zone && p.plate.zone.type !== 'plate') p.plate.zone.center = { x: p.plate.zone.center.x + d.x, y: p.plate.zone.center.y + d.y };
      p.plate.outline = 'custom';
      p.plate.custom = pts.map((q) => ({ x: q.x + d.x, y: q.y + d.y }));
      p.plate.width = x1 - x0;
      p.plate.height = y1 - y0;
    });
    this.select(null);
    this.fitSeq++;
  }

  // ---- Disposition et plaque

  /** Change de disposition en gardant une densité voisine. */
  setLayoutMode(mode: LayoutMode) {
    const cur = this.p.layout;
    if (cur.mode === mode) return;
    const s = cur.mode === 'grid' ? cur.pitch_x : cur.mode === 'cross_hatch' ? cur.step : cur.spacing;
    const off = 'offset_x' in cur ? { offset_x: cur.offset_x, offset_y: cur.offset_y } : { offset_x: 0, offset_y: 0 };
    const next: Record<LayoutMode, Layout> = {
      grid: { mode: 'grid', pattern: 'hexagonal', pitch_x: s, pitch_y: s, angle: 0, row_twist: 0, col_twist: 0, ...off },
      random: { mode: 'random', spacing: s, seed: 1 },
      spiral: { mode: 'spiral', pitch: s, spacing: s, start: 0, clockwise: false, orient: true, ...off },
      concentric: { mode: 'concentric', pitch: s, spacing: s, start: 0, phase: 0, orient: true, ...off },
      fibonacci: { mode: 'fibonacci', spacing: s, divergence: GOLDEN, orient: false, ...off },
      cross_hatch: { mode: 'cross_hatch', step: s, line_spacing: s, angle: 45, cross: 90, ...off },
      path: { mode: 'path', paths: [], spacing: s, rows: 1, row_gap: s, orient: true },
    };
    this.edit((p) => {
      p.layout = next[mode];
    });
    this.#checkTab();
    if (this.selection?.kind === 'path') this.select(null);
  }

  plateExtent(): Vec2 {
    const pl = this.p.plate;
    return pl.outline === 'circle' ? { x: pl.width, y: pl.width } : { x: pl.width, y: pl.height };
  }

  /** Nouvelles dimensions de la plaque : les contrôleurs et les tracés suivent
   *  la plaque ; avec `scalePattern`, tailles, pas, portées et cibles aussi. */
  resizePlate(width: number, height: number) {
    const before = this.plateExtent();
    this.edit((p) => {
      p.plate.width = width;
      p.plate.height = height;
    }, 'plate.size');
    const after = this.plateExtent();
    if (before.x <= 0 || before.y <= 0) return;
    const kx = after.x / before.x;
    const ky = after.y / before.y;
    const k = Math.sqrt(kx * ky);
    const all = this.scalePattern;
    const r = (v: number) => v * k;
    this.edit((p) => {
      for (const c of p.controllers) {
        c.geometry = scaleGeometry(c.geometry, kx, ky, k, all);
        if (all) {
          for (const ch of CHANNELS) {
            const inf = c[ch];
            if (!inf) continue;
            if (c.geometry.type !== 'image') inf.reach = r(inf.reach);
            if (ch !== 'angle') inf.target = r(inf.target);
          }
        }
      }
      if (p.plate.custom) p.plate.custom = p.plate.custom.map((q) => ({ x: q.x * kx, y: q.y * ky }));
      const z = p.plate.zone;
      if (z && z.type !== 'plate') {
        z.center = { x: z.center.x * kx, y: z.center.y * ky };
        if (all && z.type === 'circle') z.diameter = r(z.diameter);
        if (all && z.type === 'rectangle') {
          z.width *= kx;
          z.height *= ky;
          z.corner = r(z.corner);
        }
      }
      for (const f of p.features ?? []) {
        // Les trous de fixation gardent leur diamètre (celui de la vis).
        const s = toFeatureShape(scaleGeometry(featureGeometry(f.shape), kx, ky, k, all && !f.cut));
        if (s) f.shape = s;
      }
      const l = p.layout;
      if (l.mode === 'path') {
        l.paths = l.paths.map((c) => {
          const g = scaleGeometry(curveGeometry(c), kx, ky, k, all) as Extract<CtrlGeometry, { type: 'curve' }>;
          return { points: g.points, closed: g.closed, fillets: g.fillets };
        });
      }
      if (!all) return;
      p.shape.size = r(p.shape.size);
      if (p.shape.kind.type === 'rectangle') p.shape.kind.corner = r(p.shape.kind.corner);
      p.plate.margin = r(p.plate.margin);
      p.plate.corner_radius = r(p.plate.corner_radius);
      p.fabrication.min_size = r(p.fabrication.min_size);
      switch (l.mode) {
        case 'grid':
          l.pitch_x = r(l.pitch_x);
          l.pitch_y = r(l.pitch_y);
          l.offset_x = r(l.offset_x);
          l.offset_y = r(l.offset_y);
          break;
        case 'cross_hatch':
          l.step = r(l.step);
          l.line_spacing = r(l.line_spacing);
          break;
        case 'random':
        case 'fibonacci':
          l.spacing = r(l.spacing);
          break;
        case 'spiral':
        case 'concentric':
          l.spacing = r(l.spacing);
          l.pitch = r(l.pitch);
          l.start = r(l.start);
          break;
        case 'path':
          l.spacing = r(l.spacing);
          l.row_gap = r(l.row_gap);
          break;
      }
    }, 'plate.size');
  }

  // ---- Vue

  /** Cadre la plaque dans le canevas. */
  fit() {
    const { w, h } = this.viewport;
    if (!this.project || w === 0) return;
    const ext = this.plateExtent();
    const pad = 48;
    const s = Math.min(MAX_SCALE, Math.max(MIN_SCALE, Math.min((w - 2 * pad) / Math.max(ext.x, 1), (h - 2 * pad) / Math.max(ext.y, 1))));
    this.view = { scale: s, ox: (w - ext.x * s) / 2, oy: (h + ext.y * s) / 2 };
    this.viewTouched = false;
  }

  /** Déplacement de la vue par l'utilisateur. */
  pan(ox: number, oy: number) {
    this.view = { scale: this.view.scale, ox, oy };
    this.viewTouched = true;
  }

  /** Zoom de facteur `f` autour du point d'écran (cx, cy), le centre par défaut. */
  zoomBy(f: number, cx = this.viewport.w / 2, cy = this.viewport.h / 2) {
    const v = this.view;
    const s = Math.min(MAX_SCALE, Math.max(MIN_SCALE, v.scale * f));
    const k = s / v.scale;
    this.view = { scale: s, ox: cx - (cx - v.ox) * k, oy: cy - (cy - v.oy) * k };
    this.viewTouched = true;
  }

  // ---- Messages

  toast(text: string) {
    const id = ++this.#toastSeq;
    this.toasts = [...this.toasts, { id, text }];
    setTimeout(() => (this.toasts = this.toasts.filter((x) => x.id !== id)), 3200);
  }
}

export const store = new Store();
