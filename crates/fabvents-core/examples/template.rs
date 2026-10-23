//! Écrit le projet d'un gabarit de grille de ventilateur :
//! `cargo run -p fabvents-core --example template -- 120 > grille.json`.
fn main() {
    let size: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(120);
    let p = fabvents_core::templates::fan_grill(size)
        .expect("taille connue : 40, 60, 80, 92, 120, 140");
    println!("{}", p.to_json_pretty());
}
