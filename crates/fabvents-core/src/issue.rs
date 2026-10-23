//! Messages du moteur : un code stable (traduit par l'interface), une
//! gravité, le champ du projet concerné et une valeur éventuelle.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Le motif ne peut pas être calculé tant que la valeur n'est pas corrigée.
    Error,
    /// Le motif est calculé mais un point de fabrication demande attention.
    Warning,
    /// Information utile (trous retirés, aucune perforation…).
    Info,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Issue {
    pub code: &'static str,
    pub severity: Severity,
    /// Chemin du champ concerné (`layout.pitch_x`, `controllers.3.size.reach`…).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
}

impl Issue {
    pub fn new(code: &'static str, severity: Severity) -> Self {
        Self {
            code,
            severity,
            field: None,
            value: None,
            count: None,
        }
    }

    pub fn error(code: &'static str, field: impl Into<String>) -> Self {
        Self {
            field: Some(field.into()),
            ..Self::new(code, Severity::Error)
        }
    }

    pub fn warning(code: &'static str) -> Self {
        Self::new(code, Severity::Warning)
    }

    pub fn info(code: &'static str) -> Self {
        Self::new(code, Severity::Info)
    }

    pub fn field(mut self, f: impl Into<String>) -> Self {
        self.field = Some(f.into());
        self
    }

    pub fn value(mut self, v: f64) -> Self {
        self.value = Some(v);
        self
    }

    pub fn count(mut self, n: usize) -> Self {
        self.count = Some(n);
        self
    }
}
