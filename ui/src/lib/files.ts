// Fichiers : enregistrement (téléchargement du navigateur ou de la vue web)
// et ouverture (sélecteur de fichier, glisser-déposer).

import { store } from './store.svelte';
import { t } from './i18n.svelte';
import { isDesktop, openFile as openNative, openPath, saveFile, writeFile, type Opened } from './desktop';

export const PROJECT_EXT = '.fabvents.json';

/** Nom de fichier tiré du nom du projet. */
export function baseName(name: string): string {
  const s = name
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/[^\w-]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .toLowerCase();
  return s || 'fabvents';
}

/** Filtres des fenêtres d'enregistrement de l'application de bureau. */
const FILTERS: Record<string, string> = {
  json: 'FabVents',
  svg: 'SVG',
  dxf: 'DXF',
  step: 'STEP',
  zip: 'ZIP',
  png: 'PNG',
};

/** Remet un fichier à l'utilisateur : fenêtre d'enregistrement native dans
 *  l'application de bureau, téléchargement dans le navigateur. Renvoie le
 *  chemin choisi (bureau), `''` (navigateur) ou `null` si l'utilisateur annule. */
export async function deliver(data: string | Uint8Array<ArrayBuffer>, filename: string, mime: string): Promise<string | null> {
  if (isDesktop) {
    const ext = filename.split('.').pop() ?? '';
    return saveFile(filename, data, FILTERS[ext] ?? ext.toUpperCase(), [ext]);
  }
  download(data, filename, mime);
  return '';
}

export function download(data: string | Uint8Array<ArrayBuffer>, filename: string, mime: string) {
  const url = URL.createObjectURL(new Blob([data], { type: mime }));
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 2000);
}

const MIME = { svg: 'image/svg+xml', dxf: 'application/dxf', step: 'model/step' } as const;

export async function exportAs(format: 'svg' | 'dxf' | 'step' | 'step-zip' | 'project') {
  const p = store.project;
  if (!p) return;
  const name = baseName(p.name);
  const json = JSON.stringify(p);
  if (format === 'project') {
    const text = JSON.stringify(p, null, 2);
    // Bureau : un projet déjà ouvert ou enregistré l'est sur place.
    if (isDesktop && store.filePath) {
      await writeFile(store.filePath, text);
      store.toast(t('file.saved', { name: store.filePath.split(/[\\/]/).pop() ?? '' }));
      return;
    }
    const path = await deliver(text, name + PROJECT_EXT, 'application/json');
    if (path == null) return;
    if (path) store.filePath = path;
    store.toast(t('file.saved', { name: path ? (path.split(/[\\/]/).pop() ?? '') : name + PROJECT_EXT }));
    return;
  }
  try {
    if (format === 'step-zip') {
      const bytes = await store.engine.exportStepZip(json);
      const file = `${name}-dalles.zip`;
      if ((await deliver(bytes as Uint8Array<ArrayBuffer>, file, 'application/zip')) == null) return;
      store.toast(t('file.exported', { name: file }));
      return;
    }
    const text = await store.engine.export(format, json);
    const file = `${name}.${format}`;
    if ((await deliver(text, file, MIME[format])) == null) return;
    store.toast(t('file.exported', { name: file }));
  } catch (e) {
    const code = e instanceof Error ? e.message : '';
    store.toast(t(`errors.${code}` as never));
  }
}

export async function openFile(file: File) {
  const text = await file.text();
  if (/\.dxf$/i.test(file.name)) await store.importOutlineText(text, file.name);
  else await store.openText(text, file.name);
}

/** Choisit un SVG ou un DXF dont le contour devient celui de la plaque. */
export function pickOutline() {
  const input = document.createElement('input');
  input.type = 'file';
  input.accept = '.svg,.dxf,image/svg+xml';
  input.onchange = () => {
    const f = input.files?.[0];
    if (f) void store.importOutline(f);
  };
  input.click();
}

/** PNG haute résolution, fond transparent : la matière en gris, les
 *  perforations vides. 300 ppp, plafonné à 8 192 px de côté. */
export function exportPng() {
  const f = store.frame;
  const p = store.project;
  if (!f || !p) return;
  const ext = store.plateExtent();
  let dpi = 300;
  const maxSide = 8192;
  const side = (Math.max(ext.x, ext.y) / 25.4) * dpi;
  if (side > maxSide) dpi = Math.floor((dpi * maxSide) / side);
  const k = dpi / 25.4;
  const c = document.createElement('canvas');
  c.width = Math.max(1, Math.ceil(ext.x * k));
  c.height = Math.max(1, Math.ceil(ext.y * k));
  const ctx = c.getContext('2d')!;
  const X = (x: number) => x * k;
  const Y = (y: number) => c.height - y * k;
  ctx.beginPath();
  ctx.moveTo(X(f.outline[0]), Y(f.outline[1]));
  for (let i = 2; i < f.outline.length; i += 2) ctx.lineTo(X(f.outline[i]), Y(f.outline[i + 1]));
  ctx.closePath();
  for (let i = 0; i < f.circles.length; i += 3) {
    const x = X(f.circles[i]);
    const y = Y(f.circles[i + 1]);
    const r = f.circles[i + 2] * k;
    ctx.moveTo(x + r, y);
    ctx.arc(x, y, r, 0, Math.PI * 2);
  }
  const pts = f.polyPoints;
  const off = f.polyOffsets;
  for (let j = 0; j + 1 < off.length; j++) {
    if (off[j + 1] - off[j] < 2) continue;
    ctx.moveTo(X(pts[2 * off[j]]), Y(pts[2 * off[j] + 1]));
    for (let i = off[j] + 1; i < off[j + 1]; i++) ctx.lineTo(X(pts[2 * i]), Y(pts[2 * i + 1]));
    ctx.closePath();
  }
  ctx.fillStyle = '#3a3d42';
  ctx.fill('evenodd');
  const name = `${baseName(p.name)}.png`;
  c.toBlob(async (blob) => {
    if (!blob) return;
    const data = new Uint8Array(await blob.arrayBuffer());
    if ((await deliver(data, name, 'image/png')) == null) return;
    store.toast(t('file.exportedPng', { name, dpi, w: c.width, h: c.height }));
  }, 'image/png');
}

/** Ouvre un fichier choisi dans l'application de bureau (projet, SVG, DXF). */
export async function openNativeFile(opened: Opened | null) {
  if (!opened) return;
  if (/\.dxf$/i.test(opened.name)) await store.importOutlineText(opened.text, opened.name);
  else {
    await store.openText(opened.text, opened.name);
    store.filePath = /\.json$/i.test(opened.name) ? opened.path : null;
  }
}

/** Fichier récent (application de bureau). */
export async function openRecent(path: string) {
  try {
    await openNativeFile(await openPath(path));
  } catch (e) {
    store.toast(String(e));
  }
}

export function pickFile() {
  if (isDesktop) {
    void openNative().then(openNativeFile, (e) => store.toast(String(e)));
    return;
  }
  const input = document.createElement('input');
  input.type = 'file';
  input.accept = '.json,.svg,.dxf,application/json,image/svg+xml';
  input.onchange = () => {
    const f = input.files?.[0];
    if (f) void openFile(f);
  };
  input.click();
}

/** Choisit une image et l'ajoute comme contrôleur Image du canal affiché. */
export function pickImage() {
  const input = document.createElement('input');
  input.type = 'file';
  input.accept = 'image/png,image/jpeg,image/webp,image/gif,image/bmp';
  input.onchange = () => {
    const f = input.files?.[0];
    if (f) void store.addImage(f);
  };
  input.click();
}

/** Fichier d'image (contrôleur) plutôt que projet ? Un SVG est toujours un projet. */
export const isImageFile = (f: File) => f.type.startsWith('image/') && f.type !== 'image/svg+xml';
