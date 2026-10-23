//! Exporte le projet d'exemple (ou un projet JSON) en SVG et DXF :
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
    println!(
        "{} perforations, ouverture {:.1} %",
        frame.holes.len(),
        frame.metrics.open_ratio
    );
}
