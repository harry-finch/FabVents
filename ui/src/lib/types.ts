// Types du projet et du rapport, calqués sur `fabvents-core` (sérialisation
// serde). Millimètres et degrés ; repère de la plaque : origine en bas à
// gauche, Y vers le haut.

export type Vec2 = { x: number; y: number };

export type Outline = 'rectangle' | 'circle' | 'ellipse' | 'custom';

export interface Plate {
  outline: Outline;
  width: number;
  height: number;
  corner_radius: number;
  thickness: number;
  margin: number;
  /** Contour libre (`outline: 'custom'`), dans [0, width] × [0, height]. */
  custom?: Vec2[];
  /** Zone perforée (absente : toute la plaque). */
  zone?: Zone;
}

export type Zone =
  | { type: 'plate' }
  | { type: 'circle'; center: Vec2; diameter: number }
  | { type: 'rectangle'; center: Vec2; width: number; height: number; corner: number };

export type FeatureShape =
  | { type: 'circle'; center: Vec2; diameter: number }
  | { type: 'rectangle'; center: Vec2; width: number; height: number; corner: number }
  | ({ type: 'curve' } & Curve);

/** Trou de fixation (`cut`) ou zone réservée, et sa garde (mm). */
export interface Feature {
  id: number;
  shape: FeatureShape;
  cut: boolean;
  clearance: number;
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

export interface Curve {
  points: Vec2[];
  closed: boolean;
  /** Rayon de congé par nœud (vide : aucun). */
  fillets?: number[];
}

export interface RandomLayout {
  mode: 'random';
  spacing: number;
  seed: number;
}

export interface SpiralLayout {
  mode: 'spiral';
  pitch: number;
  spacing: number;
  start: number;
  clockwise: boolean;
  offset_x: number;
  offset_y: number;
  orient: boolean;
}

export interface ConcentricLayout {
  mode: 'concentric';
  pitch: number;
  spacing: number;
  start: number;
  phase: number;
  offset_x: number;
  offset_y: number;
  orient: boolean;
}

/** Angle d'or (degrés), divergence par défaut de la disposition Fibonacci. */
export const GOLDEN = 137.50776405003785;

export interface FibonacciLayout {
  mode: 'fibonacci';
  spacing: number;
  divergence: number;
  offset_x: number;
  offset_y: number;
  orient: boolean;
}

export interface CrossHatchLayout {
  mode: 'cross_hatch';
  line_spacing: number;
  step: number;
  angle: number;
  cross: number;
  offset_x: number;
  offset_y: number;
}

export interface PathLayout {
  mode: 'path';
  paths: Curve[];
  spacing: number;
  rows: number;
  row_gap: number;
  orient: boolean;
}

export type Layout = GridLayout | RandomLayout | SpiralLayout | ConcentricLayout | FibonacciLayout | CrossHatchLayout | PathLayout;
export type LayoutMode = Layout['mode'];
export const LAYOUT_MODES: LayoutMode[] = ['grid', 'random', 'spiral', 'concentric', 'fibonacci', 'cross_hatch', 'path'];

/** Valeur de base du canal Espacement (null : la disposition ne le suit pas). */
export function baseSpacing(l: Layout): number | null {
  return l.mode === 'grid' || l.mode === 'cross_hatch' ? null : l.spacing;
}

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

export type EdgeMode = 'center' | 'whole' | 'clip';

export interface Fabrication {
  min_size: number;
  min_bridge: number;
}

export type CullMode = 'center' | 'edge' | 'uniform';

export interface Cull {
  amount: number;
  mode: CullMode;
  seed: number;
}

export interface Influence {
  reach: number;
  target: number;
  hard_edge: boolean;
  /** Image : zones claires vers la cible. */
  invert?: boolean;
  /** Image : contraste (1 par défaut). */
  contrast?: number;
  /** Angle : rotation inversée de l'autre côté. */
  symmetric?: boolean;
  /** Angle : pas d'arrondi (degrés). */
  step?: number;
  /** Taille : disparition aléatoire. */
  cull?: Cull;
}

export interface Raster {
  width: number;
  height: number;
  /** Luminance (octets, ligne du haut d'abord) en base64. */
  luma: string;
}

export type CtrlGeometry =
  | { type: 'point'; at: Vec2 }
  | { type: 'line'; a: Vec2; b: Vec2 }
  | ({ type: 'curve' } & Curve)
  | { type: 'circle'; center: Vec2; radius: number }
  | { type: 'ellipse'; center: Vec2; rx: number; ry: number; angle: number }
  | { type: 'polygon'; center: Vec2; radius: number; sides: number; angle: number }
  | { type: 'image'; at: Vec2; width: number; height: number; raster: Raster }
  /** Rectangle (zones réservées seulement). */
  | { type: 'rect'; center: Vec2; width: number; height: number; corner: number };

export type GeometryType = CtrlGeometry['type'];

export type Channel = 'size' | 'spacing' | 'angle';
export const CHANNELS: Channel[] = ['size', 'spacing', 'angle'];

/** Onglet du canevas : un canal, ou les tracés de la disposition Chemin. */
export type Tab = Channel | 'path' | 'plate';

export type Side = 'both' | 'left' | 'right';

export interface Controller {
  id: number;
  geometry: CtrlGeometry;
  side?: Side;
  extend?: boolean;
  fill?: boolean;
  size?: Influence;
  spacing?: Influence;
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
  features?: Feature[];
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
  islands: number;
}

export interface Stats {
  seeds: number;
  removed_edge: number;
  removed_small: number;
  removed_cull: number;
  removed_feature: number;
  merged: number;
}

export interface Report {
  metrics: Metrics;
  stats: Stats;
  issues: Issue[];
  stages: [boolean, boolean, boolean, boolean];
}

/** Géométrie calculée par le moteur, prête à dessiner. */
export interface FrameData {
  circles: Float64Array;
  polyPoints: Float64Array;
  polyOffsets: Uint32Array;
  outline: Float64Array;
  /** Contours de la zone perforable. */
  zonePoints: Float64Array;
  zoneOffsets: Uint32Array;
  report: Report;
  ms: number;
}

export type Tool = 'select' | 'hand' | 'point' | 'line' | 'curve' | 'ruler';

/** Tailles des gabarits de grille de ventilateur (mm). */
export const FAN_SIZES = [40, 60, 80, 92, 120, 140];

/** Contour importé (SVG, DXF). */
export interface ImportedOutline {
  outer: Vec2[];
  inner: Vec2[][];
  width: number;
  height: number;
}
