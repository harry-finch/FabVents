// État de l'application : le projet (avec annuler / rétablir), le moteur,
// et l'état de l'interface (outil, canal, sélection, vue, unités).
//
// Toute modification du projet passe par `edit()` (une étape d'historique,
// fusionnée si la même valeur est modifiée de suite) ou par un geste
// `begin()` … `end()` (glisser une poignée, faire défiler un libellé) : une
// seule étape quelle que soit la durée du geste. Le recalcul et la
// sauvegarde automatique suivent l'état (voir App.svelte).

import { EngineClient } from './engine';
import { t } from './i18n.svelte';
import type { Channel, Controller, CtrlGeometry, FrameData, Influence, Project, Tool, Vec2 } from './types';
import type { UnitSystem } from './format';

const PROJECT_KEY = 'fabvents.project.v1';
const PREFS_KEY = 'fabvents.prefs.v1';
const HISTORY_MAX = 200;
const MIN_SCALE = 0.05;
const MAX_SCALE = 400;
const MERGE_MS = 900;

interface Prefs {
  units: UnitSystem;
  channel: Channel;
}

function readPrefs(): Prefs {
  const fallback: Prefs = { units: 'mm', channel: 'size' };
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

class Store {
  // ---- Projet
  project = $state<Project | null>(null);
  #undo: string[] = [];
  #redo: string[] = [];
  canUndo = $state(false);
  canRedo = $state(false);
  #lastEdit = { key: '', at: 0 };
  #gesture: string | null = null;

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
  channel = $state<Channel>(readPrefs().channel);
  units = $state<UnitSystem>(readPrefs().units);
  selected = $state<number | null>(null);
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
    };
    this.engine.onFrameError = (code) => (this.frameError = code);
  }

  async #restore() {
    const saved = savedProject();
    if (saved) {
      try {
        this.project = JSON.parse(await this.engine.read(saved));
        this.fitSeq++;
        return;
      } catch {
        this.toast(t('file.restoreFailed'));
      }
    }
    this.project = JSON.parse(this.#example);
    this.fitSeq++;
  }

  /** Projet courant (l'interface principale n'est montrée qu'une fois chargé). */
  get p(): Project {
    return this.project!;
  }

  get selectedController(): Controller | null {
    return this.project?.controllers.find((c) => c.id === this.selected) ?? null;
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
    if (this.project && this.#gesture == null) this.#gesture = this.#snapshot();
  }

  end() {
    if (this.#gesture == null) return;
    const before = this.#gesture;
    this.#gesture = null;
    if (this.#snapshot() !== before) this.#push(before);
    this.#lastEdit = { key: '', at: 0 };
  }

  get inGesture() {
    return this.#gesture != null;
  }

  #replace(json: string) {
    this.project = JSON.parse(json);
    if (this.selected != null && !this.project!.controllers.some((c) => c.id === this.selected)) this.selected = null;
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
    this.selected = null;
    this.fitSeq++;
  }

  /** Remplace le projet par le contenu d'un fichier (JSON ou SVG FabVents). */
  async openText(text: string, name: string) {
    try {
      const json = await this.engine.read(text);
      if (this.project) this.#push(this.#snapshot());
      this.#replace(json);
      this.selected = null;
      this.fitSeq++;
      this.toast(t('file.opened', { name }));
    } catch (e) {
      const code = e instanceof Error ? e.message : '';
      this.toast(t('file.openFailed', { name, reason: t(`errors.${code}` as never) }));
    }
  }

  persist() {
    if (!this.project) return;
    try {
      localStorage.setItem(PROJECT_KEY, this.#snapshot());
      localStorage.setItem(PREFS_KEY, JSON.stringify({ units: this.units, channel: this.channel }));
    } catch {
      /* stockage indisponible */
    }
  }

  // ---- Contrôleurs

  defaultInfluence(ch: Channel): Influence {
    const p = this.p;
    const ext = p.plate.outline === 'circle' ? [p.plate.width, p.plate.width] : [p.plate.width, p.plate.height];
    const reach = Math.max(5, Math.round(0.3 * Math.min(ext[0], ext[1])));
    if (ch === 'size') return { reach, target: Math.round(p.shape.size * 16) / 10, hard_edge: false };
    return { reach, target: p.shape.rotation + 45, hard_edge: false };
  }

  addController(geometry: CtrlGeometry): number {
    let id = 0;
    this.edit((p) => {
      id = p.controllers.reduce((m, c) => Math.max(m, c.id), 0) + 1;
      const c: Controller = { id, geometry };
      c[this.channel] = this.defaultInfluence(this.channel);
      p.controllers.push(c);
    });
    this.selected = id;
    return id;
  }

  removeController(id: number) {
    this.edit((p) => {
      p.controllers = p.controllers.filter((c) => c.id !== id);
    });
    if (this.selected === id) this.selected = null;
  }

  duplicateController(id: number) {
    const src = this.project?.controllers.find((c) => c.id === id);
    if (!src) return;
    const offset = (v: Vec2) => ({ x: v.x + 10, y: v.y - 10 });
    const copy: Controller = JSON.parse(JSON.stringify(src));
    copy.geometry =
      copy.geometry.type === 'point'
        ? { type: 'point', at: offset(copy.geometry.at) }
        : { type: 'line', a: offset(copy.geometry.a), b: offset(copy.geometry.b) };
    let newId = 0;
    this.edit((p) => {
      newId = p.controllers.reduce((m, c) => Math.max(m, c.id), 0) + 1;
      p.controllers.push({ ...copy, id: newId });
    });
    this.selected = newId;
  }

  /** Contrôleurs visibles sur le canevas : ceux du canal affiché. */
  get channelControllers(): Controller[] {
    return this.project?.controllers.filter((c) => c[this.channel]) ?? [];
  }

  setChannel(ch: Channel) {
    this.channel = ch;
    const sel = this.selectedController;
    if (sel && !sel[ch]) this.selected = null;
  }

  // ---- Vue

  plateExtent(): Vec2 {
    const pl = this.p.plate;
    return pl.outline === 'circle' ? { x: pl.width, y: pl.width } : { x: pl.width, y: pl.height };
  }

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
