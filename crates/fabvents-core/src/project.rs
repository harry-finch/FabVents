//! Projet FabVents : tout ce qui décrit un motif, sérialisé en JSON
//! (fichier `.fabvents.json`, et métadonnées des SVG exportés).
//!
//! Le format est versionné (`format`, `version`). Un champ absent prend sa
//! valeur par défaut ; un projet d'une version plus récente est refusé plutôt
//! que lu à moitié.
//!
//! Versions : 1 (v0.1, grille, point et ligne, canaux Taille et Angle) ;
//! 2 (v0.2, six dispositions de plus, courbes, formes, image, options des
//! contrôleurs, canal Espacement) ; 3 (v0.3, contours ellipse et
//! personnalisé, zone perforée, trous de fixation et zones réservées,
//! découpe au contour). Les versions précédentes se lisent telles quelles.

use crate::geom::Vec2;
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "fabvents.project";
pub const VERSION: u32 = 3;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    pub format: String,
    pub version: u32,
    pub name: String,
    pub plate: Plate,
    pub layout: Layout,
    pub shape: Shape,
    pub edge: EdgeMode,
    pub fabrication: Fabrication,
    pub controllers: Vec<Controller>,
    /// Trous de fixation et zones réservées.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<Feature>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            format: FORMAT.into(),
            version: VERSION,
            name: String::new(),
            plate: Plate::default(),
            layout: Layout::default(),
            shape: Shape::default(),
            edge: EdgeMode::Whole,
            fabrication: Fabrication::default(),
            controllers: Vec::new(),
            features: Vec::new(),
        }
    }
}

impl Project {
    /// Projet de démonstration : grille hexagonale dont les trous grandissent
    /// vers le centre de la plaque.
    pub fn example() -> Self {
        let plate = Plate::default();
        let c = plate.center();
        Self {
            controllers: vec![Controller {
                id: 1,
                geometry: Geometry::Point { at: c },
                size: Some(Influence::new(70.0, 7.0)),
                ..Controller::new(1, Geometry::Point { at: c })
            }],
            plate,
            ..Self::default()
        }
    }

    /// Lecture d'un projet JSON, avec contrôle du format et de la version.
    pub fn from_json(text: &str) -> Result<Project, ProjectError> {
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|e| ProjectError::Syntax(e.to_string()))?;
        match value.get("format").and_then(|f| f.as_str()) {
            Some(FORMAT) => {}
            _ => return Err(ProjectError::NotAProject),
        }
        let version = value.get("version").and_then(|v| v.as_u64()).unwrap_or(0);
        if version > VERSION as u64 {
            return Err(ProjectError::TooNew(version as u32));
        }
        let mut project: Project =
            serde_json::from_value(value).map_err(|e| ProjectError::Invalid(e.to_string()))?;
        project.version = VERSION;
        Ok(project)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("projet sérialisable")
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("projet sérialisable")
    }

    pub fn next_controller_id(&self) -> u32 {
        self.controllers.iter().map(|c| c.id).max().unwrap_or(0) + 1
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectError {
    Syntax(String),
    NotAProject,
    TooNew(u32),
    Invalid(String),
}

impl ProjectError {
    /// Code stable, traduit par l'interface.
    pub fn code(&self) -> &'static str {
        match self {
            ProjectError::Syntax(_) => "project.syntax",
            ProjectError::NotAProject => "project.not_fabvents",
            ProjectError::TooNew(_) => "project.too_new",
            ProjectError::Invalid(_) => "project.invalid",
        }
    }
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectError::Syntax(e) => write!(f, "JSON illisible : {e}"),
            ProjectError::NotAProject => write!(f, "ce fichier n'est pas un projet FabVents"),
            ProjectError::TooNew(v) => write!(
                f,
                "projet de format {v}, plus récent que ce FabVents ({VERSION})"
            ),
            ProjectError::Invalid(e) => write!(f, "projet invalide : {e}"),
        }
    }
}

impl std::error::Error for ProjectError {}

// ---------------------------------------------------------------------------
// Plaque

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outline {
    Rectangle,
    /// Disque de diamètre `width`.
    Circle,
    /// Ellipse de `width` × `height`.
    Ellipse,
    /// Contour libre (`Plate::custom`), importé ou dessiné.
    Custom,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Plate {
    pub outline: Outline,
    pub width: f64,
    pub height: f64,
    /// Rayon des coins (rectangle).
    pub corner_radius: f64,
    /// Épaisseur de la plaque (export STEP, contrôles de fabrication).
    pub thickness: f64,
    /// Zone sans perforation le long du bord.
    pub margin: f64,
    /// Contour libre (polygone dans `[0, width] × [0, height]`), pour `Outline::Custom`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub custom: Vec<Vec2>,
    /// Zone perforée, en plus de la marge (toute la plaque par défaut).
    #[serde(skip_serializing_if = "is_default")]
    pub zone: Zone,
}

/// Zone où les perforations sont permises, dans la plaque.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Zone {
    /// Toute la plaque, moins la marge.
    #[default]
    Plate,
    Circle {
        center: Vec2,
        diameter: f64,
    },
    Rectangle {
        center: Vec2,
        width: f64,
        height: f64,
        #[serde(default)]
        corner: f64,
    },
}

/// Trou de fixation (découpé) ou zone réservée (pleine), et sa garde :
/// aucune perforation à moins de `clearance` mm.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Feature {
    pub id: u32,
    pub shape: FeatureShape,
    /// Découpé (trou de fixation, passage) ou laissé plein (zone réservée).
    #[serde(default)]
    pub cut: bool,
    #[serde(default)]
    pub clearance: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FeatureShape {
    Circle {
        center: Vec2,
        diameter: f64,
    },
    Rectangle {
        center: Vec2,
        width: f64,
        height: f64,
        #[serde(default)]
        corner: f64,
    },
    Curve(Curve),
}

impl Default for Plate {
    fn default() -> Self {
        Self {
            outline: Outline::Rectangle,
            width: 200.0,
            height: 150.0,
            corner_radius: 6.0,
            thickness: 3.0,
            margin: 6.0,
            custom: Vec::new(),
            zone: Zone::Plate,
        }
    }
}

impl Plate {
    /// Dimensions de la boîte englobante (origine en bas à gauche).
    pub fn extent(&self) -> Vec2 {
        match self.outline {
            Outline::Circle => Vec2::new(self.width, self.width),
            _ => Vec2::new(self.width, self.height),
        }
    }

    pub fn center(&self) -> Vec2 {
        self.extent() * 0.5
    }
}

// ---------------------------------------------------------------------------
// Disposition

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Layout {
    Grid(Grid),
    Random(Random),
    Spiral(Spiral),
    Concentric(Concentric),
    Fibonacci(Fibonacci),
    CrossHatch(CrossHatch),
    Path(PathLayout),
}

impl Layout {
    /// Valeur de base du canal Espacement, pour les dispositions qui le
    /// suivent (aléatoire, spirale, concentrique, Fibonacci, chemin).
    pub fn base_spacing(&self) -> Option<f64> {
        match self {
            Layout::Grid(_) | Layout::CrossHatch(_) => None,
            Layout::Random(l) => Some(l.spacing),
            Layout::Spiral(l) => Some(l.spacing),
            Layout::Concentric(l) => Some(l.spacing),
            Layout::Fibonacci(l) => Some(l.spacing),
            Layout::Path(l) => Some(l.spacing),
        }
    }
}

impl Default for Layout {
    fn default() -> Self {
        Layout::Grid(Grid::default())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridPattern {
    /// Rangées et colonnes alignées.
    Square,
    /// Rangées décalées d'un demi-pas, pas vertical = pas × √3 / 2 (60°).
    Hexagonal,
    /// Rangées décalées d'un demi-pas, pas vertical libre.
    Staggered,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Grid {
    pub pattern: GridPattern,
    /// Entraxe horizontal (mm).
    pub pitch_x: f64,
    /// Entraxe vertical (mm), ignoré en hexagonal.
    pub pitch_y: f64,
    /// Rotation de la grille (degrés).
    pub angle: f64,
    /// Décalage de la grille par rapport au centre de la plaque (mm).
    pub offset_x: f64,
    pub offset_y: f64,
    /// Rotation ajoutée aux formes à chaque rangée / colonne (degrés).
    pub row_twist: f64,
    pub col_twist: f64,
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            pattern: GridPattern::Hexagonal,
            pitch_x: 8.0,
            pitch_y: 8.0,
            angle: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
            row_twist: 0.0,
            col_twist: 0.0,
        }
    }
}

/// Distribution aléatoire sans collision (Poisson-disc), densité suivant le
/// canal Espacement, reproductible par sa graine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Random {
    /// Distance minimale entre deux centres (mm), valeur de base du canal Espacement.
    pub spacing: f64,
    pub seed: u64,
}

impl Default for Random {
    fn default() -> Self {
        Self {
            spacing: 8.0,
            seed: 1,
        }
    }
}

/// Spirale d'Archimède continue depuis le centre.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Spiral {
    /// Écart entre deux spires (mm).
    pub pitch: f64,
    /// Espacement le long de la spirale (mm), base du canal Espacement.
    pub spacing: f64,
    /// Rayon de départ (mm).
    pub start: f64,
    pub clockwise: bool,
    pub offset_x: f64,
    pub offset_y: f64,
    /// Formes orientées selon la tangente.
    pub orient: bool,
}

impl Default for Spiral {
    fn default() -> Self {
        Self {
            pitch: 8.0,
            spacing: 8.0,
            start: 0.0,
            clockwise: false,
            offset_x: 0.0,
            offset_y: 0.0,
            orient: true,
        }
    }
}

/// Anneaux concentriques.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Concentric {
    /// Écart entre deux anneaux (mm).
    pub pitch: f64,
    /// Espacement le long d'un anneau (mm), base du canal Espacement.
    pub spacing: f64,
    /// Rayon du premier anneau (mm) ; 0 place une perforation au centre.
    pub start: f64,
    /// Rotation ajoutée à chaque anneau (degrés).
    pub phase: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    /// Formes orientées selon l'anneau.
    pub orient: bool,
}

impl Default for Concentric {
    fn default() -> Self {
        Self {
            pitch: 8.0,
            spacing: 8.0,
            start: 0.0,
            phase: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
            orient: true,
        }
    }
}

/// Phyllotaxie (spirale de Fibonacci) : chaque point tourne de l'angle de
/// divergence, son rayon suit l'aire occupée par les précédents.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fibonacci {
    /// Espacement moyen entre voisins (mm), base du canal Espacement.
    pub spacing: f64,
    /// Angle de divergence (degrés), l'angle d'or par défaut.
    pub divergence: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    /// Formes orientées vers le centre.
    pub orient: bool,
}

pub const GOLDEN_ANGLE_DEG: f64 = 137.507_764_050_037_85;

impl Default for Fibonacci {
    fn default() -> Self {
        Self {
            spacing: 8.0,
            divergence: GOLDEN_ANGLE_DEG,
            offset_x: 0.0,
            offset_y: 0.0,
            orient: false,
        }
    }
}

/// Hachures croisées : treillis de deux familles de rangées qui se croisent
/// à `cross` ; une forme au milieu de chaque arête du treillis, orientée selon
/// sa rangée, la matière restant aux croisements.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CrossHatch {
    /// Pas entre deux rangées de la première famille, mesuré le long de la seconde (mm).
    pub line_spacing: f64,
    /// Pas le long d'une rangée de la première famille (mm).
    pub step: f64,
    /// Direction de la première famille (degrés).
    pub angle: f64,
    /// Angle entre les deux familles (degrés).
    pub cross: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

impl Default for CrossHatch {
    fn default() -> Self {
        Self {
            line_spacing: 10.0,
            step: 10.0,
            angle: 45.0,
            cross: 90.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

/// Répétition le long de tracés dessinés sur la plaque.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PathLayout {
    pub paths: Vec<Curve>,
    /// Espacement le long du tracé (mm), base du canal Espacement.
    pub spacing: f64,
    /// Nombre de rangées parallèles, centrées sur le tracé.
    pub rows: u32,
    /// Écart entre rangées (mm).
    pub row_gap: f64,
    /// Formes orientées selon le tracé.
    pub orient: bool,
}

impl Default for PathLayout {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            spacing: 8.0,
            rows: 1,
            row_gap: 8.0,
            orient: true,
        }
    }
}

/// Tracé libre : polyligne ouverte ou fermée, avec un congé optionnel à
/// chaque nœud (contrôleur Courbe, disposition Chemin).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Curve {
    pub points: Vec<Vec2>,
    #[serde(default)]
    pub closed: bool,
    /// Rayon de congé à chaque nœud (vide : aucun congé).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fillets: Vec<f64>,
}

impl Curve {
    pub fn fillet(&self, i: usize) -> f64 {
        self.fillets.get(i).copied().unwrap_or(0.0)
    }
}

// ---------------------------------------------------------------------------
// Forme des perforations

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Shape {
    pub kind: ShapeKind,
    /// Dimension de référence (mm) : diamètre (cercle, polygone, étoile),
    /// largeur (rectangle), longueur (fente). Valeur de base du canal Taille.
    pub size: f64,
    /// Rotation de la forme (degrés). Valeur de base du canal Angle.
    pub rotation: f64,
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            kind: ShapeKind::Circle,
            size: 5.0,
            rotation: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ShapeKind {
    #[default]
    Circle,
    /// Hauteur = taille × `aspect` ; `corner` : rayon des coins à la taille de base (mm).
    Rectangle { aspect: f64, corner: f64 },
    /// Polygone régulier inscrit dans le cercle de diamètre `size`, base à plat.
    Polygon { sides: u32 },
    /// Étoile : `points` branches, rayon intérieur = `inner` × rayon extérieur.
    Star { points: u32, inner: f64 },
    /// Fente oblongue : largeur = longueur × `aspect` (0 < aspect ≤ 1).
    Slot { aspect: f64 },
}

// ---------------------------------------------------------------------------
// Bords et fabrication

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeMode {
    /// Garde un trou si son centre est dans la zone perforable.
    Center,
    /// Garde un trou seulement s'il est entièrement dans la zone perforable.
    Whole,
    /// Coupe les trous au bord de la zone perforable (et des zones réservées).
    Clip,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fabrication {
    /// Taille de référence sous laquelle un trou est retiré (et compté).
    pub min_size: f64,
    /// Nervure (matière entre deux trous) sous laquelle le moteur avertit.
    pub min_bridge: f64,
}

impl Default for Fabrication {
    fn default() -> Self {
        Self {
            min_size: 0.5,
            min_bridge: 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Contrôleurs

/// Un contrôleur : une géométrie posée sur la plaque et, pour chaque canal
/// où il agit, une influence. Un même contrôleur peut agir sur plusieurs
/// canaux (« synchronisation »), avec une portée et une cible propres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Controller {
    pub id: u32,
    pub geometry: Geometry,
    /// Ligne ou courbe ouverte : n'agir que d'un côté.
    #[serde(default, skip_serializing_if = "is_default")]
    pub side: Side,
    /// Ligne ou courbe ouverte : prolonger les extrémités à l'infini.
    #[serde(default, skip_serializing_if = "is_false")]
    pub extend: bool,
    /// Tracé fermé : valeur cible pleine dans tout l'intérieur.
    #[serde(default, skip_serializing_if = "is_false")]
    pub fill: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<Influence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing: Option<Influence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle: Option<Influence>,
}

impl Controller {
    /// Contrôleur sans influence (à compléter canal par canal).
    pub fn new(id: u32, geometry: Geometry) -> Self {
        Self {
            id,
            geometry,
            side: Side::Both,
            extend: false,
            fill: false,
            size: None,
            spacing: None,
            angle: None,
        }
    }

    pub fn influence(&self, ch: Channel) -> Option<&Influence> {
        match ch {
            Channel::Size => self.size.as_ref(),
            Channel::Spacing => self.spacing.as_ref(),
            Channel::Angle => self.angle.as_ref(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Geometry {
    Point {
        at: Vec2,
    },
    Line {
        a: Vec2,
        b: Vec2,
    },
    Curve(Curve),
    Circle {
        center: Vec2,
        radius: f64,
    },
    /// Ellipse de demi-axes `rx`, `ry`, tournée de `angle` degrés.
    Ellipse {
        center: Vec2,
        rx: f64,
        ry: f64,
        angle: f64,
    },
    /// Polygone régulier inscrit dans le cercle de rayon `radius`, premier
    /// sommet à `angle` degrés.
    Polygon {
        center: Vec2,
        radius: f64,
        sides: u32,
        angle: f64,
    },
    /// Image en niveaux de gris posée sur la plaque (coin bas gauche `at`) :
    /// sa luminance pilote le canal (sombre = valeur cible).
    Image {
        at: Vec2,
        width: f64,
        height: f64,
        raster: Raster,
    },
}

impl Geometry {
    /// Tracé fermé (remplissable).
    pub fn is_closed(&self) -> bool {
        match self {
            Geometry::Curve(c) => c.closed,
            Geometry::Circle { .. } | Geometry::Ellipse { .. } | Geometry::Polygon { .. } => true,
            _ => false,
        }
    }

    /// Ligne ou courbe ouverte (côté unique, extension).
    pub fn is_open_path(&self) -> bool {
        matches!(self, Geometry::Line { .. }) || matches!(self, Geometry::Curve(c) if !c.closed)
    }
}

/// Image en niveaux de gris : `luma` contient `width × height` octets
/// (ligne du haut d'abord), encodés en base64.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub luma: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    #[default]
    Both,
    /// À gauche dans le sens du tracé.
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Influence {
    /// Distance (mm) à laquelle l'influence s'annule (sans objet pour une image).
    pub reach: f64,
    /// Valeur atteinte sur le contrôleur (mm pour la taille et l'espacement,
    /// degrés pour l'angle).
    pub target: f64,
    /// Seuil franc : valeur cible pleine dans la portée, aucune au-delà.
    #[serde(default)]
    pub hard_edge: bool,
    /// Image : les zones claires (au lieu des sombres) atteignent la cible.
    #[serde(default, skip_serializing_if = "is_false")]
    pub invert: bool,
    /// Image : contraste (1 = luminance telle quelle).
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub contrast: f64,
    /// Angle : rotation inversée de l'autre côté du contrôleur.
    #[serde(default, skip_serializing_if = "is_false")]
    pub symmetric: bool,
    /// Angle : rotation arrondie à ce pas (degrés), 0 = continu.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub step: f64,
    /// Taille : disparition aléatoire de perforations dans la portée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cull: Option<Cull>,
}

impl Influence {
    pub fn new(reach: f64, target: f64) -> Self {
        Self {
            reach,
            target,
            hard_edge: false,
            invert: false,
            contrast: 1.0,
            symmetric: false,
            step: 0.0,
            cull: None,
        }
    }
}

/// Disparition aléatoire : probabilité `amount` (0 à 1), répartie selon `mode`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cull {
    pub amount: f64,
    #[serde(default)]
    pub mode: CullMode,
    #[serde(default)]
    pub seed: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CullMode {
    /// Plus probable près du contrôleur.
    #[default]
    Center,
    /// Plus probable vers la limite de la portée.
    Edge,
    /// Même probabilité dans toute la portée.
    Uniform,
}

/// Canaux pilotés par les contrôleurs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Size,
    Spacing,
    Angle,
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

fn is_one(v: &f64) -> bool {
    *v == 1.0
}

fn one() -> f64 {
    1.0
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_json() {
        let p = Project::example();
        let back = Project::from_json(&p.to_json()).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn missing_fields_take_defaults() {
        let p = Project::from_json(
            r#"{"format":"fabvents.project","version":1,"plate":{"width":120}}"#,
        )
        .unwrap();
        assert_eq!(p.plate.width, 120.0);
        assert_eq!(p.plate.height, Plate::default().height);
        assert_eq!(p.layout, Layout::default());
    }

    #[test]
    fn rejects_foreign_or_newer_files() {
        assert_eq!(
            Project::from_json(r#"{"a":1}"#),
            Err(ProjectError::NotAProject)
        );
        assert_eq!(
            Project::from_json(r#"{"format":"fabvents.project","version":99}"#),
            Err(ProjectError::TooNew(99))
        );
        assert!(matches!(
            Project::from_json("{"),
            Err(ProjectError::Syntax(_))
        ));
    }

    #[test]
    fn version_1_files_are_read() {
        let v1 = r#"{"format":"fabvents.project","version":1,"name":"","plate":{"outline":"rectangle","width":200.0,"height":150.0,"corner_radius":6.0,"thickness":3.0,"margin":6.0},"layout":{"mode":"grid","pattern":"hexagonal","pitch_x":8.0,"pitch_y":8.0,"angle":0.0,"offset_x":0.0,"offset_y":0.0,"row_twist":0.0,"col_twist":0.0},"shape":{"kind":{"type":"circle"},"size":5.0,"rotation":0.0},"edge":"whole","fabrication":{"min_size":0.5,"min_bridge":1.0},"controllers":[{"id":1,"geometry":{"type":"point","at":{"x":100.0,"y":75.0}},"size":{"reach":70.0,"target":7.0,"hard_edge":false}}]}"#;
        let p = Project::from_json(v1).unwrap();
        assert_eq!(p, Project::example());
        assert_eq!(p.version, VERSION);
    }

    #[test]
    fn plate_features_roundtrip() {
        let mut p = Project::example();
        p.plate.zone = Zone::Circle {
            center: Vec2::new(100.0, 75.0),
            diameter: 120.0,
        };
        p.features.push(Feature {
            id: 1,
            shape: FeatureShape::Circle {
                center: Vec2::new(10.0, 10.0),
                diameter: 4.3,
            },
            cut: true,
            clearance: 2.0,
        });
        p.edge = EdgeMode::Clip;
        let json = p.to_json();
        assert!(json.contains(r#""edge":"clip""#));
        assert_eq!(Project::from_json(&json).unwrap(), p);
        // Sans zone ni élément, rien n'est écrit.
        let plain = Project::example().to_json();
        assert!(!plain.contains("zone") && !plain.contains("features"));
    }

    #[test]
    fn new_geometries_roundtrip() {
        let mut p = Project::example();
        p.layout = Layout::Path(PathLayout {
            paths: vec![Curve {
                points: vec![Vec2::new(10.0, 10.0), Vec2::new(90.0, 40.0)],
                closed: false,
                fillets: vec![],
            }],
            ..PathLayout::default()
        });
        let mut c = Controller::new(
            2,
            Geometry::Curve(Curve {
                points: vec![Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(10.0, 10.0)],
                closed: true,
                fillets: vec![0.0, 2.0, 0.0],
            }),
        );
        c.fill = true;
        c.angle = Some(Influence {
            symmetric: true,
            step: 15.0,
            ..Influence::new(20.0, 45.0)
        });
        p.controllers.push(c);
        let json = p.to_json();
        assert!(json.contains(r#""mode":"path""#));
        assert!(json.contains(r#""type":"curve""#));
        assert!(!json.contains("contrast"));
        assert_eq!(Project::from_json(&json).unwrap(), p);
    }

    #[test]
    fn tagged_enums_are_readable() {
        let json = Project::example().to_json();
        assert!(json.contains(r#""mode":"grid""#));
        assert!(json.contains(r#""type":"point""#));
        assert!(json.contains(r#""type":"circle""#));
    }
}
