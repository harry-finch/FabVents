// Client du moteur (Web Worker). Calcul « le dernier gagne » : un seul
// calcul en cours ; si le projet change pendant ce temps, seul le plus
// récent est envoyé ensuite. Les exports et lectures passent par des promesses.

import type { FrameData } from './types';

type Pending = { resolve: (v: string) => void; reject: (e: Error) => void };

export class EngineClient {
  #worker: Worker;
  #busy = false;
  #queued: string | null = null;
  #seq = 0;
  #pending = new Map<number, Pending>();

  onReady: (version: string, example: string, blank: string) => void = () => {};
  onFailed: (error: string) => void = () => {};
  onFrame: (frame: FrameData) => void = () => {};
  onFrameError: (code: string) => void = () => {};

  constructor() {
    this.#worker = new Worker(new URL('./engine.worker.ts', import.meta.url), { type: 'module' });
    this.#worker.onmessage = (ev) => this.#receive(ev.data);
    this.#worker.onerror = (ev) => this.onFailed(ev.message);
  }

  #receive(msg: any) {
    switch (msg.kind) {
      case 'ready':
        this.onReady(msg.version, msg.example, msg.blank);
        break;
      case 'failed':
        this.onFailed(msg.error);
        break;
      case 'frame':
      case 'frame-error':
        if (msg.kind === 'frame') this.onFrame(msg.frame as FrameData);
        else this.onFrameError(msg.error);
        this.#busy = false;
        if (this.#queued) {
          const next = this.#queued;
          this.#queued = null;
          this.compute(next);
        }
        break;
      case 'result': {
        const p = this.#pending.get(msg.id);
        this.#pending.delete(msg.id);
        if (msg.ok) p?.resolve(msg.value);
        else p?.reject(new Error(msg.error));
        break;
      }
    }
  }

  compute(project: string) {
    if (this.#busy) {
      this.#queued = project;
      return;
    }
    this.#busy = true;
    this.#worker.postMessage({ kind: 'compute', id: ++this.#seq, project });
  }

  #call(msg: Record<string, unknown>): Promise<string> {
    const id = ++this.#seq;
    return new Promise((resolve, reject) => {
      this.#pending.set(id, { resolve, reject });
      this.#worker.postMessage({ ...msg, id });
    });
  }

  export(format: 'svg' | 'dxf', project: string): Promise<string> {
    return this.#call({ kind: 'export', format, project });
  }

  /** Contour de plaque depuis un SVG ou un DXF (JSON `ImportedOutline`). */
  importOutline(text: string): Promise<string> {
    return this.#call({ kind: 'import', text });
  }

  /** Projet d'un gabarit de grille de ventilateur. */
  fanTemplate(size: number): Promise<string> {
    return this.#call({ kind: 'template', size });
  }

  /** Projet normalisé depuis un fichier `.fabvents.json` ou un SVG FabVents. */
  read(text: string): Promise<string> {
    return this.#call({ kind: 'read', text });
  }
}
