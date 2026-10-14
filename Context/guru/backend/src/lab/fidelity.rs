//! Template fidelity (Customize spec §6): the unmodified template's trades next to the book engine's trades on the
//! same instrument and window. Display only — never a verification, never stored. Book code is only called.
use crate::connors::Trade;

pub const FIDELITY_LABEL: &str = "Template fidelity · display only · not a verification";
pub const FIDELITY_NOTE: &str = "Compares the template's simulated trades with the book engine's simulated trades on the same data, window and default parameters, before costs. It does not verify the book method, its published figures or either engine.";
/// At most this many differences are listed (the counts cover all of them).
pub const MAX_DIFFS: usize = 50;

fn same_px(a: f64, b: f64) -> bool { (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.) }
/// Same direction, one leg each, same entry bar and price, same exit bar and price, same open flag.
pub fn same(a: &Trade, b: &Trade) -> bool {
    a.dir == b.dir && a.legs.len() == 1 && b.legs.len() == 1 && a.legs[0].i == b.legs[0].i && same_px(a.legs[0].px, b.legs[0].px)
        && a.exit_i == b.exit_i && same_px(a.exit_px, b.exit_px) && a.open == b.open
}
/// Two empty lists compare nothing: that is not a reproduction.
pub fn reproduces(book: usize, lab: usize, matched: usize) -> bool { book > 0 && matched == book && matched == lab }
pub fn note(book: usize, lab: usize) -> String {
    if book == 0 && lab == 0 { format!("{FIDELITY_NOTE} No trades in this window to compare.") } else { FIDELITY_NOTE.into() }
}
pub struct Comparison<'a> { pub matched: usize, pub diffs: Vec<(Option<&'a Trade>, Option<&'a Trade>)> }
/// Pairs trades by entry bar (both lists are chronological and non-overlapping).
pub fn compare<'a>(book: &'a [Trade], lab: &'a [Trade]) -> Comparison<'a> {
    let (mut i, mut j, mut matched, mut diffs) = (0, 0, 0, Vec::new());
    while i < book.len() || j < lab.len() {
        let (b, l) = (book.get(i), lab.get(j));
        match (b.map(|t| t.legs[0].i), l.map(|t| t.legs[0].i)) {
            (Some(x), Some(y)) if x == y => {
                if same(b.unwrap(), l.unwrap()) { matched += 1 } else { diffs.push((b, l)) }
                i += 1; j += 1;
            }
            (Some(x), Some(y)) if x < y => { diffs.push((b, None)); i += 1; }
            (Some(_), None) => { diffs.push((b, None)); i += 1; }
            _ => { diffs.push((None, l)); j += 1; }
        }
    }
    Comparison { matched, diffs }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connors::Leg;
    fn t(e: usize, x: usize, px: f64) -> Trade { Trade { dir: 1., legs: vec![Leg { i: e, px: 100., w: 1. }], exit_i: x, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: x, open: false, exits: vec![] } }
    #[test] fn empty_against_empty_does_not_reproduce() {
        assert!(!reproduces(0, 0, 0) && reproduces(2, 2, 2) && !reproduces(2, 3, 2));
        assert!(note(0, 0).contains("No trades in this window to compare") && !note(1, 1).contains("No trades"));
    }
    #[test] fn pairs_by_entry_bar_and_names_each_difference() {
        let book = [t(1, 3, 110.), t(5, 7, 105.), t(9, 10, 99.)];
        let lab = [t(1, 3, 110. + 1e-12), t(5, 8, 105.), t(12, 14, 101.)];
        let c = compare(&book, &lab);
        assert_eq!(c.matched, 1);
        let kinds: Vec<(Option<usize>, Option<usize>)> = c.diffs.iter().map(|(b, l)| (b.map(|t| t.legs[0].i), l.map(|t| t.legs[0].i))).collect();
        assert_eq!(kinds, vec![(Some(5), Some(5)), (Some(9), None), (None, Some(12))]);
        let mut short = t(1, 3, 110.);
        short.dir = -1.;
        assert!(!same(&short, &book[0]));
        let mut two = t(1, 3, 110.);
        two.legs.push(Leg { i: 2, px: 101., w: 0.5 });
        assert!(!same(&two, &book[0]), "a scale-in trade never counts as reproduced");
    }
}
