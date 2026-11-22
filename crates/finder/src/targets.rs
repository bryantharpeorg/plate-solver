//! The offline OpenNGC target catalog and its exact-id and common-name search.
//!
//! The pinned extract and its CC BY-SA 4.0 provenance are described in `data/README.md`.
//! Objects and duplicate aliases are loaded once from data compiled into the binary.
//! This catalog is separate from [`crate::catalog::messier`], which supplies the chart.

use std::collections::HashMap;
use std::sync::OnceLock;

use math_core::UnitVector;
use serde::Serialize;

use crate::catalog::{kind_of, Family};
use crate::pushto::{self, Class};
use crate::session::Site;

/// One search result, classed at the console's time for the observing session.
#[derive(Debug, Serialize)]
pub struct SearchResult {
    /// The canonical catalog object.
    #[serde(flatten)]
    pub object: &'static Target,
    /// Pointability now, with the same precedence as the session guidance.
    pub class: Class,
}

impl SearchResult {
    /// Class one result using the site, minimum altitude and time of a single read.
    pub fn at(object: &'static Target, site: Option<Site>, min_alt_deg: f64, time: f64) -> Self {
        Self {
            object,
            class: pushto::class(object, site, min_alt_deg, time),
        }
    }
}

/// One canonical OpenNGC object, shared by its designation, Messier id and aliases.
#[derive(Debug, Clone, Serialize)]
pub struct Target {
    /// Designation with a space and no leading zeros, or a Messier-only id (`M40`).
    pub id: String,
    /// Messier number, as `M31`, when present.
    pub messier: Option<String>,
    /// Every common name given by OpenNGC.
    pub common_names: Vec<String>,
    /// OpenNGC type code, as `G` or `Cl+N`.
    pub code: String,
    /// The type in plain words.
    pub kind: &'static str,
    /// Object family, shared with the Messier chart.
    pub family: Family,
    /// J2000 right ascension in degrees.
    pub ra_deg: f64,
    /// J2000 declination in degrees.
    pub dec_deg: f64,
    /// OpenNGC's constellation abbreviation (including `Se1` and `Se2` for Serpens).
    pub constellation: String,
    /// Major axis in arcminutes, when known.
    pub major_arcmin: Option<f64>,
    /// Minor axis in arcminutes, when known.
    pub minor_arcmin: Option<f64>,
    /// Major-axis position angle in degrees, north through east, when known.
    pub pa_deg: Option<f64>,
    /// Visual magnitude, falling back to blue magnitude, when known.
    pub mag: Option<f64>,
}

impl Target {
    /// Its J2000 direction on the sky.
    pub fn vector(&self) -> UnitVector {
        UnitVector::from_radec(self.ra_deg.to_radians(), self.dec_deg.to_radians())
    }
}

struct Catalog {
    objects: Vec<Target>,
    ids: HashMap<String, usize>,
}

/// A key keeps designation suffixes, removing only the space and initial numeric zeros.
fn normalize(id: &str) -> String {
    let id = id.trim();
    match id.find(|c: char| c.is_ascii_digit()) {
        Some(digits) => format!(
            "{}{}",
            id[..digits].trim_end(),
            id[digits..].trim_start_matches('0')
        )
        .to_ascii_uppercase(),
        None => id.to_ascii_uppercase(),
    }
}

// Same display form as catalog::designation; Messier-only names retain their M id.
fn canonical_id(name: &str) -> String {
    let digits = name
        .find(|c: char| c.is_ascii_digit())
        .expect("catalog number");
    let (catalog, number) = name.split_at(digits);
    let space = if catalog == "M" { "" } else { " " };
    format!("{catalog}{space}{}", number.trim_start_matches('0'))
}

impl Catalog {
    fn parse(extract: &str) -> Self {
        let mut catalog = Self {
            objects: Vec::new(),
            ids: HashMap::new(),
        };
        let mut aliases = Vec::new();
        for line in extract
            .lines()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
        {
            let f: Vec<_> = line.split(';').collect();
            assert_eq!(f.len(), 12, "OpenNGC extract columns");
            match f[1] {
                "NonEx" => continue,
                "Dup" => {
                    aliases.push((f[0], f[11]));
                    continue;
                }
                _ => {}
            }
            let opt =
                |s: &str| (!s.is_empty()).then(|| s.parse::<f64>().expect("numeric OpenNGC field"));
            let index = catalog.objects.len();
            catalog.ids.insert(normalize(f[0]), index);
            if !f[9].is_empty() {
                catalog.ids.insert(normalize(f[9]), index);
            }
            catalog.objects.push(Target {
                id: canonical_id(f[0]),
                messier: (!f[9].is_empty()).then(|| f[9].to_string()),
                common_names: f[10]
                    .split(',')
                    .filter(|name| !name.is_empty())
                    .map(str::to_string)
                    .collect(),
                code: f[1].to_string(),
                kind: kind_of(f[1]),
                family: Family::of(f[1]),
                ra_deg: f[2].parse().expect("OpenNGC right ascension"),
                dec_deg: f[3].parse().expect("OpenNGC declination"),
                constellation: f[4].to_string(),
                major_arcmin: opt(f[5]),
                minor_arcmin: opt(f[6]),
                pa_deg: opt(f[7]),
                mag: opt(f[8]),
            });
        }
        // The extract resolves aliases directly to originals, which can occur later.
        for (alias, original) in aliases {
            let index = *catalog
                .ids
                .get(&normalize(original))
                .expect("OpenNGC duplicate's original");
            catalog.ids.insert(normalize(alias), index);
        }
        catalog
    }

    fn search(&self, query: &str, limit: usize) -> Vec<&Target> {
        let query = query.trim().to_lowercase();
        // Names are matched by letters and digits only, and every id holds one, so a
        // query without any finds nothing.
        let terms = words(&query);
        if terms.is_empty() || limit == 0 {
            return Vec::new();
        }
        let exact = self
            .ids
            .get(&normalize(&query))
            .map(|&index| &self.objects[index]);
        let mut matches: Vec<_> = self
            .objects
            .iter()
            .filter(|object| {
                exact.is_some_and(|exact| std::ptr::eq(*object, exact))
                    || object
                        .common_names
                        .iter()
                        .any(|name| name_matches(name, &terms))
            })
            .collect();
        matches.sort_by(|a, b| {
            let is_exact = |object: &Target| exact.is_some_and(|exact| std::ptr::eq(object, exact));
            is_exact(b)
                .cmp(&is_exact(a))
                .then_with(|| {
                    a.mag
                        .unwrap_or(f64::INFINITY)
                        .total_cmp(&b.mag.unwrap_or(f64::INFINITY))
                })
                .then_with(|| a.id.cmp(&b.id))
        });
        matches.truncate(limit);
        matches
    }
}

/// The words of `text`, lowercased, each reduced to its letters and digits.
fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split_whitespace()
        .map(|word| word.chars().filter(|c| c.is_alphanumeric()).collect())
        .filter(|word: &String| !word.is_empty())
        .collect()
}

/// Whether each of `terms` begins a word of `name`, in any order, or the terms run
/// together read straight along the name from the start of one of its words.
fn name_matches(name: &str, terms: &[String]) -> bool {
    let words = words(name);
    let typed = terms.concat();
    let each_begins_a_word = terms
        .iter()
        .all(|term| words.iter().any(|word| word.starts_with(term.as_str())));
    each_begins_a_word || (0..words.len()).any(|from| words[from..].concat().starts_with(&typed))
}

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| Catalog::parse(include_str!("../data/openngc.csv")))
}

/// Every canonical object, excluding duplicate and nonexistent entries.
pub fn all() -> &'static [Target] {
    &catalog().objects
}

/// Find a Messier, NGC, IC or addendum id, including duplicate aliases.
///
/// Case, the space before the number and its leading zeros do not matter.
pub fn find(id: &str) -> Option<&'static Target> {
    let catalog = catalog();
    catalog
        .ids
        .get(&normalize(id))
        .map(|&index| &catalog.objects[index])
}

/// Exact ids first, then common names in brightness order.
///
/// In a common name only letters and digits are compared, whatever their case. A name
/// matches when each word of the query begins one of its words, in any order, or when
/// the query reads straight along the name from the start of one of its words. Objects
/// without a magnitude come last; canonical id breaks equal-magnitude ties. At most
/// `limit` objects are returned, with no duplicates.
pub fn search(query: &str, limit: usize) -> Vec<&'static Target> {
    catalog().search(query, limit)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use math_core::angular_distance;

    use super::*;
    use crate::catalog;

    const EXTRACT: &str = include_str!("../data/openngc.csv");
    const README: &str = include_str!("../data/README.md");

    #[test]
    fn object_and_alias_counts_match_the_pinned_extract_readme() {
        // Oracle: the maintainer's pinned extraction recipe; counts must agree exactly.
        let counts = README.split("**Object count: ").nth(1).unwrap();
        let words: Vec<_> = counts.split_whitespace().collect();
        let objects: usize = words[0].parse().unwrap();
        let aliases: usize = words[3].parse().unwrap();
        assert_eq!(all().len(), objects);
        assert_eq!(
            EXTRACT
                .lines()
                .filter(|row| row.split(';').nth(1) == Some("Dup"))
                .count(),
            aliases
        );
        let ids: HashSet<_> = all().iter().map(|object| &object.id).collect();
        assert_eq!(ids.len(), objects, "one canonical id per object");
        assert!(all()
            .iter()
            .all(|object| object.code != "Dup" && object.code != "NonEx"));
    }

    #[test]
    fn every_duplicate_resolves_to_its_original_without_becoming_an_object() {
        // Oracle: every alias and its destination in the pinned OpenNGC extract.
        let objects: HashSet<_> = all().iter().map(|object| object.id.as_str()).collect();
        let mut aliases = 0;
        for fields in EXTRACT
            .lines()
            .skip(1)
            .map(|row| row.split(';').collect::<Vec<_>>())
        {
            if fields[1] != "Dup" {
                continue;
            }
            aliases += 1;
            let duplicate = find(fields[0]).unwrap_or_else(|| panic!("missing {}", fields[0]));
            let original = find(fields[11]).unwrap_or_else(|| panic!("missing {}", fields[11]));
            assert!(
                std::ptr::eq(duplicate, original),
                "{} -> {}",
                fields[0],
                fields[11]
            );
            assert!(objects.contains(original.id.as_str()));
            let digits = fields[0].find(|c: char| c.is_ascii_digit()).unwrap();
            let alias_id = format!(
                "{} {}",
                &fields[0][..digits],
                fields[0][digits..].trim_start_matches('0')
            );
            assert!(
                !objects.contains(alias_id.as_str()),
                "alias {alias_id} listed as an object"
            );
        }
        assert_eq!(aliases, 651);
    }

    #[test]
    fn every_messier_position_agrees_with_the_existing_chart_catalog_within_one_arcsecond() {
        // Independent oracle: the console's existing 110-object Messier extract.
        assert_eq!(catalog::messier().len(), 110);
        for reference in catalog::messier() {
            let object = find(&reference.id).unwrap_or_else(|| panic!("missing {}", reference.id));
            let arcseconds =
                angular_distance(object.vector(), reference.vector()).to_degrees() * 3600.0;
            assert!(
                arcseconds < 1.0,
                "{} differs by {arcseconds} arcseconds",
                reference.id
            );
            assert_eq!(object.messier.as_deref(), Some(reference.id.as_str()));
            let expected_id = reference.designation.as_ref().unwrap_or(&reference.id);
            assert_eq!(&object.id, expected_id);
        }
    }

    #[test]
    fn disputed_and_addendum_messier_ids_follow_the_extract_notes() {
        for (query, id) in [
            ("M102", "NGC 5866"),
            ("M76", "NGC 650"),
            ("M45", "Mel 22"),
            ("M40", "M40"),
        ] {
            assert_eq!(find(query).unwrap().id, id);
        }
        assert!(std::ptr::eq(find("NGC651").unwrap(), find("M76").unwrap()));
        assert!(find("M111").is_none());
    }

    #[test]
    fn case_spacing_and_leading_zeros_find_the_same_canonical_object() {
        let andromeda = find("NGC 224").unwrap();
        for query in ["m31", "M 31", "ngc224", "NGC 0224", "ngc0224", " m0031 "] {
            assert!(std::ptr::eq(find(query).unwrap(), andromeda), "{query}");
        }
        assert_eq!(andromeda.id, "NGC 224");
        for (query, id) in [
            ("ic0434", "IC 434"),
            ("mel00022", "Mel 22"),
            ("pgc000143", "PGC 143"),
            ("eso351-030", "ESO 351-030"),
            ("ngc1530a", "NGC 1530A"),
        ] {
            assert_eq!(find(query).unwrap().id, id);
        }
    }

    #[test]
    fn unknown_ids_are_absent() {
        for query in ["NGC 99999", "M111", "", "qqq", "NGC"] {
            assert!(find(query).is_none(), "{query}");
        }
    }

    #[test]
    fn andromeda_preserves_the_openngc_fields() {
        // Oracle: NGC0224's row, with numeric fields compared exactly to its decimals.
        let object = find("M31").unwrap();
        assert_eq!(object.common_names, ["Andromeda Galaxy"]);
        assert_eq!(object.code, "G");
        assert_eq!((object.ra_deg, object.dec_deg), (10.68479, 41.26906));
        assert_eq!(object.constellation, "And");
        assert_eq!(
            (
                object.major_arcmin,
                object.minor_arcmin,
                object.pa_deg,
                object.mag
            ),
            (Some(177.83), Some(69.66), Some(35.0), Some(3.44))
        );
        assert_eq!(
            find("M76").unwrap().common_names,
            ["Barbell Nebula", "Cork Nebula", "Little Dumbbell Nebula"]
        );
        assert!(find("M22").unwrap().mag.is_none());
    }

    #[test]
    fn search_uses_name_word_prefixes_and_orders_by_magnitude_with_unknowns_last() {
        // Oracle: the extract's 60 names with a word beginning "nebula", and magnitudes.
        let matches = search("NEBULA", usize::MAX);
        assert_eq!(matches.len(), 60);
        assert_eq!(matches[0].id, "NGC 3372");
        assert!(matches.iter().any(|object| object.id == "NGC 1976"));
        assert_eq!(
            matches
                .iter()
                .filter(|object| object.id == "NGC 1976")
                .count(),
            1
        );
        let mut unknown = false;
        let mut previous = f64::NEG_INFINITY;
        for object in matches {
            if let Some(mag) = object.mag {
                assert!(!unknown, "known magnitude after an unknown one");
                assert!(mag >= previous, "objects must be brightest first");
                previous = mag;
            } else {
                unknown = true;
            }
        }
        assert!(unknown);
        assert_eq!(
            search("cork", 20)[0].id,
            "NGC 650",
            "all common names are searched"
        );
        assert_eq!(search("orion", 20)[0].id, "NGC 1976", "match any word");
        assert!(
            search("romeda", 20).is_empty(),
            "no substring matching inside a word"
        );
        assert!(search("qqq", 20).is_empty());
        assert!(search("  ", 20).is_empty());
        assert!(search("nebula", 0).is_empty());
        assert_eq!(search("M31", 1)[0].id, "NGC 224");
        assert_eq!(search("ngc0651", 20)[0].id, "NGC 650");
    }

    #[test]
    fn exact_ids_precede_brighter_name_matches_and_nonexistent_rows_are_excluded() {
        // Independent fixture: the exact M31 is faint, also matches by name, and has
        // a duplicate before its original. The other three matches have known ordering.
        let extract = [
            EXTRACT.lines().next().unwrap(),
            "IC0001;Dup;;;;;;;;;;NGC0001",
            "NGC0001;G;10;20;And;;;;15;M31;M31 Faint Galaxy;",
            "NGC0002;G;10;20;And;;;;1;;M31 Bright Galaxy;",
            "NGC0003;G;10;20;And;;;;;;M31 Unknown Galaxy;",
            "NGC0004;G;10;20;And;;;;3;;M31 Medium Galaxy;",
            "NGC0005;NonEx;;;;;;;;;;",
            "",
        ]
        .join("\n");
        let catalog = Catalog::parse(&extract);
        assert_eq!(catalog.objects.len(), 4);
        let ids: Vec<_> = catalog
            .search("m31", 20)
            .into_iter()
            .map(|object| object.id.as_str())
            .collect();
        assert_eq!(ids, ["NGC 1", "NGC 2", "NGC 4", "NGC 3"]);
        assert_eq!(catalog.search("ic1", 1)[0].id, "NGC 1");
        assert!(catalog.search("ngc5", 20).is_empty());
    }

    #[test]
    fn every_source_object_is_reachable_by_its_raw_and_canonical_id() {
        for fields in EXTRACT
            .lines()
            .skip(1)
            .map(|line| line.split(';').collect::<Vec<_>>())
        {
            if fields[1] == "Dup" {
                continue;
            }
            let object = find(fields[0]).unwrap_or_else(|| panic!("missing {}", fields[0]));
            assert!(std::ptr::eq(object, find(&object.id).unwrap()));
            assert_eq!(object.code, fields[1]);
            assert_eq!(object.constellation, fields[4]);
        }
    }

    fn ids(matches: Vec<&Target>) -> Vec<&str> {
        matches
            .into_iter()
            .map(|object| object.id.as_str())
            .collect()
    }

    #[test]
    fn whole_names_and_unordered_word_prefixes_find_the_named_object() {
        // Oracle: the pinned extract's common names; exact ids and order, no tolerance.
        for (query, expected) in [
            ("Orion Nebula", "NGC 1976"),
            ("Great Orion Nebula", "NGC 1976"),
            ("orion neb", "NGC 1976"),
            ("ring NEBULA", "NGC 6720"),
            ("Whirlpool Galaxy", "NGC 5194"),
            ("Double Cluster", "C 14"),
            ("h & chi Persei", "C 14"),
            ("North America Nebula", "NGC 7000"),
            ("  Ring   Nebula ", "NGC 6720"),
            ("nebula orion", "NGC 1976"),
            ("nebula\torion", "NGC 1976"),
            ("gr or ne", "NGC 1976"),
            ("whirl gal", "NGC 5194"),
        ] {
            assert_eq!(ids(search(query, 20)), [expected], "{query:?}");
        }
    }

    #[test]
    fn words_must_match_within_one_name_from_word_starts() {
        // Oracle: no single extract name satisfies these queries; no fuzzy matches.
        for query in [
            "orion galaxy",
            "andromeda nebula",
            "barbell cork",
            "rion nebula",
            "orion nebulae",
            "tie",
            "tie nebula",
            "orion é",
            "bow bow tie",
        ] {
            assert!(search(query, 20).is_empty(), "{query:?}");
        }
    }

    #[test]
    fn matching_names_keep_magnitude_order_limits_and_canonical_id_ties() {
        // Oracle: the extract's names and magnitudes, including equal and absent ones.
        for (query, limit, expected) in [
            ("cluster s", 20, vec!["Cl 399", "NGC 6087", "Mel 111"]),
            ("little nebula", 20, vec!["NGC 6818", "NGC 650", "NGC 6369"]),
            ("little nebula", 2, vec!["NGC 6818", "NGC 650"]),
            ("eagle nebula", 20, vec!["IC 4703", "NGC 6611"]),
            ("orion o", 20, vec!["NGC 1976", "IC 434"]),
        ] {
            assert_eq!(ids(search(query, limit)), expected, "{query:?}, {limit}");
        }
    }

    #[test]
    fn spaced_exact_id_precedes_brighter_names_read_from_any_word() {
        // Independent five-row fixture: exact M31 is faint, one name has no magnitude,
        // and M 3 must not match M 31. Expected ids and ordering are exact.
        let extract = [
            EXTRACT.lines().next().unwrap(),
            "NGC0001;G;10;20;And;;;;15;M31;Faint Galaxy;",
            "NGC0002;G;10;20;And;;;;1;;M 31 Bright Galaxy;",
            "NGC0003;G;10;20;And;;;;;;Galaxy M 31;",
            "NGC0004;G;10;20;And;;;;3;;Medium M31 Galaxy;",
            "NGC0005;G;10;20;And;;;;2;;M 3 Galaxy;",
        ]
        .join("\n");
        let catalog = Catalog::parse(&extract);
        assert_eq!(
            ids(catalog.search("M 31", 20)),
            ["NGC 1", "NGC 2", "NGC 4", "NGC 3"]
        );
    }

    #[test]
    fn every_source_common_name_in_capitals_finds_its_object() {
        // Oracle: raw extract names, counted per row; unchanged find supplies identity.
        let mut names = 0;
        for fields in EXTRACT
            .lines()
            .skip(1)
            .map(|row| row.split(';').collect::<Vec<_>>())
        {
            if matches!(fields[1], "Dup" | "NonEx") {
                continue;
            }
            let object = find(fields[0]).unwrap();
            for name in fields[10].split(',').filter(|name| !name.is_empty()) {
                names += 1;
                assert!(
                    ids(search(&name.to_uppercase(), usize::MAX)).contains(&object.id.as_str()),
                    "{name:?} must find {}",
                    object.id
                );
            }
        }
        assert_eq!(names, 182);
    }

    #[test]
    fn all_source_word_prefixes_preserve_old_matches_with_only_declared_gains() {
        // Oracle: 012's whitespace-word prefix rule applied directly to extract text,
        // plus the spec's exhaustive literal gains. Compare sets exactly, no tolerance.
        let mut prefixes: HashMap<String, HashSet<&str>> = HashMap::new();
        for fields in EXTRACT
            .lines()
            .skip(1)
            .map(|row| row.split(';').collect::<Vec<_>>())
        {
            if matches!(fields[1], "Dup" | "NonEx") {
                continue;
            }
            let id = find(fields[0]).unwrap().id.as_str();
            for name in fields[10].split(',').filter(|name| !name.is_empty()) {
                for word in name.split_whitespace() {
                    let word = word.to_lowercase();
                    for (offset, character) in word.char_indices() {
                        prefixes
                            .entry(word[..offset + character.len_utf8()].to_string())
                            .or_default()
                            .insert(id);
                    }
                }
            }
        }
        assert_eq!(prefixes.len(), 951);
        let queries: Vec<_> = prefixes
            .iter()
            .filter(|(query, _)| query.chars().any(char::is_alphanumeric))
            .collect();
        assert_eq!(queries.len(), 950);
        for (query, baseline) in queries {
            let gains: &[&str] = match query.as_str() {
                "l'" => &[
                    "ESO 56-115",
                    "NGC 1980",
                    "IC 2944",
                    "NGC 6523",
                    "NGC 6618",
                    "NGC 6818",
                    "NGC 650",
                    "UGC 5470",
                    "NGC 6445",
                    "NGC 6369",
                ],
                "l'e" => &["UGC 5470"],
                "le" => &["NGC 6705"],
                "sn" | "sno" => &["NGC 6087"],
                _ => &[],
            };
            let mut expected = baseline.clone();
            expected.extend(gains.iter().copied());
            let actual: HashSet<_> = ids(search(query, usize::MAX)).into_iter().collect();
            assert!(
                baseline.is_subset(&actual),
                "lost a 012 match for {query:?}"
            );
            assert_eq!(actual, expected, "{query:?}");
        }
    }

    #[test]
    fn every_canonical_and_spaced_messier_id_finds_exactly_its_object() {
        // Oracle: unchanged all() and its canonical/Messier identities, exact lists.
        for object in all() {
            assert_eq!(
                ids(search(&object.id, usize::MAX)),
                [object.id.as_str()],
                "{}",
                object.id
            );
            if let Some(messier) = &object.messier {
                let query = format!("M {}", messier.strip_prefix('M').unwrap());
                assert_eq!(
                    ids(search(&query, usize::MAX)),
                    [object.id.as_str()],
                    "{query}"
                );
            }
        }
    }

    #[test]
    fn letters_and_digits_match_across_marks_and_joined_or_split_words() {
        // Oracle: the extract's marked and compound common names, exact ids.
        for (query, expected) in [
            ("bow tie", "NGC 40"),
            ("bowtie", "NGC 40"),
            ("Bow-Tie", "NGC 40"),
            ("cats eye", "NGC 6543"),
            ("catseye", "NGC 6543"),
            ("catseye nebula", "NGC 6543"),
            ("cat's eye", "NGC 6543"),
            ("cat’s eye", "NGC 6543"),
            ("cat eye nebula", "NGC 6543"),
            ("bear paw", "NGC 2537"),
            ("eight burst", "NGC 3132"),
            ("Wolf Lundmark Melotte", "PGC 143"),
            ("Wolf-Lundmark-Melotte", "PGC 143"),
            ("stephans quintet", "HCG 92"),
            ("bode galaxy", "NGC 3031"),
            ("horse head nebula", "B 33"),
            ("whirl pool", "NGC 5194"),
            ("jewelbox", "NGC 4755"),
            ("h chi persei", "C 14"),
        ] {
            assert_eq!(ids(search(query, 20)), [expected], "{query:?}");
        }
    }

    #[test]
    fn queries_without_letters_or_digits_find_nothing() {
        // Oracle: the spec's no-word rule, including the extract's ampersand name.
        for query in ["-", "'", "--", "’", "' -", "&"] {
            assert!(search(query, 20).is_empty(), "{query:?}");
        }
    }
}
