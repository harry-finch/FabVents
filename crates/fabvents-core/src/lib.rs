//! Moteur FabVents : motifs de perforation paramétriques.
//!
//! Bibliothèque pure et déterministe (aucune IO, aucun réseau, aucune
//! interface) : un [`Project`] entre, une géométrie, des mesures et des
//! fichiers d'export sortent. Millimètres et degrés ; Y vers le haut.
//!
//! ```
//! use fabvents_core::{export, Pipeline, Project};
//!
//! let project = Project::example();
//! let mut pipeline = Pipeline::new();
//! let frame = pipeline.compute(&project);
//! assert!(frame.metrics.open_ratio > 0.0);
//! let dxf = export::dxf::dxf(&export::Drawing { project: &project, holes: frame.holes });
//! assert!(dxf.contains("CIRCLE"));
//! ```

pub mod contour;
pub mod export;
pub mod field;
pub mod geom;
pub mod issue;
pub mod layout;
pub mod metrics;
pub mod pipeline;
pub mod plate;
pub mod project;
pub mod shape;
pub mod validate;

pub use contour::{Contour, Vertex};
pub use geom::Vec2;
pub use issue::{Issue, Severity};
pub use metrics::Metrics;
pub use pipeline::{Frame, Hole, Pipeline, Stats};
pub use project::{Project, ProjectError};

/// Version du moteur (celle du workspace).
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
