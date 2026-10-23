//! Temps de calcul par étage d'un projet JSON :
//! `cargo run --release -p fabvents-core --example bench -- projet.json`.

use fabvents_core::field::Field;
use fabvents_core::layout::seeds;
use fabvents_core::metrics::measure;
use fabvents_core::project::Channel;
use fabvents_core::{Pipeline, Project};
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("projet.json");
    let p = Project::from_json(&std::fs::read_to_string(path).expect("lecture")).expect("projet");
    let t = Instant::now();
    let field = Field::new(&p.controllers);
    let s = seeds(&p.layout, &p.plate, &field.channel(Channel::Spacing)).expect("germes");
    let t_seeds = t.elapsed();
    let mut pl = Pipeline::new();
    let t = Instant::now();
    let f = pl.compute(&p);
    let t_all = t.elapsed();
    let holes = f.holes.to_vec();
    let t = Instant::now();
    let m = measure(&holes, p.plate.area(), p.fabrication.min_bridge);
    let t_metrics = t.elapsed();
    println!(
        "germes {} en {:?} ; total {:?} pour {} trous ; mesures {:?} ({} chevauchements)",
        s.len(),
        t_seeds,
        t_all,
        holes.len(),
        t_metrics,
        m.overlaps
    );
}
