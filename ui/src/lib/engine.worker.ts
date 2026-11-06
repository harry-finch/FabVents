// Web Worker du moteur : charge le WebAssembly de `fabvents-wasm` et répond
// aux demandes du fil principal. Le calcul ne bloque jamais l'interface.

import init, { Engine, read_project, example_project, default_project, engine_version, import_outline, fan_template, import_shape, shape_preview } from '../wasm/fabvents.js';

type Request =
  | { kind: 'compute'; id: number; project: string }
  | { kind: 'export'; id: number; format: 'svg' | 'dxf' | 'step'; project: string; timestamp?: string }
  | { kind: 'export-zip'; id: number; project: string; timestamp: string }
  | { kind: 'read'; id: number; text: string }
  | { kind: 'import'; id: number; text: string }
  | { kind: 'template'; id: number; size: number }
  | { kind: 'import-shape'; id: number; text: string }
  | { kind: 'preview'; id: number; shape: string };

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
      const zonePoints = d.take_zone_points();
      const zoneOffsets = d.take_zone_offsets();
      const alertPoints = d.take_alert_points();
      const alertOffsets = d.take_alert_offsets();
      const tileCuts = d.take_tile_cuts();
      const report = JSON.parse(d.report());
      d.free();
      const ms = performance.now() - t0;
      post({ kind: 'frame', id: req.id, frame: { circles, polyPoints, polyOffsets, outline, zonePoints, zoneOffsets, alertPoints, alertOffsets, tileCuts, report, ms } }, [
        circles.buffer as ArrayBuffer,
        polyPoints.buffer as ArrayBuffer,
        polyOffsets.buffer as ArrayBuffer,
        outline.buffer as ArrayBuffer,
        zonePoints.buffer as ArrayBuffer,
        zoneOffsets.buffer as ArrayBuffer,
        alertPoints.buffer as ArrayBuffer,
        alertOffsets.buffer as ArrayBuffer,
        tileCuts.buffer as ArrayBuffer,
      ]);
    } else if (req.kind === 'export') {
      const text =
        req.format === 'svg' ? e.export_svg(req.project) : req.format === 'dxf' ? e.export_dxf(req.project) : e.export_step(req.project, req.timestamp ?? '');
      post({ kind: 'result', id: req.id, ok: true, value: text });
    } else if (req.kind === 'export-zip') {
      const bytes = e.export_step_zip(req.project, req.timestamp);
      post({ kind: 'result', id: req.id, ok: true, value: bytes }, [bytes.buffer as ArrayBuffer]);
    } else if (req.kind === 'read') {
      post({ kind: 'result', id: req.id, ok: true, value: read_project(req.text) });
    } else if (req.kind === 'import') {
      post({ kind: 'result', id: req.id, ok: true, value: import_outline(req.text) });
    } else if (req.kind === 'template') {
      post({ kind: 'result', id: req.id, ok: true, value: fan_template(req.size) });
    } else if (req.kind === 'import-shape') {
      post({ kind: 'result', id: req.id, ok: true, value: import_shape(req.text) });
    } else if (req.kind === 'preview') {
      post({ kind: 'result', id: req.id, ok: true, value: shape_preview(req.shape) });
    }
  } catch (err) {
    post({ kind: req.kind === 'compute' ? 'frame-error' : 'result', id: req.id, ok: false, error: code(err) });
  }
};
