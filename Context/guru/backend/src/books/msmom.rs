//! "MACD Momentum System for MS" (MetaStock text after Thomas Aspray, Stocks & Commodities v6:8-9). Source (research drive,
//! read in place, read-only): `USB HDD 1/MetaStock/@Systems & Indicators/MACD Momentum System for MS/MACD Momentum System for
//! MS.rar` -> `MACD Momentum System for MS.txt` (extracted as data). Book id `msmom`. Notes: library/msmom/README.md.
//! The printed MetaStock formulas are stored as text and run by the msws MetaStock formula engine (books::msws).
use crate::books::msws::{Book, Sys, EXEC_DEFAULT};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use std::sync::OnceLock;

pub const BOOK: &str = "msmom";
/// Admitted ids (authors/lanea/vendor2/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["msmom_macd_momentum"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }

const EL: &str = r##"TmpA:=Mov(CLOSE,8,E)-Mov(CLOSE,14,E); MACDH:= TmpA-Mov(TmpA,8,E); TmpB:=MACDH - Ref(MACDH,-8); MACDMO:=Mov(TmpB,3,S); (MACDH<0) AND (MACDMO<0)"##;
const XL: &str = r##"TmpA:=Mov(CLOSE,8,E)-Mov(CLOSE,14,E); MACDH:= TmpA-Mov(TmpA,8,E); TmpB:=MACDH - Ref(MACDH,-8); MACDMO:=Mov(TmpB,3,S); (MACDMO>0)"##;

static SYS: &[Sys] = &[
    Sys { id: "msmom_macd_momentum", name: "MACD Momentum System (8/14, long: MACDH < 0 and MACDMO < 0, out when MACDMO > 0)", group: "MACD Momentum (MetaStock text)",
        family: ("msmom", "MACD Momentum (Aspray, MetaStock)"), page: "txt 'MACD Momentum System'", status: Status::Live, f: [Some(EL), Some(XL), None, None], exec: EXEC_DEFAULT,
        notes: "Long only as printed (no short formulas). The text first shows the 10/20 MACD histogram/momentum indicator and then says 'Note that different periods are used in this system' (8/14, momentum 8 bars back, 3-bar SMA)." },
    Sys { id: "msmom_macd_momentum_opt", name: "MACD Momentum System w/Opt (opt1 7-10, opt2 12-20)", group: "MACD Momentum (MetaStock text)",
        family: ("msmom", "MACD Momentum (Aspray, MetaStock)"), page: "txt 'MACD Momentum System w/Opt'",
        status: Status::NotTestable("optimisation template: only the ranges 'opt1 7 to 10 step 1, opt2 12 to 20 step 1' are given, no values (the fixed 8/14 version is msmom_macd_momentum)"),
        f: [None, None, None, None], exec: EXEC_DEFAULT, notes: "Same formulas with opt1/opt2 in place of 8/14." },
];

const CONV: &str = "One position at a time; a position closes only by its own Close formula (from the bar after a close-filled entry). Indicators: MetaStock definitions as in module msws (Mov E seeded with the first value). \
India: instrument swap (NIFTY 50 instead of the stocks/futures it was written for).";
pub static MSMOM: Book = Book { id: BOOK, sys: SYS, fml: &[], conv: CONV, cat: OnceLock::new(), par: OnceLock::new() };

pub fn catalogue() -> Vec<StrategyDef> { MSMOM.catalogue() }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { MSMOM.params(id) }
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> { MSMOM.run(id, x, from, to, p) }
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { MSMOM.family_of(id) }
