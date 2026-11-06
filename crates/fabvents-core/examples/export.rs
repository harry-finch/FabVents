//! Exporte le projet d'exemple (ou un projet JSON) en SVG, DXF et STEP
//! (un fichier, et une archive ZIP de dalles si le projet en demande) :
//! `cargo run -p fabvents-core --example export -- <dossier> [projet.json]`.

use fabvents_core::{export, Pipeline, Project};

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = std::path::PathBuf::from(args.next().unwrap_or_else(|| ".".into()));
    let project = match args.next() {
        Some(path) => {
            Project::from_json(&std::fs::read_to_string(path).expect("lecture")).expect("projet")
        }
        None => Project::example(),
    };
    let mut pipeline = Pipeline::new();
    let frame = pipeline.compute(&project);
    for i in &frame.issues {
        eprintln!("{:?} {} {:?}", i.severity, i.code, i.count);
    }
    let d = export::Drawing::new(&project, &frame);
    std::fs::create_dir_all(&dir).expect("dossier");
    std::fs::write(dir.join("fabvents.svg"), export::svg::svg(&d)).expect("svg");
    std::fs::write(dir.join("fabvents.dxf"), export::dxf::dxf(&d)).expect("dxf");
    let ts = "2026-01-01T00:00:00";
    match export::step::step_file(&project, &frame, ts) {
        Ok(s) => std::fs::write(dir.join("fabvents.step"), s).expect("step"),
        Err(e) => eprintln!("STEP : {e}"),
    }
    if project.fabrication.tiles != [1, 1] {
        match export::step::step_zip(&project, &frame, ts) {
            Ok(z) => std::fs::write(dir.join("fabvents-dalles.zip"), z).expect("zip"),
            Err(e) => eprintln!("ZIP : {e}"),
        }
    }
    println!(
        "{} perforations, ouverture {:.1} %, volume sans dépouille {:.4} mm³",
        frame.holes.len(),
        frame.metrics.open_ratio,
        (frame.metrics.plate_area - frame.metrics.open_area) * project.plate.thickness
    );
}
