//! Projet FabVents : tout ce qui décrit un motif, sérialisé en JSON
//! (fichier `.fabvents.json`, et métadonnées des SVG exportés).
//!
//! Le format est versionné (`format`, `version`). Un champ absent prend sa
//! valeur par défaut ; un projet d'une version plus récente est refusé plutôt
//! que lu à moitié.

use crate::geom::Vec2;
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "fabvents.project";
pub const VERSION: u32 = 1;

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
                size: Some(Influence {
                    reach: 70.0,
                    target: 7.0,
                    hard_edge: false,
                }),
                angle: None,
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
        }
    }
}

impl Plate {
    /// Dimensions de la boîte englobante (origine en bas à gauche).
    pub fn extent(&self) -> Vec2 {
        match self.outline {
            Outline::Rectangle => Vec2::new(self.width, self.height),
            Outline::Circle => Vec2::new(self.width, self.width),
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
/// canaux (« synchronisation »).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Controller {
    pub id: u32,
    pub geometry: Geometry,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<Influence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle: Option<Influence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Geometry {
    Point { at: Vec2 },
    Line { a: Vec2, b: Vec2 },
}

impl Geometry {
    pub fn distance(&self, p: Vec2) -> f64 {
        match *self {
            Geometry::Point { at } => p.distance(at),
            Geometry::Line { a, b } => crate::geom::dist_point_segment(p, a, b),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Influence {
    /// Distance (mm) à laquelle l'influence s'annule.
    pub reach: f64,
    /// Valeur atteinte sur le contrôleur (mm pour la taille, degrés pour l'angle).
    pub target: f64,
    /// Seuil franc : valeur cible pleine dans la portée, aucune au-delà.
    #[serde(default)]
    pub hard_edge: bool,
}

/// Canaux pilotés par les contrôleurs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Size,
    Angle,
}

impl Controller {
    pub fn influence(&self, ch: Channel) -> Option<&Influence> {
        match ch {
            Channel::Size => self.size.as_ref(),
            Channel::Angle => self.angle.as_ref(),
        }
    }
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
    fn tagged_enums_are_readable() {
        let json = Project::example().to_json();
        assert!(json.contains(r#""mode":"grid""#));
        assert!(json.contains(r#""type":"point""#));
        assert!(json.contains(r#""type":"circle""#));
    }
}
