// Types du projet et du rapport, calqués sur `fabvents-core` (sérialisation
// serde). Millimètres et degrés ; repère de la plaque : origine en bas à
// gauche, Y vers le haut.

export type Vec2 = { x: number; y: number };

export type Outline = 'rectangle' | 'circle';

export interface Plate {
  outline: Outline;
  width: number;
  height: number;
  corner_radius: number;
  thickness: number;
  margin: number;
}

export type GridPattern = 'square' | 'hexagonal' | 'staggered';

export interface GridLayout {
  mode: 'grid';
  pattern: GridPattern;
  pitch_x: number;
  pitch_y: number;
  angle: number;
  offset_x: number;
  offset_y: number;
  row_twist: number;
  col_twist: number;
}

export type Layout = GridLayout;

export type ShapeKind =
  | { type: 'circle' }
  | { type: 'rectangle'; aspect: number; corner: number }
  | { type: 'polygon'; sides: number }
  | { type: 'star'; points: number; inner: number }
  | { type: 'slot'; aspect: number };

export type ShapeType = ShapeKind['type'];

export interface Shape {
  kind: ShapeKind;
  size: number;
  rotation: number;
}

export type EdgeMode = 'center' | 'whole';

export interface Fabrication {
  min_size: number;
  min_bridge: number;
}

export interface Influence {
  reach: number;
  target: number;
  hard_edge: boolean;
}

export type CtrlGeometry = { type: 'point'; at: Vec2 } | { type: 'line'; a: Vec2; b: Vec2 };

export type Channel = 'size' | 'angle';
export const CHANNELS: Channel[] = ['size', 'angle'];

export interface Controller {
  id: number;
  geometry: CtrlGeometry;
  size?: Influence;
  angle?: Influence;
}

export interface Project {
  format: 'fabvents.project';
  version: number;
  name: string;
  plate: Plate;
  layout: Layout;
  shape: Shape;
  edge: EdgeMode;
  fabrication: Fabrication;
  controllers: Controller[];
}

export type Severity = 'error' | 'warning' | 'info';

export interface Issue {
  code: string;
  severity: Severity;
  field?: string;
  value?: number;
  count?: number;
}

export interface Metrics {
  holes: number;
  plate_area: number;
  open_area: number;
  open_ratio: number;
  size_min: number | null;
  size_max: number | null;
  bridge_min: number | null;
  bridge_search: number;
  overlaps: number;
  thin_bridges: number;
}

export interface Stats {
  seeds: number;
  removed_edge: number;
  removed_small: number;
}

export interface Report {
  metrics: Metrics;
  stats: Stats;
  issues: Issue[];
  stages: [boolean, boolean, boolean];
}

/** Géométrie calculée par le moteur, prête à dessiner. */
export interface FrameData {
  circles: Float64Array;
  polyPoints: Float64Array;
  polyOffsets: Uint32Array;
  outline: Float64Array;
  report: Report;
  ms: number;
}

export type Tool = 'select' | 'hand' | 'point' | 'line';
