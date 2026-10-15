//! Whole-lot cash accounting for lab simulations. Raw fills remain separate from the effective-price
//! trades used by the existing book statistics; no book calculation is modified.
use super::rules::{MoneyCosts, Sizing, TaxSide};
use crate::connors::{Leg, Trade};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contract { pub lot: f64, pub point_value: f64, pub tick: f64, pub margin_pct: Option<f64> }
pub const CASH: Contract = Contract { lot: 1., point_value: 1., tick: 0., margin_pct: None };
pub struct Money { pub capital: f64, pub sizing: Sizing, pub costs: MoneyCosts, pub contract: Contract }
pub struct Fill { pub i: usize, pub px: f64, pub qty: f64, pub commission: f64, pub taxes: f64, pub slippage: f64 }
impl Fill { pub fn costs(&self) -> f64 { self.commission + self.taxes + self.slippage } }
pub struct Row {
    /// Effective-price trade: internal accounting only. `fills` contains the observed prices.
    pub real: Trade, pub fills: Trade, pub qty: f64, pub notional: f64, pub costs: f64, pub pnl: f64,
    pub entries: Vec<Fill>, pub exits: Vec<Fill>, pub borrow: f64,
    point_value: f64, borrow_pct_year: f64,
}
pub struct Booked { pub rows: Vec<Row>, pub skipped: usize, pub final_equity: f64, pub costs_total: f64 }
pub const BASIS: &str = "User-declared costs; not a verified statutory schedule. Quantities are computed on the admitted price series, whose corporate-action adjustment is not established; they are not verified as-traded quantities. Sizing uses equity at the trade start; risk pauses and cooldowns use the simulator's unit returns, before money rounding and declared costs. Open quantities are marked, with no invented liquidation order. Partial exits round down to whole lots; the final exit closes the remainder.";

impl Booked {
    pub fn trades(&self) -> Vec<Trade> { self.rows.iter().map(|r| r.real.clone()).collect() }
    pub fn fills(&self) -> Vec<Trade> { self.rows.iter().map(|r| r.fills.clone()).collect() }
    /// Daily money equity uses actual paid costs and accrued borrow, including on still-open positions.
    pub fn equity(&self, close: &[f64], dates: &[String], from: usize, to: usize, m: &Money, net: bool) -> Vec<f64> {
        let (mut out, mut capital, mut next) = (vec![1.; to - from + 1], m.capital, from);
        for r in &self.rows {
            let start = r.fills.legs[0].i;
            for i in next..start { out[i - from] = capital / m.capital; }
            for i in start..=r.fills.exit_i { out[i - from] = (capital + r.mark(i, close[i], dates, net)) / m.capital; }
            capital += r.mark(r.fills.exit_i, close[r.fills.exit_i], dates, net);
            next = r.fills.exit_i + 1;
        }
        for i in next..=to { out[i - from] = capital / m.capital; }
        out
    }
}
impl Row {
    fn borrow_at(&self, i: usize, dates: &[String]) -> f64 {
        if self.fills.dir > 0. { return 0.; }
        self.entries.iter().filter(|l| l.i <= i).map(|l| {
            let mut left = 1.; let mut days = 0.;
            for e in self.exits.iter().filter(|e| e.i <= i) {
                let f = e.qty / self.qty;
                days += f * (super::resample::day_number(&dates[e.i]) - super::resample::day_number(&dates[l.i])) as f64;
                left -= f;
            }
            days += left.max(0.) * (super::resample::day_number(&dates[i]) - super::resample::day_number(&dates[l.i])) as f64;
            self.borrow_pct_year / 100. * l.px * l.qty * self.point_value * days / 365.
        }).sum()
    }
    fn mark(&self, i: usize, close: f64, dates: &[String], net: bool) -> f64 {
        let exits: Vec<_> = self.exits.iter().filter(|e| e.i <= i).collect();
        let sold: f64 = exits.iter().map(|e| e.qty).sum();
        let mark = (exits.iter().map(|e| e.px * e.qty).sum::<f64>() + (self.qty - sold) * close) / self.qty;
        let gross: f64 = self.entries.iter().filter(|l| l.i <= i).map(|l| l.qty * self.point_value * self.fills.dir * (mark - l.px)).sum();
        if !net { return gross; }
        gross - self.entries.iter().filter(|l| l.i <= i).map(Fill::costs).sum::<f64>() - exits.iter().map(|e| e.costs()).sum::<f64>() - self.borrow_at(i, dates)
    }
}
pub(super) fn fill(c: &MoneyCosts, k: &Contract, i: usize, px: f64, qty: f64, buy: bool, slips: bool) -> Fill {
    let value = px * qty * k.point_value;
    let tax: f64 = c.taxes.iter().filter(|t| matches!((t.2, buy), (TaxSide::Both, _) | (TaxSide::Buy, true) | (TaxSide::Sell, false))).map(|t| t.1 / 100. * value).sum();
    Fill { i, px, qty, commission: c.per_order + c.per_unit * qty + c.pct / 100. * value,
        taxes: tax, slippage: if slips { c.slip_bps / 1e4 * value + c.slip_ticks * k.tick * qty * k.point_value } else { 0. } }
}
fn lots(units: f64, lot: f64) -> f64 { if units.is_finite() && units > 0. { (units / lot).floor() * lot } else { 0. } }

pub fn book(trades: &[Trade], dates: &[String], m: &Money) -> Booked {
    let (k, c) = (&m.contract, &m.costs);
    let (mut equity, mut rows, mut skipped, mut costs_total) = (m.capital, Vec::new(), 0, 0.);
    for t in trades {
        let (d, buy) = (t.dir, t.dir > 0.);
        let basis = |px: f64| px * k.point_value * k.margin_pct.map_or(1., |p| p / 100.);
        let (mut entries, mut used, mut paid) = (Vec::<Fill>::new(), 0., 0.);
        for l in &t.legs {
            let wanted = match m.sizing {
                Sizing::Units(n) => l.w * n * k.lot,
                Sizing::Value(v) => l.w * v / basis(l.px),
                Sizing::Full | Sizing::PctEquity(_) | Sizing::RiskPct(_) => l.w * equity / basis(l.px),
            };
            let slips = !matches!(t.entry_kind, "limit" | "stop_limit") || !entries.is_empty();
            // Costs are affine in quantity: solve affordability once, without an unbounded one-lot decrement loop.
            let unit = fill(c, k, l.i, l.px, 1., buy, slips).costs() - c.per_order;
            let room = (equity - used - paid - c.per_order).max(0.) / (basis(l.px) + unit);
            let q = lots(wanted.min(room), k.lot);
            if q <= 0. { if entries.is_empty() { break; } else { continue; } }
            let f = fill(c, k, l.i, l.px, q, buy, slips);
            used += basis(l.px) * q; paid += f.costs(); entries.push(f);
        }
        if entries.is_empty() { skipped += 1; continue; }
        let qty: f64 = entries.iter().map(|l| l.qty).sum();
        let pieces = if t.exits.is_empty() { vec![(t.exit_i, t.exit_px, 1.)] } else { t.exits.clone() };
        let (mut exits, mut remaining) = (Vec::new(), qty);
        for (n, &(i, px, fraction)) in pieces.iter().enumerate() {
            let last = n + 1 == pieces.len();
            let q = if last { remaining } else { lots(qty * fraction, k.lot).min(remaining) };
            if q <= 0. || (last && t.open) { continue; }
            let limit = !last || t.reason.starts_with("exit.target");
            exits.push(fill(c, k, i, px, q, !buy, !limit)); remaining -= q;
        }
        let notional = entries.iter().map(|l| l.px * l.qty * k.point_value).sum();
        let mut raw_pieces: Vec<_> = exits.iter().map(|e| (e.i, e.px, e.qty / qty)).collect();
        if remaining > 0. { raw_pieces.push((t.exit_i, pieces.last().unwrap().1, remaining / qty)); }
        let raw_px = raw_pieces.iter().map(|e| e.1 * e.2).sum();
        let fills = Trade { legs: entries.iter().map(|l| Leg { i: l.i, px: l.px, w: l.qty * k.point_value * l.px / equity }).collect(),
            exit_px: raw_px, exits: if t.exits.is_empty() { vec![] } else { raw_pieces }, ..t.clone() };
        let mut row = Row { real: fills.clone(), fills, qty, notional, costs: 0., pnl: 0., entries, exits, borrow: 0., point_value: k.point_value, borrow_pct_year: c.borrow_pct_year.unwrap_or(0.) };
        row.borrow = row.borrow_at(t.exit_i, dates);
        row.costs = paid + row.exits.iter().map(Fill::costs).sum::<f64>() + row.borrow;
        let mut eff_pieces: Vec<_> = row.exits.iter().map(|e| (e.i, e.px - d * e.costs() / (e.qty * k.point_value), e.qty / qty)).collect();
        if remaining > 0. { eff_pieces.push((t.exit_i, pieces.last().unwrap().1, remaining / qty)); }
        // Borrow is accrued in the daily ledger; its final total is folded into the internal return representation.
        let (_, px, f) = eff_pieces.last_mut().unwrap();
        *px -= d * row.borrow / (qty * k.point_value * *f);
        row.real = Trade { legs: row.entries.iter().map(|l| {
            let eff = l.px + d * l.costs() / (l.qty * k.point_value);
            Leg { i: l.i, px: eff, w: l.qty * k.point_value * eff / equity }
        }).collect(), exit_px: eff_pieces.iter().map(|e| e.1 * e.2).sum(), exits: if t.exits.is_empty() { vec![] } else { eff_pieces }, ..t.clone() };
        row.pnl = row.mark(t.exit_i, pieces.last().unwrap().1, dates, true);
        equity += row.pnl; costs_total += row.costs; rows.push(row);
    }
    Booked { rows, skipped, final_equity: equity, costs_total }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn trade(dir: f64, legs: &[(usize, f64, f64)], exit: (usize, f64), kind: &'static str) -> Trade {
        Trade { dir, legs: legs.iter().map(|&(i, px, w)| Leg { i, px, w }).collect(), exit_i: exit.0, exit_px: exit.1, reason: "rule".into(), entry_kind: kind, signal_i: exit.0, open: false, exits: vec![] }
    }
    fn dates(n: usize) -> Vec<String> { (0..n).map(|i| crate::lab::resample::civil(crate::lab::resample::day_number("2024-01-01") + i as i64)).collect() }
    fn money(sizing: Sizing, costs: MoneyCosts) -> Money { Money { capital: 10_000., sizing, costs, contract: CASH } }
    #[test] fn whole_shares_and_costs_reach_the_book_return_exactly() {
        let costs = MoneyCosts { per_order: 20., pct: 0.1, taxes: vec![("STT".into(), 0.1, TaxSide::Both)], ..MoneyCosts::default() };
        let b = book(&[trade(1., &[(1, 99.5, 1.)], (5, 110.), "next_open")], &dates(10), &money(Sizing::PctEquity(100.), costs));
        let r = &b.rows[0];
        assert_eq!(r.qty, 100.); // 100.5 wanted; 10,000 covers 100 shares at 99.5 plus 0.2% costs and the 20 order fee
        let buy = 20. + 0.002 * 99.5 * 100.;
        let sell = 20. + 0.002 * 110. * 100.;
        assert!((r.costs - buy - sell).abs() < 1e-9);
        assert!((r.pnl - (100. * (110. - 99.5) - buy - sell)).abs() < 1e-9);
        assert!((r.real.ret(0.) * 10_000. - r.pnl).abs() < 1e-6, "the effective-price trade carries the money P&L");
        assert!((b.final_equity - 10_000. - r.pnl).abs() < 1e-9 && b.costs_total == r.costs);
    }
    #[test] fn units_value_shorts_borrow_and_skips() {
        let units = book(&[trade(1., &[(1, 100., 1.)], (3, 101.), "close")], &dates(10), &money(Sizing::Units(7.), MoneyCosts::default()));
        assert_eq!((units.rows[0].qty, units.rows[0].pnl), (7., 7.));
        let value = book(&[trade(1., &[(1, 30., 1.)], (3, 31.), "close")], &dates(10), &money(Sizing::Value(1000.), MoneyCosts::default()));
        assert_eq!(value.rows[0].qty, 33.);
        // a short pays borrow on the entry notional per calendar day: 100 × 50 × 10% × 9/365
        let short = book(&[trade(-1., &[(1, 50., 1.)], (10, 50.), "close")], &dates(12), &money(Sizing::Units(100.), MoneyCosts { borrow_pct_year: Some(10.), ..MoneyCosts::default() }));
        assert!((short.rows[0].pnl + 5000. * 0.1 * 9. / 365.).abs() < 1e-9);
        let skip = book(&[trade(1., &[(1, 20_000., 1.)], (3, 21_000.), "close")], &dates(10), &money(Sizing::PctEquity(100.), MoneyCosts::default()));
        assert_eq!((skip.rows.len(), skip.skipped, skip.final_equity), (0, 1, 10_000.));
    }
    #[test] fn limit_fills_pay_no_slippage_and_futures_size_on_margin() {
        let slip = MoneyCosts { slip_bps: 10., ..MoneyCosts::default() };
        let market = book(&[trade(1., &[(1, 100., 1.)], (3, 100.), "next_open")], &dates(10), &money(Sizing::Units(10.), slip.clone()));
        let limit = book(&[trade(1., &[(1, 100., 1.)], (3, 100.), "limit")], &dates(10), &money(Sizing::Units(10.), slip));
        assert!((market.rows[0].costs - 2. * 0.001 * 1000.).abs() < 1e-9 && (limit.rows[0].costs - 0.001 * 1000.).abs() < 1e-9);
        // a lot of 50 units at 20,000 with 12% margin needs 120,000 a lot: 1,000,000 × 50% buys 4 lots (200 units)
        let fut = Money { capital: 1_000_000., sizing: Sizing::PctEquity(50.), costs: MoneyCosts::default(), contract: Contract { lot: 50., point_value: 1., tick: 0.05, margin_pct: Some(12.) } };
        let b = book(&[trade(1., &[(1, 20_000., 0.5)], (3, 20_100.), "next_open")], &dates(10), &fut);
        assert_eq!((b.rows[0].qty, b.rows[0].pnl), (200., 20_000.));
        assert!(b.rows[0].real.legs[0].w > 1., "futures notional exceeds equity (margin leverage)");
    }
    #[test] fn entry_cash_includes_per_unit_fees_and_only_the_entry_tax_side() {
        let m = money(Sizing::Full, MoneyCosts { per_unit: 20., taxes: vec![("sell".into(), 10., TaxSide::Sell)], ..MoneyCosts::default() });
        let b = book(&[trade(1., &[(1, 100., 1.)], (3, 110.), "close")], &dates(5), &m);
        assert_eq!(b.rows[0].qty, 83.); // 83*(100+20)=9960, no sell tax until exit
    }
    #[test] fn exits_round_to_whole_units_and_open_marks_are_not_orders() {
        let mut t = trade(1., &[(1, 100., 1.)], (4, 120.), "close");
        t.exits = vec![(2, 110., 0.5), (4, 120., 0.5)]; t.exit_px = 115.;
        let b = book(&[t.clone()], &dates(5), &money(Sizing::Units(3.), MoneyCosts { per_order: 1., ..MoneyCosts::default() }));
        assert!((b.rows[0].pnl - 47.).abs() < 1e-9); // 1*10 + 2*20 - three order fees
        t.open = true;
        let b = book(&[t], &dates(5), &money(Sizing::Units(3.), MoneyCosts { per_order: 1., ..MoneyCosts::default() }));
        assert!((b.rows[0].pnl - 48.).abs() < 1e-9); // two paid orders; remaining two shares marked, not sold
    }
    #[test] fn scale_in_shorts_borrow_only_for_each_legs_actual_days() {
        let t = trade(-1., &[(1, 100., 0.5),(3, 100., 0.5)], (5, 100.), "close");
        let b = book(&[t], &dates(6), &money(Sizing::Units(10.), MoneyCosts { borrow_pct_year: Some(36.5), ..MoneyCosts::default() }));
        assert!((b.rows[0].pnl + 3.).abs() < 1e-9); // 5*100*.365*(4+2)/365
    }

    #[test] fn money_curve_accrues_borrow_and_never_charges_a_future_exit() {
        let mut t = trade(-1., &[(0, 100., 1.)], (4, 100.), "close"); t.open = true;
        let m = money(Sizing::Units(10.), MoneyCosts { per_order: 1., borrow_pct_year: Some(36.5), ..MoneyCosts::default() });
        let dates = dates(5); let b = book(&[t.clone()], &dates, &m);
        let eq = b.equity(&[100.;5], &dates,0,4,&m,true);
        for (actual, want) in eq.iter().zip([0.9999,0.9998,0.9997,0.9996,0.9995]) { assert!((actual-want).abs()<1e-10); }
        assert_eq!(b.rows[0].exits.len(),0);
        assert!((b.rows[0].real.ret(0.) * 10000. + 5.).abs() < 1e-8);
        assert_eq!(b.equity(&[100.;5], &dates,0,4,&m,false),vec![1.;5]);
        t.open = false;
        let closed = book(&[t],&dates,&m);
        let closed_eq = closed.equity(&[100.;5],&dates,0,4,&m,true);
        assert_eq!(&eq[..4], &closed_eq[..4]);
        assert!((closed_eq[4]-0.9994).abs()<1e-10);
    }
    #[test] fn rounded_partial_exit_curve_and_effective_return_reconcile() {
        let mut t = trade(1., &[(1, 100., 1.)], (4, 120.), "close");
        t.exits = vec![(2,110.,0.5),(4,120.,0.5)]; t.exit_px = 115.;
        let m = money(Sizing::Units(3.), MoneyCosts { per_order: 1., ..MoneyCosts::default() });
        let dates = dates(5); let b = book(&[t], &dates, &m); let r=&b.rows[0];
        assert_eq!((r.exits[0].qty,r.exits[1].qty),(1.,2.));
        assert_eq!(r.fills.legs[0].px,100.);
        assert!((r.real.ret(0.)*m.capital-47.).abs()<1e-8);
        let eq=b.equity(&[100.,100.,110.,115.,120.],&dates,0,4,&m,true);
        assert!((eq[2]*m.capital-10028.).abs()<1e-8);
        assert!((eq[4]*m.capital-b.final_equity).abs()<1e-8);
    }

    #[test] fn stop_limit_entries_are_limit_fills_but_later_legs_are_market_fills() {
        let t = trade(1., &[(1,100.,0.5),(2,100.,0.5)],(3,100.),"stop_limit");
        let m=money(Sizing::Units(10.),MoneyCosts { slip_bps:10., ..MoneyCosts::default() });
        let b=book(&[t],&dates(5),&m); let r=&b.rows[0];
        assert_eq!((r.entries[0].slippage,r.entries[1].slippage),(0.,0.5));
        assert!((r.costs-1.5).abs()<1e-9);
    }

}
