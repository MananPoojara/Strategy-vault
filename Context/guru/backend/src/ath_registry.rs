//! Every ATH sub-variant the Rust engine runs (one list; each family lives in its own module).
#![allow(dead_code)]
use crate::ath_core::VariantDef;

pub fn all() -> Vec<VariantDef> {
    let mut v = crate::ath_pctswing::variants();
    v.extend(crate::ath_exits1::variants());
    v.extend(crate::ath_exits2::variants());
    v.extend(crate::ath_loss1::variants());
    v.extend(crate::ath_loss2::variants());
    v.extend(crate::ath_plus::variants());
    v.extend(crate::ath_rank::variants());
    v.extend(crate::ath_new_scan::variants());
    v.extend(crate::ath_new_swing::variants());
    v.extend(crate::ath_new_st::variants());
    v.extend(crate::ath_new_prob::variants());
    v.extend(crate::ath_v2_e2::variants());
    v.extend(crate::ath_v2_e1::variants());
    v.extend(crate::ath_v2_d2::variants());
    v.extend(crate::ath_v2_d1::variants());
    v.extend(crate::ath_v2_c2::variants());
    v.extend(crate::ath_v2_c1::variants());
    v.extend(crate::ath_v2_b::variants());
    v.extend(crate::ath_v2_a2::variants());
    v.extend(crate::ath_v2_a1::variants());
    v
}

pub fn find(id: &str) -> Option<VariantDef> {
    all().into_iter().find(|v| v.id == id)
}
