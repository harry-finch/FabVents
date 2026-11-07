//! Serveur MCP (Model Context Protocol) sur stdio : JSON-RPC 2.0, un message
//! par ligne. Rien d'autre que les réponses n'est écrit sur la sortie
//! standard (les journaux vont sur la sortie d'erreur).
//!
//! Outils : guide du format, nouveau projet, calcul (mesures et messages),
//! solveur de taux d'ouverture, aperçu PNG, export dans un fichier.

use crate::render::base64;
use crate::tools;
use fabvents_core::{templates, Project};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

const PROTOCOL: &str = "2025-06-18";

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": { "type": "object", "properties": properties, "required": required },
    })
}

fn project_prop() -> Value {
    json!({ "type": "object", "description": "Projet FabVents (format JSON `fabvents.project`, voir fabvents_guide)." })
}

pub fn tools_list() -> Value {
    json!([
        tool("fabvents_guide", "Guide du format de projet FabVents (dispositions, formes, contrôleurs, unités). À lire avant de modifier un projet.", json!({}), &[]),
        tool(
            "fabvents_new_project",
            "Nouveau projet de départ : `example` (plaque 200 × 150 avec contrôleurs), `default` (grille simple) ou `fan` (grille de ventilateur, avec `fan_size` 40, 60, 80, 92, 120 ou 140).",
            json!({ "kind": { "type": "string", "enum": ["example", "default", "fan"] }, "fan_size": { "type": "integer" } }),
            &["kind"],
        ),
        tool(
            "fabvents_compute",
            "Calcule un projet : nombre de perforations, taux d'ouverture (%), nervure minimale (mm), statistiques et messages du moteur (codes stables).",
            json!({ "project": project_prop() }),
            &["project"],
        ),
        tool(
            "fabvents_solve_open_ratio",
            "Règle la taille de base (`size`) ou l'espacement (`spacing`) pour atteindre un taux d'ouverture visé (%), sans nervure plus fine que `fabrication.min_bridge`. Renvoie le projet réglé et le bilan (`reached` faux si le taux est hors d'atteinte : meilleur réglage possible).",
            json!({ "project": project_prop(), "target": { "type": "number", "description": "Taux d'ouverture visé, en %." }, "param": { "type": "string", "enum": ["size", "spacing"] } }),
            &["project", "target"],
        ),
        tool(
            "fabvents_preview",
            "Aperçu PNG du motif (plaque grise, perforations transparentes).",
            json!({ "project": project_prop(), "width": { "type": "integer", "description": "Largeur en pixels (16 à 4096, 800 par défaut)." } }),
            &["project"],
        ),
        tool(
            "fabvents_export",
            "Exporte le projet dans un fichier : `svg` (laser), `dxf` (R12, mm), `step` (solide B-Rep), `step_zip` (une dalle par fichier), `png` ou `project` (JSON). `path` : chemin absolu du fichier à écrire.",
            json!({ "project": project_prop(), "format": { "type": "string", "enum": tools::FORMATS }, "path": { "type": "string" } }),
            &["project", "format", "path"],
        ),
    ])
}

fn text(s: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": s.into() }] })
}

fn failure(s: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": s.into() }], "isError": true })
}

fn project_arg(args: &Value) -> Result<Project, String> {
    let p = args.get("project").ok_or("argument `project` manquant")?;
    let text = if let Some(s) = p.as_str() {
        s.to_string()
    } else {
        p.to_string()
    };
    tools::read_project(&text)
}

/// Exécute un outil ; l'erreur métier revient comme résultat `isError`.
pub fn call(name: &str, args: &Value) -> Value {
    let run = || -> Result<Value, String> {
        match name {
            "fabvents_guide" => Ok(text(tools::GUIDE)),
            "fabvents_new_project" => {
                let p = match args
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("example")
                {
                    "default" => Project::default(),
                    "fan" => {
                        let size =
                            args.get("fan_size").and_then(Value::as_u64).unwrap_or(120) as u32;
                        templates::fan_grill(size)
                            .ok_or("taille de ventilateur inconnue (40, 60, 80, 92, 120 ou 140)")?
                    }
                    _ => Project::example(),
                };
                Ok(text(p.to_json_pretty()))
            }
            "fabvents_compute" => {
                let p = project_arg(args)?;
                Ok(text(
                    serde_json::to_string_pretty(&tools::report(&p)).unwrap_or_default(),
                ))
            }
            "fabvents_solve_open_ratio" => {
                let p = project_arg(args)?;
                let target = args
                    .get("target")
                    .and_then(Value::as_f64)
                    .ok_or("argument `target` manquant")?;
                let param = tools::parse_param(
                    args.get("param").and_then(Value::as_str).unwrap_or("size"),
                )?;
                let (q, s) = tools::solve(&p, target, param)?;
                Ok(text(format!(
                    "{}\n\nProjet réglé :\n{}",
                    serde_json::to_string_pretty(&s).unwrap_or_default(),
                    q.to_json_pretty()
                )))
            }
            "fabvents_preview" => {
                let p = project_arg(args)?;
                let width = args.get("width").and_then(Value::as_u64).unwrap_or(800) as u32;
                let (png, _) = tools::export_bytes(&p, "png", width)?;
                Ok(
                    json!({ "content": [{ "type": "image", "data": base64(&png), "mimeType": "image/png" }] }),
                )
            }
            "fabvents_export" => {
                let p = project_arg(args)?;
                let format = args
                    .get("format")
                    .and_then(Value::as_str)
                    .ok_or("argument `format` manquant")?;
                let path = args
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or("argument `path` manquant")?;
                if !std::path::Path::new(path).is_absolute() {
                    return Err("`path` doit être un chemin absolu".into());
                }
                let (bytes, _) = tools::export_bytes(&p, format, 2000)?;
                std::fs::write(path, &bytes).map_err(|e| format!("écriture impossible : {e}"))?;
                Ok(text(format!("{path} écrit ({} octets).", bytes.len())))
            }
            other => Err(format!("outil inconnu : {other}")),
        }
    };
    run().unwrap_or_else(failure)
}

/// Réponse à un message JSON-RPC (`None` pour une notification).
pub fn handle(msg: &Value) -> Option<Value> {
    let id = msg.get("id")?.clone();
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": params.get("protocolVersion").and_then(Value::as_str).unwrap_or(PROTOCOL),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "fabvents", "version": fabvents_core::ENGINE_VERSION },
            "instructions": "Générateur de motifs de perforation (grilles de ventilation, de haut-parleur, panneaux). Lisez fabvents_guide, partez de fabvents_new_project, vérifiez avec fabvents_compute et fabvents_preview, puis exportez.",
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools_list() })),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            Ok(call(name, &args))
        }
        _ => Err(json!({ "code": -32601, "message": format!("méthode inconnue : {method}") })),
    };
    Some(match result {
        Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err(e) => json!({ "jsonrpc": "2.0", "id": id, "error": e }),
    })
}

/// Boucle stdio : une requête par ligne, une réponse par ligne.
pub fn serve() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle(&msg),
            Err(e) => {
                eprintln!("[fabvents mcp] message illisible : {e}");
                Some(
                    json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": "JSON illisible" } }),
                )
            }
        };
        if let Some(r) = reply {
            writeln!(out, "{r}")?;
            out.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rpc(method: &str, params: Value) -> Value {
        handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params })).unwrap()
    }

    #[test]
    fn handshake_and_tools() {
        let r = rpc("initialize", json!({ "protocolVersion": "2025-06-18" }));
        assert_eq!(r["result"]["serverInfo"]["name"], "fabvents");
        let tools = rpc("tools/list", json!({}));
        assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 6);
        assert!(
            handle(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).is_none()
        );
    }

    #[test]
    fn generate_solve_preview() {
        let r = rpc(
            "tools/call",
            json!({ "name": "fabvents_new_project", "arguments": { "kind": "fan", "fan_size": 80 } }),
        );
        let project: Value =
            serde_json::from_str(r["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
        let c = rpc(
            "tools/call",
            json!({ "name": "fabvents_compute", "arguments": { "project": project } }),
        );
        assert!(c["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("open_ratio"));
        let s = rpc(
            "tools/call",
            json!({ "name": "fabvents_solve_open_ratio", "arguments": { "project": project, "target": 30, "param": "size" } }),
        );
        assert!(s["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("\"reached\""));
        let p = rpc(
            "tools/call",
            json!({ "name": "fabvents_preview", "arguments": { "project": project, "width": 200 } }),
        );
        assert_eq!(p["result"]["content"][0]["mimeType"], "image/png");
        let bad = rpc(
            "tools/call",
            json!({ "name": "fabvents_export", "arguments": { "project": project, "format": "svg", "path": "relatif.svg" } }),
        );
        assert_eq!(bad["result"]["isError"], true);
    }
}
