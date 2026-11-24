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
    // batch 1 (24 Nov 2026)
    ("mstt_wow", Mode::Stem), ("kets_ma", Mode::Stem), ("lwnl_seasonal", Mode::Stem), ("msws_gm2", Mode::Stem), ("msadd_candle", Mode::Stem),
    ("msws_gm4", Mode::Stem), ("kdta_breadth", Mode::Stem),
    ("mssta_systems", Mode::Each), ("msws_gm3", Mode::Each), ("mscvl_systems", Mode::Each), ("apwms_other", Mode::Each), ("lwnl_commodity", Mode::Each),
    ("mbhfe_macro", Mode::Each), ("jsats", Mode::Each), ("khtwg", Mode::Each), ("kdta_other", Mode::Each), ("acbff_intraday", Mode::Each),
    ("capttm_other", Mode::Each), ("mstt_equis", Mode::Each), ("jchr2", Mode::Each), ("msws_gm8", Mode::Each),
    // batch 2 (24 Nov 2026)
    ("mkits_gold", Mode::Stem), ("msadd_cci", Mode::Stem), ("msadd_stoch", Mode::Stem), ("r3r4", Mode::Stem), ("lwlts_oops", Mode::Stem),
    ("acbff_eitheror", Mode::Each), ("acbff_calendar", Mode::Each), ("cktwo", Mode::Each), ("msadd_misc", Mode::Each), ("tdperl_other", Mode::Each),
    ("apsmts_other", Mode::Each), ("apoi_other", Mode::Each), ("mbsot_patterns_nt", Mode::Each), ("lbi12_twelve", Mode::Each), ("atm02_tsl", Mode::Each),
    ("lebcafm_daytrading", Mode::Each), ("mbsot_macro", Mode::Each), ("masmt_cal", Mode::Each), ("tkttl_setups", Mode::Each), ("jscgf", Mode::Each), ("atmcsa", Mode::Each),
    // batch 3 (24 Nov 2026)
    ("msadd_trix", Mode::Stem),
    ("jchr1_other", Mode::Each), ("hbqts_filters", Mode::Each), ("lebfut_futures", Mode::Each), ("cwin_windows", Mode::Each), ("wdgig_other", Mode::Each),
    ("rwmts_intraday", Mode::Each), ("hbqts_entries", Mode::Each), ("acbff_extremes", Mode::Each), ("jhbct_charting", Mode::Each), ("tdwb_other", Mode::Each),
    ("vsbrw_other", Mode::Each), ("mslib_misc", Mode::Each), ("mslib_hr", Mode::Each), ("cov_tfm", Mode::Each), ("lwdg2_daytrade", Mode::Each),
    ("putg_studies", Mode::Each), ("jbcdt_intraday", Mode::Each), ("hsta_stats", Mode::Each), ("tdnmtt_other", Mode::Each),
    // batch 4 (24 Nov 2026)
    ("mbsot_appel", Mode::Stem), ("strtw_gd", Mode::Stem), ("lkqts_monthly", Mode::Stem), ("sfo_sector", Mode::Stem),
    ("maetf_alloc", Mode::Each), ("acbff_potpourri", Mode::Each), ("cbpst_acme", Mode::Each), ("svtv2_rules", Mode::Each), ("ramtm_models", Mode::Each), ("jswct_campaign", Mode::Each), ("mslib_combined", Mode::Each), ("jsgsta", Mode::Each), ("ch_ntt_ideas", Mode::Each), ("wdnbp_other", Mode::Each), ("ecqt", Mode::Each), ("lkcpt_krut", Mode::Each), ("acbff_other", Mode::Each), ("mzwws_models", Mode::Each), ("lebws", Mode::Each), ("masmt_ma", Mode::Each), ("mpmm_interp", Mode::Each), ("cgift_nt", Mode::Each), ("dgnl_spi", Mode::Each), ("atm01_tsl", Mode::Each), ("wmny_breadth", Mode::Each), ("kllien_nt", Mode::Each), ("ws5_momentum_windows", Mode::Each), ("msadd_rsi", Mode::Each),
];

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
        Mode::Each => (leak(id.to_string()), name, ""),
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
    #[test]
    fn stems_group_variants() {
        assert_eq!(stem("200 dma violated by 100%"), stem("200 dma violated by 50%"));
        assert_eq!(stem("Dual SMA crossover · next open (Test 1)"), stem("Dual SMA crossover · limit entry (Test 2)"));
        assert_ne!(stem("Gap1 System (Ton Maas)"), stem("Hammer CCI"));
    }
}
