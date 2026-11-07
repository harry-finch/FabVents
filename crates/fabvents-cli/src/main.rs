//! `fabvents` : le moteur FabVents en ligne de commande, et serveur MCP.
//!
//! ```text
//! fabvents info <projet.json>                         mesures et messages (JSON)
//! fabvents export <projet.json> <sortie>…             export selon l'extension :
//!                                                     .svg .dxf .step .zip .png .json
//! fabvents solve <projet.json> --target 40 [--param size|spacing] [-o réglé.json]
//! fabvents template <40|60|80|92|120|140>             gabarit de grille de ventilateur
//! fabvents example                                    projet d'exemple
//! fabvents mcp                                        serveur MCP sur stdio
//! ```

mod mcp;
mod render;
mod tools;

use std::process::ExitCode;

const USAGE: &str = "Usage :
  fabvents info <projet.json>
  fabvents export <projet.json> <sortie.svg|.dxf|.step|.zip|.png|.json>… [--width px]
  fabvents solve <projet.json> --target <%> [--param size|spacing] [-o <réglé.json>]
  fabvents template <40|60|80|92|120|140>
  fabvents example
  fabvents mcp";

fn read(path: &str) -> Result<fabvents_core::Project, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path} : {e}"))?;
    tools::read_project(&text).map_err(|e| format!("{path} : {e}"))
}

/// Valeur d'une option `--nom valeur`.
fn option<'a>(args: &'a [String], names: &[&str]) -> Option<&'a str> {
    args.windows(2)
        .find(|w| names.contains(&w[0].as_str()))
        .map(|w| w[1].as_str())
}

fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("mcp") => mcp::serve().map_err(|e| e.to_string()),
        Some("example") => {
            println!("{}", fabvents_core::Project::example().to_json_pretty());
            Ok(())
        }
        Some("template") => {
            let size: u32 = args.get(1).and_then(|s| s.parse().ok()).ok_or(USAGE)?;
            let p = fabvents_core::templates::fan_grill(size)
                .ok_or("taille de ventilateur inconnue (40, 60, 80, 92, 120 ou 140)")?;
            println!("{}", p.to_json_pretty());
            Ok(())
        }
        Some("info") => {
            let p = read(args.get(1).ok_or(USAGE)?)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&tools::report(&p)).unwrap_or_default()
            );
            Ok(())
        }
        Some("export") => {
            let p = read(args.get(1).ok_or(USAGE)?)?;
            let width: u32 = option(args, &["--width"])
                .and_then(|s| s.parse().ok())
                .unwrap_or(2000);
            let outs: Vec<&String> = args[2..]
                .iter()
                .filter(|a| a.contains('.') && !a.starts_with("--"))
                .collect();
            if outs.is_empty() {
                return Err(USAGE.into());
            }
            for out in outs {
                let lower = out.to_lowercase();
                let format = if lower.ends_with(".svg") {
                    "svg"
                } else if lower.ends_with(".dxf") {
                    "dxf"
                } else if lower.ends_with(".step") || lower.ends_with(".stp") {
                    "step"
                } else if lower.ends_with(".zip") {
                    "step_zip"
                } else if lower.ends_with(".png") {
                    "png"
                } else if lower.ends_with(".json") {
                    "project"
                } else {
                    return Err(format!("{out} : extension inconnue"));
                };
                let (bytes, _) = tools::export_bytes(&p, format, width)?;
                std::fs::write(out, &bytes).map_err(|e| format!("{out} : {e}"))?;
                eprintln!("{out} ({} octets)", bytes.len());
            }
            Ok(())
        }
        Some("solve") => {
            let p = read(args.get(1).ok_or(USAGE)?)?;
            let target: f64 = option(args, &["--target", "-t"])
                .and_then(|s| s.parse().ok())
                .ok_or(USAGE)?;
            let param = tools::parse_param(option(args, &["--param", "-p"]).unwrap_or("size"))?;
            let (q, s) = tools::solve(&p, target, param)?;
            eprintln!("{}", serde_json::to_string_pretty(&s).unwrap_or_default());
            let json = q.to_json_pretty();
            match option(args, &["-o", "--output"]) {
                Some(out) => std::fs::write(out, json).map_err(|e| format!("{out} : {e}"))?,
                None => println!("{json}"),
            }
            Ok(())
        }
        Some("--version" | "-V") => {
            println!("fabvents {}", fabvents_core::ENGINE_VERSION);
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
