// Fichiers : enregistrement (téléchargement du navigateur ou de la vue web)
// et ouverture (sélecteur de fichier, glisser-déposer).

import { store } from './store.svelte';
import { t } from './i18n.svelte';

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

export function download(text: string, filename: string, mime: string) {
  const url = URL.createObjectURL(new Blob([text], { type: mime }));
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 2000);
}

export async function exportAs(format: 'svg' | 'dxf' | 'project') {
  const p = store.project;
  if (!p) return;
  const name = baseName(p.name);
  const json = JSON.stringify(p);
  if (format === 'project') {
    download(JSON.stringify(p, null, 2), name + PROJECT_EXT, 'application/json');
    store.toast(t('file.saved', { name: name + PROJECT_EXT }));
    return;
  }
  try {
    const text = await store.engine.export(format, json);
    const file = `${name}.${format}`;
    download(text, file, format === 'svg' ? 'image/svg+xml' : 'application/dxf');
    store.toast(t('file.exported', { name: file }));
  } catch (e) {
    const code = e instanceof Error ? e.message : '';
    store.toast(t(`errors.${code}` as never));
  }
}

export async function openFile(file: File) {
  store.openText(await file.text(), file.name);
}

export function pickFile() {
  const input = document.createElement('input');
  input.type = 'file';
  input.accept = '.json,.svg,application/json,image/svg+xml';
  input.onchange = () => {
    const f = input.files?.[0];
    if (f) void openFile(f);
  };
  input.click();
}
