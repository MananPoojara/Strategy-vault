//! R4 #386 (Q390 family audit, lane K4, 24 Nov 2026): families that group DISTINCT printed systems are split so each system
//! is its own method (variants of ONE system stay together). Display grouping only: no id, rule or result changes.
//! Every family listed here was source-checked (authors/conv-1123/Q390-families.tsv, column 'applied'): its members are
//! separately named / separately paged systems of one book or pack.
//! `Each`: every id is its own method. `Stem`: ids whose names share a system stem (name before '(' / ' · ' / ' w/', digits
//! and punctuation removed) stay together.
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq)]
enum Mode { Each, Stem }
const SPLIT: &[(&str, Mode)] = &[
    // R4 #387 (24 Nov 2026): re-checked after the independent audit. A family is listed only if EVERY member is a separately named /
    // separately printed system; filter, fill, stop, parameter, version and table-row variants of one system stay ONE method.
    // Reverted after the audit (grids / variants / table rows): msws_gm3, msws_gm2, msws_gm4, acbff_intraday, lwnl_commodity, lwnl_seasonal, jsats, msadd_*, r3r4, ...
    // Stem (variants of one system grouped by name stem):
    ("mstt_wow", Mode::Stem), ("kets_ma", Mode::Stem), ("kdta_breadth", Mode::Stem), ("mbsot_appel", Mode::Stem),
    // Each (every member its own system):
    ("mssta_systems", Mode::Each), ("jchr2", Mode::Each), ("msws_gm8", Mode::Each), ("mstt_equis", Mode::Each), ("khtwg", Mode::Each),
    ("apwms_other", Mode::Each), ("mbhfe_macro", Mode::Each), ("kdta_other", Mode::Each), ("capttm_other", Mode::Each), ("cktwo", Mode::Each),
    ("tdperl_other", Mode::Each), ("apsmts_other", Mode::Each), ("apoi_other", Mode::Each), ("lbi12_twelve", Mode::Each),
    ("lebcafm_daytrading", Mode::Each), ("mbsot_macro", Mode::Each), ("tkttl_setups", Mode::Each), ("jchr1_other", Mode::Each),
    ("hbqts_filters", Mode::Each), ("lebfut_futures", Mode::Each), ("jhbct_charting", Mode::Each), ("tdwb_other", Mode::Each),
    ("vsbrw_other", Mode::Each), ("mslib_misc", Mode::Each), ("mslib_hr", Mode::Each), ("cov_tfm", Mode::Each), ("lwdg2_daytrade", Mode::Each),
    ("putg_studies", Mode::Each), ("jbcdt_intraday", Mode::Each), ("hsta_stats", Mode::Each), ("tdnmtt_other", Mode::Each),
    ("maetf_alloc", Mode::Each), ("cbpst_acme", Mode::Each), ("ramtm_models", Mode::Each), ("acbff_other", Mode::Each), ("mzwws_models", Mode::Each),
    ("mpmm_interp", Mode::Each), ("cgift_nt", Mode::Each), ("dgnl_spi", Mode::Each), ("atm01_tsl", Mode::Each), ("kllien_nt", Mode::Each),
    ("pbwts_examples", Mode::Each), ("ehmd_indicators", Mode::Each), ("tdnsta_other", Mode::Each), ("mzwws_seasonal", Mode::Each),
    ("wbmdd_other", Mode::Each), ("jbcaf_nt", Mode::Each), ("dgnl_methods", Mode::Each), ("atm00_tsl", Mode::Each), ("gg_nt", Mode::Each),
    ("bstacm_nt", Mode::Each), ("mmft_boucher", Mode::Each), ("lwdg2_tools", Mode::Each),
];

/// R4 #387 variant check on the 'Each' families: ids that differ only by a number, parameter or version suffix are ONE method.
/// (ids, shown name); the method key is the first id.
const MERGE: &[(&[&str], &str)] = &[
    (&["mssta_regress_slope_1", "mssta_regress_slope_2", "mssta_regress_slope_3", "mssta_regress_slope_4"], "Regress Slope"),
    (&["mssta_ma_100", "mssta_ma_50"], "Moving average (100% / 50%)"),
    (&["mstt_tezel_maavpb", "mstt_tezel_regavpb"], "Asymmetric Volatility Price Bands (Tezel)"),
    (&["tdperl_tdma1", "tdperl_tdma2"], "TD Moving Average"),
    (&["tdperl_demarker1", "tdperl_demarker2"], "TD DeMarker"),
];
fn merged(id: &str) -> Option<(&'static str, &'static str)> {
    MERGE.iter().find(|(ids, _)| ids.contains(&id)).map(|(ids, name)| (ids[0], *name))
}

/// System stem of a strategy name (same normalisation as the audit: authors/conv-1123/Q390-families.tsv).
pub fn stem(name: &str) -> String {
    let n = name.to_lowercase();
    let n = n.split('(').next().unwrap_or("").split('·').next().unwrap_or("").split(" w/").next().unwrap_or("").to_string();
    let n: String = n.chars().map(|c| if c.is_ascii_digit() || ".%,$/-".contains(c) { ' ' } else { c }).collect();
    n.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn names() -> &'static HashMap<String, &'static str> {
    static M: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    M.get_or_init(|| crate::books::catalogue().into_iter().map(|d| (d.id.to_string(), d.name)).collect())
}
/// Group name of a stem group = its shortest member name (ties: alphabetical), so every member shows the same title.
fn groups() -> &'static HashMap<String, &'static str> {
    static G: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    G.get_or_init(|| {
        let mut g: HashMap<String, &'static str> = HashMap::new();
        for d in crate::books::catalogue() {
            let Some(f) = crate::books::family_of(d.id) else { continue };
            if !SPLIT.iter().any(|(fam, m)| *fam == f.0 && *m == Mode::Stem) { continue; }
            let key = format!("{}~{}", f.0, stem(d.name));
            let better = |cur: &str| (d.name.len(), d.name) < (cur.len(), cur);
            if g.get(&key).map_or(true, |cur| better(cur)) { g.insert(key, d.name); }
        }
        g
    })
}
fn leak(s: String) -> &'static str { Box::leak(s.into_boxed_str()) }

/// Apply the split to a module's (family id, family name, variant) answer for `id`.
pub fn apply(id: &str, f: (&'static str, &'static str, &'static str)) -> (&'static str, &'static str, &'static str) {
    let Some((_, mode)) = SPLIT.iter().find(|(fam, _)| *fam == f.0) else { return f };
    let Some(name) = names().get(id).copied() else { return f };
    static MEMO: std::sync::Mutex<Option<HashMap<String, (&'static str, &'static str, &'static str)>>> = std::sync::Mutex::new(None);
    if let Some(hit) = MEMO.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|m| m.get(id)) { return *hit; }
    let out = match mode {
        Mode::Each => match merged(id) { Some((key, shown)) => (key, shown, name), None => (leak(id.to_string()), name, "") },
        Mode::Stem => {
            let s = stem(name);
            let k = format!("{}~{}", f.0, if s.is_empty() { id.to_string() } else { s });
            let shown = groups().get(&k).copied().unwrap_or(name);
            (leak(k), shown, if f.2.is_empty() { name } else { f.2 })
        }
    };
    MEMO.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(Default::default).insert(id.to_string(), out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    /// (family, ids in the family, methods after the split) at 24 Nov 2026: update when ids are added to these families.
    const PINS: &[(&str, usize, usize)] = &[("mssta_systems", 38, 34), ("jchr2", 15, 15), ("msws_gm8", 15, 15), ("mstt_equis", 17, 16), ("khtwg", 18, 18), ("apwms_other", 24, 24), ("mbhfe_macro", 20, 20), ("kdta_other", 18, 18), ("capttm_other", 17, 17), ("cktwo", 13, 13), ("tdperl_other", 12, 10), ("apsmts_other", 11, 11), ("apoi_other", 12, 12), ("lbi12_twelve", 12, 12), ("lebcafm_daytrading", 11, 11), ("mbsot_macro", 11, 11), ("tkttl_setups", 11, 11), ("jchr1_other", 10, 10), ("hbqts_filters", 10, 10), ("lebfut_futures", 10, 10), ("jhbct_charting", 9, 9), ("tdwb_other", 9, 9), ("vsbrw_other", 9, 9), ("mslib_misc", 9, 9), ("mslib_hr", 9, 9), ("cov_tfm", 8, 8), ("lwdg2_daytrade", 8, 8), ("putg_studies", 8, 8), ("jbcdt_intraday", 8, 8), ("hsta_stats", 8, 8), ("tdnmtt_other", 8, 8), ("maetf_alloc", 8, 8), ("cbpst_acme", 8, 8), ("ramtm_models", 8, 8), ("acbff_other", 7, 7), ("mzwws_models", 7, 7), ("mpmm_interp", 7, 7), ("cgift_nt", 7, 7), ("dgnl_spi", 7, 7), ("atm01_tsl", 7, 7), ("kllien_nt", 12, 12), ("pbwts_examples", 6, 6), ("ehmd_indicators", 6, 6), ("tdnsta_other", 6, 6), ("mzwws_seasonal", 6, 6), ("wbmdd_other", 6, 6), ("jbcaf_nt", 6, 6), ("dgnl_methods", 6, 6), ("atm00_tsl", 6, 6), ("gg_nt", 9, 9), ("bstacm_nt", 6, 6), ("mmft_boucher", 5, 5), ("lwdg2_tools", 5, 5), ("mstt_wow", 147, 107), ("kets_ma", 52, 20), ("kdta_breadth", 16, 8), ("mbsot_appel", 8, 4)];
    #[test]
    fn stems_group_variants() {
        assert_eq!(stem("200 dma violated by 100%"), stem("200 dma violated by 50%"));
        assert_eq!(stem("Dual SMA crossover · next open (Test 1)"), stem("Dual SMA crossover · limit entry (Test 2)"));
        assert_ne!(stem("Gap1 System (Ton Maas)"), stem("Hammer CCI"));
    }
    /// The split is display grouping only: it never touches an id, a rule or a result. Every catalogue id still resolves a family,
    /// ids outside the split families keep exactly their module's family, ids inside get a key derived from the original family.
    #[test]
    fn split_is_grouping_only() {
        let cat = crate::books::catalogue();
        let mut seen = std::collections::HashSet::new();
        for d in &cat { assert!(seen.insert(d.id), "duplicate id {}", d.id); }
        for d in &cat {
            let Some(orig) = crate::books::family_of(d.id) else { continue };
            let now = crate::connors::family_of(d.id);
            assert_ne!(now.0, "other", "{} lost its family", d.id);
            match SPLIT.iter().find(|(f, _)| *f == orig.0) {
                None => assert_eq!(now.0, orig.0, "{} must keep its family", d.id),
                Some((f, Mode::Each)) => assert_eq!(now.0, merged(d.id).map_or(d.id, |m| m.0), "{} (family {f}) must be its own method (or its merged variant group)", d.id),
                Some((f, Mode::Stem)) => assert!(now.0.starts_with(&format!("{f}~")), "{} key {}", d.id, now.0),
            }
        }
    }
    /// Per-family method counts, so a split cannot drift silently (R4 #387).
    #[test]
    fn per_family_method_counts() {
        let cat = crate::books::catalogue();
        for (fam, ids, methods) in PINS {
            let keys: std::collections::HashSet<&str> = cat.iter().filter(|d| crate::books::family_of(d.id).map_or(false, |f| f.0 == *fam)).map(|d| crate::connors::family_of(d.id).0).collect();
            let n = cat.iter().filter(|d| crate::books::family_of(d.id).map_or(false, |f| f.0 == *fam)).count();
            assert_eq!((n, keys.len()), (*ids, *methods), "family {fam}");
        }
    }
    /// Variants that differ only by a number / parameter / version suffix are ONE method (R4 #387 audit: Regress Slope 1-4 etc.).
    #[test]
    fn numbered_variants_are_one_method() {
        for (ids, _) in MERGE {
            let keys: std::collections::HashSet<&str> = ids.iter().map(|id| crate::connors::family_of(id).0).collect();
            assert_eq!(keys.len(), 1, "{ids:?} must share one method key");
            assert_eq!(*keys.iter().next().unwrap(), ids[0]);
        }
        assert_ne!(crate::connors::family_of("mssta_ma_asimetric").0, crate::connors::family_of("mssta_ma_100").0);
    }
    /// Reverted families stay ONE method (R4 #387: pattern x filter grids, fill / stop variants, table rows).
    #[test]
    fn grids_and_variants_stay_one_method() {
        let cat = crate::books::catalogue();
        for fam in ["msws_gm3", "msws_gm2", "msws_gm4", "acbff_intraday", "lwnl_commodity", "jsats", "lwnl_seasonal", "msadd_cci", "r3r4"] {
            let keys: std::collections::HashSet<&str> = cat.iter().filter(|d| crate::books::family_of(d.id).map_or(false, |f| f.0 == fam)).map(|d| crate::connors::family_of(d.id).0).collect();
            assert!(keys.len() <= 1, "family {fam} must stay one method, found {}", keys.len());
        }
    }
}
