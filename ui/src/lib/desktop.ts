// Application de bureau (Tauri) : fenêtres natives d'ouverture et
// d'enregistrement, enregistrement sur place, fichiers récents. Dans un
// navigateur, `isDesktop` est faux et les fichiers passent par le
// téléchargement et le sélecteur du navigateur.

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

const tauri = (globalThis as { __TAURI__?: { core: { invoke: Invoke } } }).__TAURI__;

export const isDesktop = !!tauri;

export interface Opened {
  path: string;
  name: string;
  text: string;
}

const bytes = (data: string | Uint8Array) => Array.from(typeof data === 'string' ? new TextEncoder().encode(data) : data);

/** Fenêtre d'enregistrement ; renvoie le chemin choisi, ou `null` si annulé. */
export function saveFile(name: string, data: string | Uint8Array, filter: string, extensions: string[]): Promise<string | null> {
  return tauri!.core.invoke<string | null>('save_file', { name, data: bytes(data), filter, extensions });
}

/** Enregistre sur place. */
export function writeFile(path: string, data: string | Uint8Array): Promise<void> {
  return tauri!.core.invoke<void>('write_file', { path, data: bytes(data) });
}

/** Fenêtre d'ouverture ; `null` si annulé. */
export function openFile(): Promise<Opened | null> {
  return tauri!.core.invoke<Opened | null>('open_file');
}

export function openPath(path: string): Promise<Opened> {
  return tauri!.core.invoke<Opened>('open_path', { path });
}

export function recentFiles(): Promise<string[]> {
  return isDesktop ? tauri!.core.invoke<string[]>('recent_files') : Promise.resolve([]);
}

/** Nom affiché d'un chemin (dernier élément). */
export const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;
