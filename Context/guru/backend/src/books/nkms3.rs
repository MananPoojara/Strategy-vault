//! NIKSAN MetaStock formula pack: Equis "PS RSI Trailing Exit" (MS72PRFT.DTA record index 81, research drive USB HDD 1/MetaStock/
//! @Systems & Indicators/@Misc/- Studies - No Sorted/[sm] Metastock Plugin - NIKSAN's MS Formula All/NIKSAN's MSFormula3/).
//! Kept apart from K2's books::nkms2 because this record alone depends on whether an opposite Enter closes the position.
//! MetaStock 7.2 manual (research drive USB HDD 1/MetaStock/@Knowledge/Manual  Metastock 7.2 y Downloader 7.2/MSWIN.doc, "The System
//! Editor Dialog"): "A short position can only be entered when the system is long or out"; "A long position can only be entered
//! when the system is short or out". The tester has three states and no setting for this (System Testing Options lists none),
//! so an opposite Enter reverses. p.reverse = 0 = a position closes only by its own Close formula (the earlier vault reading).
//! Book id `nkms3`. Lane K4 (titles loop, 30 Oct 2026); independent check authors/titles-1030/K4w/k4_ref.py (header).
use crate::books::msws::{self, Book, Eng, Sys, EXEC_DEFAULT};
use crate::connors::{Ctx, ParamDef, Params, StrategyDef, Trade};
use crate::connors::Status::Live;
use std::sync::OnceLock;

pub const BOOK: &str = "nkms3";
const ID: &str = "nkms3_rsi_trailing_exit";
/// Admitted ids (authors/titles-1030/K4w/k4_ref.py (header)).
pub const REVIEWED: &[&str] = &[ID];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const CONV: &str = "One position at a time; an opposite Enter reverses the position at the close (MetaStock 7.2 manual, The System Editor Dialog: a short can only be entered when the system is long or out), p.reverse = 0 turns that off; a Close formula is checked from the bar after entry; no same-direction re-entry on the exit bar; parameter side = 1/-1 runs one side alone. The record has no stops (all-zero stop block) and no fill field: MetaStock defaults, trade at the close, no delay. Indicators: MetaStock definitions as in module msws. Runs on indices, ETFs and single stocks (US and India) are a declared application: the record names no market and prints no results.";
const SYS: &[Sys] = &[Sys { id: ID, name: "PS RSI Trailing Exit", group: "NIKSAN MetaStock pack · Equis PS system tests (MS72PRFT.DTA)", family: ("nkms3_ps", "Equis PS RSI Trailing Exit (NIKSAN pack)"), page: "MS72PRFT.DTA record index 81 (0-based)", status: Live,
    f: [Some("RSI(14)>Mov(RSI(14),3,S) AND Cross(RSI(14),30)"), Some("Cross(30,RSI(14)) OR (RSI(14)<Mov(RSI(14),3,S) AND RSI(14)>70)"), Some("RSI(14)<Mov(RSI(14),3,S) AND Cross(70,RSI(14))"), Some("Cross(RSI(14),70) OR (RSI(14)>Mov(RSI(14),3,S) AND RSI(14)<30)")],
    exec: EXEC_DEFAULT, notes: "" }];
static NK: Book = Book { id: BOOK, sys: SYS, fml: &[], conv: CONV, cat: OnceLock::new(), par: OnceLock::new() };

pub fn catalogue() -> Vec<StrategyDef> { NK.catalogue() }
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { NK.family_of(id) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let mut v = NK.params(id)?;
    v.push(ParamDef { key: "reverse", label: "Opposite Enter: 1 = reverses (MetaStock 7.2 manual), 0 = ignored, only the Close formula closes", def: 1., min: 0., max: 1., step: 1. });
    Some(v)
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let s = SYS.iter().find(|s| s.id == id)?;
    if from >= to || to >= x.r.len() { return None; }
    let side = p.get("side").copied().unwrap_or(0.).round() as i32;
    let raw: Vec<&str> = s.f.iter().flatten().copied().collect();
    let map: Vec<(f64, f64)> = msws::constants(&raw).into_iter().map(|c| (c, p.get(&msws::const_key(c)).copied().unwrap_or(c))).collect();
    let fl: &dyn Fn(&str) -> Option<String> = &|_| None;
    let mut sig = msws::signals(&mut Eng::new(x, fl), s.f, &map, side).ok()?;
    if p.get("reverse").copied().unwrap_or(1.) == 0. { return Some(msws::simulate(x, from, to, &sig, s.exec)); }
    let (xl, xs) = (sig.xl.clone(), sig.xs.clone());
    for i in 0..sig.xl.len() { sig.xl[i] |= sig.es[i]; sig.xs[i] |= sig.el[i]; }
    let mut t = msws::simulate(x, from, to, &sig, s.exec);
    for q in t.iter_mut().filter(|q| !q.open && !(if q.dir > 0. { xl[q.exit_i] } else { xs[q.exit_i] })) { q.reason = "reversed by the opposite Enter · close".into(); }
    Some(t)
}
