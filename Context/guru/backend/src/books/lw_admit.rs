//! Larry Williams admission (authors/lwilliams/admission/SOURCE-CONTRACT.md, 23 Oct 2026). Each book module keeps its own
//! REVIEWED list (and SPX_POINTS for rules whose stops/targets are S&P-futures dollars); this file only joins them.
use crate::books::{lwcmc, lwcot, lwdg2, lwdtfo, lwfc10, lwic6, lwicw, lwinb, lwlts, lwmdc, lwnl, lwpro};

fn lists(book: &str) -> Option<(&'static [&'static str], &'static [&'static str])> {
    Some(match book {
        lwlts::BOOK => (lwlts::REVIEWED, lwlts::SPX_POINTS),
        lwicw::BOOK => (lwicw::REVIEWED, lwicw::SPX_POINTS),
        lwmdc::BOOK => (lwmdc::REVIEWED, lwmdc::SPX_POINTS),
        lwpro::BOOK => (lwpro::REVIEWED, lwpro::SPX_POINTS),
        lwcot::BOOK => (lwcot::REVIEWED, lwcot::SPX_POINTS),
        lwdtfo::BOOK => (lwdtfo::REVIEWED, lwdtfo::SPX_POINTS),
        lwdg2::BOOK => (lwdg2::REVIEWED, lwdg2::SPX_POINTS),
        lwcmc::BOOK => (lwcmc::REVIEWED, lwcmc::SPX_POINTS),
        lwnl::BOOK => (lwnl::REVIEWED, lwnl::SPX_POINTS),
        lwfc10::BOOK => (lwfc10::REVIEWED, lwfc10::SPX_POINTS),
        lwic6::BOOK => (lwic6::REVIEWED, lwic6::SPX_POINTS),
        lwinb::BOOK => (lwinb::REVIEWED, lwinb::SPX_POINTS),
        _ => return None,
    })
}
const BOOKS: &[&str] = &[lwlts::BOOK, lwicw::BOOK, lwmdc::BOOK, lwpro::BOOK, lwcot::BOOK, lwdtfo::BOOK, lwdg2::BOOK, lwcmc::BOOK, lwnl::BOOK, lwfc10::BOOK, lwic6::BOOK, lwinb::BOOK];

pub fn reviewed(book: &str, id: &str) -> bool { lists(book).is_some_and(|(r, _)| r.contains(&id)) }

/// The one ticker where a rule's dollar/point amounts mean what the book meant: SPX for S&P-futures amounts, NDX for the
/// Nasdaq-100 futures stop.
pub fn only_ticker(id: &str) -> Option<&'static str> {
    if lwnl::NDX_POINTS.contains(&id) { return Some("NDX"); }
    BOOKS.iter().any(|b| lists(b).is_some_and(|(_, s)| s.contains(&id))).then_some("SPX")
}

/// US-holiday trades: US indices, ETFs and stocks only.
pub fn us_only(id: &str) -> bool { lwnl::US_ONLY.contains(&id) || lwfc10::US_ONLY.contains(&id) }

/// Every admitted Williams rule reads the open (open entries, stop fills at a gapped open, bailout exits) except the
/// close-only holiday entries, which still exit at openings; all are checked for stale index opens.
pub fn reads_open(book: &str, id: &str) -> bool { reviewed(book, id) }
