// Bibliothèque locale de préréglages : motifs enregistrés sur le poste
// (disposition, forme, contrôleurs, fabrication…), appliqués à la plaque
// en cours en suivant ses proportions. Import / export d'un fichier JSON
// `fabvents.presets` pour les partager.

import type { Project } from './types';

const KEY = 'fabvents.presets.v1';
export const PRESETS_FORMAT = 'fabvents.presets';

export interface Preset {
  id: string;
  name: string;
  /** Date d'enregistrement (ISO 8601). */
  saved: string;
  project: Project;
}

function read(): Preset[] {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? '[]');
    return Array.isArray(v) ? v : [];
  } catch {
    return [];
  }
}

class Presets {
  list = $state<Preset[]>(read());

  #save() {
    try {
      localStorage.setItem(KEY, JSON.stringify(this.list));
    } catch {
      /* stockage indisponible : la liste vit pour la session */
    }
  }

  add(name: string, project: Project) {
    const p: Preset = {
      id: `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`,
      name: name.trim() || project.name || 'Préréglage',
      saved: new Date().toISOString(),
      project: JSON.parse(JSON.stringify(project)),
    };
    this.list = [p, ...this.list];
    this.#save();
  }

  remove(id: string) {
    this.list = this.list.filter((p) => p.id !== id);
    this.#save();
  }

  rename(id: string, name: string) {
    this.list = this.list.map((p) => (p.id === id ? { ...p, name } : p));
    this.#save();
  }

  /** Fichier de la bibliothèque (tous les préréglages). */
  exportJson(): string {
    return JSON.stringify({ format: PRESETS_FORMAT, version: 1, presets: this.list }, null, 2);
  }

  /** Ajoute les préréglages d'un fichier ; un nom déjà présent est remplacé.
   *  Renvoie le nombre de préréglages lus, ou `null` si le fichier n'en est pas un. */
  importJson(text: string): number | null {
    let data: unknown;
    try {
      data = JSON.parse(text);
    } catch {
      return null;
    }
    const d = data as { format?: string; presets?: Preset[] };
    if (d?.format !== PRESETS_FORMAT || !Array.isArray(d.presets)) return null;
    const incoming = d.presets.filter((p) => p && typeof p.name === 'string' && p.project?.format === 'fabvents.project');
    const names = new Set(incoming.map((p) => p.name));
    this.list = [...incoming, ...this.list.filter((p) => !names.has(p.name))];
    this.#save();
    return incoming.length;
  }
}

export const presets = new Presets();
