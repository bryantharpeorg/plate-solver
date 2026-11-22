//! Small databases built from vendored stars, cached only as complete files in target.

use crate::support::live::{sim, DB};
use database_generation::{
    build_pattern_catalog, clean_and_limit, enumerate_patterns, parse_bsc5, serialize_to_path,
    CatalogEntry, CatalogId,
};
use finder::databases::{Database, Databases};
use pattern_database::{DatabaseProperties, PatternDatabase};
use std::path::Path;
use std::sync::{Arc, LazyLock};

mod several;

pub static NARROW: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    generated("narrow", 4.5, 7.0, 20.0, "hip_main", || {
        let simulator = sim();
        simulator.set_optics(simulator::Optics {
            fov_deg: 30.0,
            ..Default::default()
        });
        simulator
            .field_stars(1e6, 100.0)
            .into_iter()
            .map(|(index, _)| {
                let star = &DB.star_table[index * 6..][..6];
                CatalogEntry {
                    ra: f64::from(star[0]),
                    dec: f64::from(star[1]),
                    mag: f64::from(star[5]),
                    id: CatalogId::Hip(index as u32 + 1),
                    pm_ra: None,
                    pm_dec: None,
                }
            })
            .collect()
    })
});

pub static MID: LazyLock<Arc<PatternDatabase>> =
    LazyLock::new(|| generated("mid", 20.0, 30.0, 20.0, "bsc5", bsc5));

pub static WIDE: LazyLock<Arc<PatternDatabase>> =
    LazyLock::new(|| generated("wide", 35.0, 50.0, 100.0, "bsc5", bsc5));

fn bsc5() -> Vec<CatalogEntry> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tests/data/bsc5");
    parse_bsc5(std::fs::File::open(path).unwrap()).unwrap()
}

fn generated(
    name: &str,
    min: f64,
    max: f64,
    oversampling: f64,
    star_catalog: &str,
    entries: impl FnOnce() -> Vec<CatalogEntry>,
) -> Arc<PatternDatabase> {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(folder).unwrap();
    let path = folder.join(format!("047-{name}.npz"));
    if !path.exists() {
        let mut entries = entries();
        clean_and_limit(&mut entries, None);
        let patterns = enumerate_patterns(&entries, min, max, 150.0, oversampling, 50);
        let catalog = build_pattern_catalog(&entries, &patterns, 0.001, false);
        let properties = DatabaseProperties {
            pattern_mode: "edge_ratio".into(),
            hash_table_type: "quadratic_probe".into(),
            pattern_size: 4,
            pattern_bins: catalog.pattern_bins as u16,
            pattern_max_error: 0.001,
            min_fov: min as f32,
            max_fov: max as f32,
            star_catalog: star_catalog.into(),
            epoch_equinox: 2000,
            epoch_proper_motion: 2000.0,
            verification_stars_per_fov: 150,
            star_max_magnitude: 0.0,
            num_patterns: catalog.num_patterns as u32,
        };
        let part = folder.join(format!("047-{name}.npz.{}.part", std::process::id()));
        serialize_to_path(&part, &entries, &catalog, &properties).unwrap();
        std::fs::rename(part, &path).unwrap();
    }
    Arc::new(pattern_database::load_from_path(&path).unwrap())
}

pub fn databases(list: &[(&str, &Arc<PatternDatabase>)]) -> Databases {
    Databases::new(
        list.iter()
            .map(|(name, db)| Database {
                name: (*name).into(),
                db: (*db).clone(),
            })
            .collect(),
    )
    .unwrap()
}
