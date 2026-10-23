// Web Worker du moteur : charge le WebAssembly de `fabvents-wasm` et répond
// aux demandes du fil principal. Le calcul ne bloque jamais l'interface.

import init, { Engine, read_project, example_project, default_project, engine_version } from '../wasm/fabvents.js';

type Request =
  | { kind: 'compute'; id: number; project: string }
  | { kind: 'export'; id: number; format: 'svg' | 'dxf'; project: string }
  | { kind: 'read'; id: number; text: string };

let engine: Engine | null = null;

/** `postMessage` du worker (les types DOM ne connaissent que celui de `window`). */
const post = (msg: unknown, transfer: Transferable[] = []) => (self as unknown as Worker).postMessage(msg, transfer);

const ready = init().then(() => {
  engine = new Engine();
  post({ kind: 'ready', version: engine_version(), example: example_project(), blank: default_project() });
});
ready.catch((e) => post({ kind: 'failed', error: String(e) }));

/** Code d'erreur stable du moteur (message d'un `JsError`). */
const code = (e: unknown) => (e instanceof Error ? e.message : String(e));

onmessage = async (ev: MessageEvent<Request>) => {
  await ready;
  const req = ev.data;
  const e = engine!;
  try {
    if (req.kind === 'compute') {
      const t0 = performance.now();
      const d = e.compute(req.project);
      const circles = d.take_circles();
      const polyPoints = d.take_poly_points();
      const polyOffsets = d.take_poly_offsets();
      const outline = d.take_outline();
      const report = JSON.parse(d.report());
      d.free();
      const ms = performance.now() - t0;
      post({ kind: 'frame', id: req.id, frame: { circles, polyPoints, polyOffsets, outline, report, ms } }, [
        circles.buffer as ArrayBuffer,
        polyPoints.buffer as ArrayBuffer,
        polyOffsets.buffer as ArrayBuffer,
        outline.buffer as ArrayBuffer,
      ]);
    } else if (req.kind === 'export') {
      const text = req.format === 'svg' ? e.export_svg(req.project) : e.export_dxf(req.project);
      post({ kind: 'result', id: req.id, ok: true, value: text });
    } else if (req.kind === 'read') {
      post({ kind: 'result', id: req.id, ok: true, value: read_project(req.text) });
    }
  } catch (err) {
    post({ kind: req.kind === 'compute' ? 'frame-error' : 'result', id: req.id, ok: false, error: code(err) });
  }
};
