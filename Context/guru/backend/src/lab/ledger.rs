//! Whole-lot cash accounting for lab simulations. Raw fills remain separate from the effective-price
//! trades used by the existing book statistics; no book calculation is modified.
use super::rules::{Limits, MoneyCosts, Sizing, TaxSide};
use std::collections::BTreeMap;
use crate::connors::{Leg, Trade};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contract { pub lot: f64, pub point_value: f64, pub tick: f64, pub margin_pct: Option<f64> }
pub const CASH: Contract = Contract { lot: 1., point_value: 1., tick: 0., margin_pct: None };
pub struct Money { pub capital: f64, pub sizing: Sizing, pub costs: MoneyCosts, pub contract: Contract, pub limits: Limits }
/// The booked window and the bars that position limits read: close (previous-bar equity) and volume (max_volume_pct).
pub struct Span<'a> { pub close: &'a [f64], pub volume: Option<&'a [f64]>, pub from: usize, pub to: usize }
pub struct Fill { pub i: usize, pub px: f64, pub qty: f64, pub commission: f64, pub taxes: f64, pub slippage: f64 }
impl Fill { pub fn costs(&self) -> f64 { self.commission + self.taxes + self.slippage } }
pub struct Row {
    /// Effective-price trade: internal accounting only. `fills` contains the observed prices.
    pub real: Trade, pub fills: Trade, pub qty: f64, pub notional: f64, pub costs: f64, pub pnl: f64,
    pub entries: Vec<Fill>, pub exits: Vec<Fill>, pub borrow: f64,
    point_value: f64, borrow_pct_year: f64,
}
/// `skip_reasons` splits `skipped`; `interest` is the cumulative net interest by bar index, idle-cash interest less margin-loan
/// interest (empty without interest_pct or margin_rate_pct); `interest_earned` and `margin_interest` are the two totals.
pub struct Booked { pub rows: Vec<Row>, pub skipped: usize, pub final_equity: f64, pub costs_total: f64, pub skip_reasons: BTreeMap<&'static str, usize>, pub interest: Vec<f64>, pub interest_earned: f64, pub margin_interest: f64 }
pub const BASIS: &str = "User-declared costs; not a verified statutory schedule. Quantities are computed on the admitted price series, whose corporate-action adjustment is not established; they are not verified as-traded quantities. Sizing uses equity at the trade start; risk pauses and cooldowns use the simulator's unit returns, before money rounding and declared costs. Open quantities are marked, with no invented liquidation order. Partial exits round down to whole lots; the final exit closes the remainder.";
pub const LIMITS_BASIS: &str = "Position limits (AmiBroker settings) apply to each entry order: the quantity rounds down to round_lot; max_value (the position's total value, scale-in legs included) and max_volume_pct cap it, and a capped entry is skipped unless shrink is on; an entry below min_shares or min_value is skipped (a later scale-in leg is dropped, the position kept). max_volume_pct reads the last volume complete when the order fills, so it never looks ahead: the fill bar's volume for a fill at the close, otherwise the previous bar's (AmiBroker reads the entry bar's). A bar with zero volume is not capped (AmiBroker default). A cash shortfall skips the entry (insufficient_cash) unless shrink is on, which takes the affordable lots as without limits. prev_bar_equity sizes from the previous bar's closing equity, with an open position marked at that close. interest_pct: idle cash (equity less the entry value and entry costs of the open position) earns interest_pct / 365 per calendar day between bars, credited at each bar; it is in the equity curve and final equity, not in trade returns. margin_rate_pct (AmiBroker Margin rate): on account margin, the part of an open long position's entry value and entry costs above the closed-trade equity is a loan charged margin_rate_pct / 365 per calendar day, debited at each bar like interest_pct; shorts and futures borrow no cash, and there is no margin call.";

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
        if !self.interest.is_empty() { for i in from..=to { out[i - from] += self.interest[i] / m.capital; } }
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
/// AmiBroker "Tick size" on a cash instrument. As in AmiBroker it affects only exits by the built-in stops: a max-loss,
/// trailing or breakeven exit rounds against the trader (sells down, covers up); a profit target or partial take-profit
/// rounds toward its limit side (sells up, covers down), so it never fills through it. Entries and other exits are not
/// rounded. Levels are tested unrounded; with bars on the tick grid a stop then triggers exactly as at its adverse-rounded
/// level. An open position's mark is not a fill and is left as is.
pub fn snap_ticks(trades: Vec<Trade>, tick: f64) -> Vec<Trade> {
    let snap = |px: f64, up: bool| { let n = px / tick; if (n - n.round()).abs() < 1e-9 { px } else if up { n.ceil() * tick } else { n.floor() * tick } };
    trades.into_iter().map(|mut t| {
        let buy = t.dir > 0.;
        let stop = ["exit.stop_loss", "exit.target", "exit.trailing", "exit.breakeven"].iter().any(|k| t.reason.starts_with(k));
        let target = t.reason.starts_with("exit.target");
        let pieces = t.exits.len();
        for (n, e) in t.exits.iter_mut().enumerate() {
            let last = n + 1 == pieces;
            // pieces before the last are take-profits; the last is the closing exit
            if !last || (stop && !t.open) { e.1 = snap(e.1, !buy != (!last || target)); }
        }
        if pieces > 0 { t.exit_px = t.exits.iter().map(|e| e.1 * e.2).sum(); } else if stop && !t.open { t.exit_px = snap(t.exit_px, !buy != target); }
        t
    }).collect()
}
fn lots(units: f64, lot: f64) -> f64 { if units.is_finite() && units > 0. { (units / lot).floor() * lot } else { 0. } }

/// Idle-cash interest credited and margin-loan interest debited bar by bar (simple daily rates per calendar day, compounding at each bar).
struct Interest { rate: f64, loan_rate: f64, cum: Vec<f64>, at: usize, earned: f64, paid: f64 }
impl Interest {
    /// Bars (at, upto] at each previous bar's close: `held(bar)` is (committed, long cash) — the idle cash is equity less
    /// committed, the loan the long cash position above equity.
    fn to(&mut self, upto: usize, equity: &mut f64, dates: &[String], held: impl Fn(usize) -> (f64, f64)) {
        if self.rate <= 0. && self.loan_rate <= 0. { return; }
        for i in self.at + 1..=upto {
            let days = (super::resample::day_number(&dates[i]) - super::resample::day_number(&dates[i - 1])) as f64;
            let (committed, long) = held(i - 1);
            let add = (*equity - committed).max(0.) * self.rate * days;
            let pay = (long - *equity).max(0.) * self.loan_rate * days;
            *equity += add - pay; self.cum[i] = self.cum[i - 1] + add - pay; self.earned += add; self.paid += pay;
        }
        self.at = self.at.max(upto);
    }
    fn between(&self, a: usize, b: usize) -> f64 { if self.cum.is_empty() { 0. } else { self.cum[b] - self.cum[a] } }
}

pub fn book(trades: &[Trade], dates: &[String], m: &Money, span: &Span) -> Booked {
    let (k, c, lim) = (&m.contract, &m.costs, &m.limits);
    let lot = k.lot * lim.round_lot;
    let (mut equity, mut rows, mut skipped, mut costs_total) = (m.capital, Vec::<Row>::new(), 0, 0.);
    let mut skip_reasons = BTreeMap::new();
    let (rate, loan_rate) = (lim.interest_pct / 100. / 365., lim.margin_rate_pct / 100. / 365.);
    let mut interest = Interest { rate, loan_rate, cum: if rate > 0. || loan_rate > 0. { vec![0.; dates.len()] } else { vec![] }, at: span.from, earned: 0., paid: 0. };
    for t in trades {
        let (d, buy) = (t.dir, t.dir > 0.);
        // a futures contract's own margin, else the cash account margin (AmiBroker "Account margin"; 100 = none, × 1 exactly)
        let basis = |px: f64| px * k.point_value * k.margin_pct.map_or(lim.margin_pct / 100., |p| p / 100.);
        let e = t.legs[0].i;
        interest.to(e, &mut equity, dates, |_| (0., 0.));
        // Previous-bar equity: drop bar e's interest and, if the last trade closed on bar e, use its mark at bar e-1's close.
        let sizing_equity = if !lim.prev_bar_equity || e <= span.from { equity } else {
            let j = e - 1;
            let open = rows.last().filter(|r| r.fills.exit_i > j).map_or(0., |r| r.mark(j, span.close[j], dates, true) - r.pnl);
            equity - interest.between(j, e) + open
        };
        let (mut entries, mut used, mut paid, mut why) = (Vec::<Fill>::new(), 0., 0., "rounded_to_zero");
        for l in &t.legs {
            let wanted = match m.sizing {
                Sizing::Units(n) => l.w * n * k.lot,
                Sizing::Value(v) => l.w * v / basis(l.px),
                Sizing::Full | Sizing::PctEquity(_) | Sizing::RiskPct(_) => l.w * sizing_equity / basis(l.px),
            };
            let slips = !matches!(t.entry_kind, "limit" | "stop_limit") || !entries.is_empty();
            // Costs are affine in quantity: solve affordability once, without an unbounded one-lot decrement loop.
            let unit = fill(c, k, l.i, l.px, 1., buy, slips).costs() - c.per_order;
            let room = (equity - used - paid - c.per_order).max(0.) / (basis(l.px) + unit);
            let free = wanted.min(room);
            let held_value: f64 = entries.iter().map(|f| f.qty * f.px * k.point_value).sum();
            let by_value = lim.max_value.map_or(f64::INFINITY, |v| (v - held_value).max(0.) / (l.px * k.point_value));
            // a close fill knows its bar's volume; any earlier fill (open, high, low, average, an order) only the previous bar's
            let vol_bar = if t.entry_kind == "close" { l.i } else { l.i.saturating_sub(1) };
            // AmiBroker default "Disable trade size limit when bar volume is zero": a bar with no volume is not capped.
            let by_volume = lim.max_volume_pct.map_or(f64::INFINITY, |p| match span.volume.map_or(0., |v| v[vol_bar]) { v if v > 0. => p / 100. * v, _ => f64::INFINITY });
            let mut q = lots(free.min(by_value).min(by_volume), lot);
            let cut = q < lots(free, lot);
            // AmiBroker without shrinking: an order the cash on hand cannot pay for in full is skipped, not cut
            let poor = !lim.shrink && lots(room, lot) < lots(wanted.min(by_value).min(by_volume), lot);
            let value = q * l.px * k.point_value;
            // a cap that leaves no lot names itself, shrink or not
            let skip = if cut && (!lim.shrink || q <= 0.) { Some(if by_volume < by_value { "max_volume_pct" } else { "max_value" }) }
                else if poor { Some("insufficient_cash") } else if q <= 0. { None } else if q < lim.min_shares { Some("min_shares") } else if value < lim.min_value { Some("min_value") } else { None };
            if let Some(r) = skip { q = 0.; if entries.is_empty() { why = r; } }
            if q <= 0. { if entries.is_empty() { break; } else { continue; } }
            let f = fill(c, k, l.i, l.px, q, buy, slips);
            used += basis(l.px) * q; paid += f.costs(); entries.push(f);
        }
        if entries.is_empty() { skipped += 1; *skip_reasons.entry(why).or_insert(0) += 1; continue; }
        let qty: f64 = entries.iter().map(|l| l.qty).sum();
        let pieces = if t.exits.is_empty() { vec![(t.exit_i, t.exit_px, 1.)] } else { t.exits.clone() };
        let (mut exits, mut remaining) = (Vec::new(), qty);
        for (n, &(i, px, fraction)) in pieces.iter().enumerate() {
            let last = n + 1 == pieces.len();
            let q = if last { remaining } else { lots(qty * fraction, lot).min(remaining) };
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
        // cash on account margin: the loan is not idle cash, so the whole entry value is committed; a long cash position above equity is the loan
        let held: Vec<(usize, f64)> = row.entries.iter().map(|l| (l.i, if k.margin_pct.is_some() { basis(l.px) } else { l.px * k.point_value } * l.qty + l.costs())).collect();
        let borrows = buy && k.margin_pct.is_none();
        interest.to(t.exit_i, &mut equity, dates, |i| { let c = held.iter().filter(|h| h.0 <= i).map(|h| h.1).sum(); (c, if borrows { c } else { 0. }) });
        equity += row.pnl; costs_total += row.costs; rows.push(row);
    }
    interest.to(span.to, &mut equity, dates, |_| (0., 0.));
    Booked { rows, skipped, final_equity: equity, costs_total, skip_reasons, interest_earned: interest.earned, margin_interest: interest.paid, interest: interest.cum }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn trade(dir: f64, legs: &[(usize, f64, f64)], exit: (usize, f64), kind: &'static str) -> Trade {
        Trade { dir, legs: legs.iter().map(|&(i, px, w)| Leg { i, px, w }).collect(), exit_i: exit.0, exit_px: exit.1, reason: "rule".into(), entry_kind: kind, signal_i: exit.0, open: false, exits: vec![] }
    }
    fn dates(n: usize) -> Vec<String> { (0..n).map(|i| crate::lab::resample::civil(crate::lab::resample::day_number("2024-01-01") + i as i64)).collect() }
    fn money(sizing: Sizing, costs: MoneyCosts) -> Money { Money { capital: 10_000., sizing, costs, contract: CASH, limits: Limits::none() } }
    /// The pre-limits call shape: no limits read the span, so it only bounds the window.
    fn book(t: &[Trade], d: &[String], m: &Money) -> Booked { super::book(t, d, m, &Span { close: &[], volume: None, from: 0, to: d.len() - 1 }) }
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
        let fut = Money { capital: 1_000_000., sizing: Sizing::PctEquity(50.), costs: MoneyCosts::default(), contract: Contract { lot: 50., point_value: 1., tick: 0.05, margin_pct: Some(12.) }, limits: Limits::none() };
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

    fn limited(sizing: Sizing, limits: Limits) -> Money { Money { limits, ..money(sizing, MoneyCosts::default()) } }
    #[test] fn limits_round_lots_and_skip_small_entries_with_a_reason() {
        let t = [trade(1., &[(1, 30., 1.)], (3, 31.), "close")];
        let lot = book(&t, &dates(10), &limited(Sizing::Value(1000.), Limits { round_lot: 10., ..Limits::default() }));
        assert_eq!((lot.rows[0].qty, lot.skipped), (30., 0)); // 33 wanted, rounded down to 3 lots of 10
        for (limits, why) in [(Limits { round_lot: 10., min_shares: 31., ..Limits::default() }, "min_shares"), (Limits { round_lot: 10., min_value: 950., ..Limits::default() }, "min_value")] {
            let b = book(&t, &dates(10), &limited(Sizing::Value(1000.), limits));
            assert_eq!((b.rows.len(), b.skipped, b.final_equity), (0, 1, 10_000.));
            assert_eq!(b.skip_reasons.get(why), Some(&1));
        }
        let cash = book(&[trade(1., &[(1, 20_000., 1.)], (3, 21_000.), "close")], &dates(10), &money(Sizing::PctEquity(100.), MoneyCosts::default()));
        assert_eq!(cash.skip_reasons.get("rounded_to_zero"), Some(&1));
    }
    #[test] fn without_shrinking_a_cash_short_entry_is_skipped_as_amibroker_does() {
        let t = [trade(1., &[(1, 100., 1.)], (3, 110.), "close")];
        let costs = MoneyCosts { per_order: 20., ..MoneyCosts::default() };
        let with = |limits| Money { limits, ..money(Sizing::PctEquity(100.), costs.clone()) };
        // 100 shares wanted; after the 20 commission the cash pays for 99
        assert_eq!(book(&t, &dates(10), &money(Sizing::PctEquity(100.), costs.clone())).rows[0].qty, 99., "no limits: today's shrink");
        assert_eq!(book(&t, &dates(10), &with(Limits { shrink: true, ..Limits::default() })).rows[0].qty, 99.);
        let b = book(&t, &dates(10), &with(Limits::default()));
        assert_eq!((b.rows.len(), b.skipped, b.final_equity), (0, 1, 10_000.));
        assert_eq!(b.skip_reasons.get("insufficient_cash"), Some(&1));
        // a scale-in leg the cash cannot pay for in full is dropped, the position kept
        let legs = [trade(1., &[(1, 100., 0.5), (2, 100., 0.5)], (3, 110.), "close")];
        let b = book(&legs, &dates(10), &with(Limits::default()));
        assert_eq!((b.rows[0].qty, b.rows[0].entries.len(), b.skipped), (50., 1, 0));
    }
    #[test] fn caps_shrink_or_skip_and_volume_is_the_entry_bars() {
        let t = [trade(1., &[(1, 30., 1.)], (3, 31.), "close")];
        let cap = |shrink| Limits { max_value: Some(600.), shrink, ..Limits::default() };
        assert_eq!(book(&t, &dates(10), &limited(Sizing::Value(1000.), cap(true))).rows[0].qty, 20.);
        let skip = book(&t, &dates(10), &limited(Sizing::Value(1000.), cap(false)));
        assert_eq!((skip.rows.len(), skip.skip_reasons.get("max_value")), (0, Some(&1)));
        assert_eq!(book(&t, &dates(10), &limited(Sizing::Value(500.), cap(false))).rows[0].qty, 16.); // under the cap: untouched
        let volume = [1e9, 500., 1e9, 1e9, 1e9];
        let span = Span { close: &[], volume: Some(&volume), from: 0, to: 4 };
        let vol = |shrink| Limits { max_volume_pct: Some(10.), shrink, ..Limits::default() };
        let b = super::book(&t, &dates(5), &limited(Sizing::Units(100.), vol(true)), &span);
        assert_eq!(b.rows[0].qty, 50.); // 10% of the entry bar's 500
        let b = super::book(&t, &dates(5), &limited(Sizing::Units(100.), vol(false)), &span);
        assert_eq!((b.rows.len(), b.skip_reasons.get("max_volume_pct")), (0, Some(&1)));
        // zero volume on the bar the cap reads: no limit (AmiBroker "Disable trade size limit when bar volume is zero")
        let zero = [1e9, 0., 1e9, 1e9, 1e9];
        let span0 = Span { close: &[], volume: Some(&zero), from: 0, to: 4 };
        assert_eq!(super::book(&t, &dates(5), &limited(Sizing::Units(100.), vol(false)), &span0).rows[0].qty, 100.);
    }
    #[test] fn previous_bar_equity_marks_a_position_that_closes_on_the_entry_bar() {
        let close = [100., 100., 110., 120., 120., 120.];
        let span = Span { close: &close, volume: None, from: 0, to: 5 };
        let t = [trade(1., &[(1, 100., 1.)], (3, 120.), "close"), trade(1., &[(3, 120., 1.)], (5, 120.), "close")];
        let now = super::book(&t, &dates(6), &money(Sizing::PctEquity(100.), MoneyCosts::default()), &span);
        assert_eq!((now.rows[0].qty, now.rows[1].qty), (100., 100.)); // 12,000 after the first exit
        let prev = super::book(&t, &dates(6), &limited(Sizing::PctEquity(100.), Limits { prev_bar_equity: true, ..Limits::default() }), &span);
        assert_eq!((prev.rows[0].qty, prev.rows[1].qty), (100., 91.)); // bar 2 close: 10,000 + 100 × (110 − 100) = 11,000 → 91 at 120
    }
    #[test] fn idle_cash_earns_interest_by_calendar_day_and_reaches_the_curve() {
        let m = limited(Sizing::Units(50.), Limits { interest_pct: 36.5, ..Limits::default() }); // 0.1% a day
        let span = Span { close: &[], volume: None, from: 0, to: 4 };
        let flat = super::book(&[], &dates(5), &m, &span);
        assert!((flat.final_equity - 10_000. * 1.001f64.powi(4)).abs() < 1e-9);
        let b = super::book(&[trade(1., &[(1, 100., 1.)], (3, 100.), "close")], &dates(5), &m, &span);
        // bar 1 on 10,000; bars 2-3 on the 5,000 not in the position; bar 4 on all cash again
        let mut eq = 10_000. * 1.001; eq += (eq - 5000.) * 0.001; eq += (eq - 5000.) * 0.001; eq *= 1.001;
        assert!((b.final_equity - eq).abs() < 1e-9 && b.rows[0].pnl == 0.);
        let curve = b.equity(&[100.; 5], &dates(5), 0, 4, &m, true);
        assert!(curve[0] == 1. && (curve[4] * 10_000. - b.final_equity).abs() < 1e-9);
        let weekend = vec!["2024-01-05".to_string(), "2024-01-08".into()];
        let w = super::book(&[], &weekend, &m, &Span { close: &[], volume: None, from: 0, to: 1 });
        assert!((w.final_equity - 10_000. * 1.003).abs() < 1e-9, "Friday to Monday is three calendar days");
    }
    #[test] fn volume_cap_reads_only_volume_complete_at_the_fill() {
        let volume = [300., 500., 1e9, 1e9, 1e9];
        let span = Span { close: &[], volume: Some(&volume), from: 0, to: 4 };
        let m = limited(Sizing::Units(100.), Limits { max_volume_pct: Some(10.), shrink: true, ..Limits::default() });
        let qty = |kind| super::book(&[trade(1., &[(1, 30., 1.)], (3, 31.), kind)], &dates(5), &m, &span).rows[0].qty;
        assert_eq!((qty("close"), qty("next_open"), qty("limit")), (50., 30., 30.)); // bar 1's volume is unknown at its open
        // a cap that leaves no lot is reported as that cap, shrink or not (10 % of volume 5 = 0.5 → 0 lots)
        let tiny = [5.; 5];
        let b = super::book(&[trade(1., &[(1, 30., 1.)], (3, 31.), "close")], &dates(5), &m, &Span { volume: Some(&tiny), ..span });
        assert_eq!((b.rows.len(), b.skip_reasons.get("max_volume_pct")), (0, Some(&1)));
        // zero volume means no cap (AmiBroker "Disable trade size limit when bar volume is zero")
        let zero = [0.; 5];
        assert_eq!(super::book(&[trade(1., &[(1, 30., 1.)], (3, 31.), "close")], &dates(5), &m, &Span { volume: Some(&zero), ..span }).rows[0].qty, 100.);
    }
    #[test] fn max_value_caps_the_whole_scaled_in_position() {
        let t = [trade(1., &[(1, 30., 0.5), (2, 30., 0.5)], (3, 31.), "close")];
        let cap = |shrink| Limits { max_value: Some(600.), shrink, ..Limits::default() };
        // leg 1: 16 shares (480); leg 2 has 120 of room left: 4 shares with shrink, dropped without it
        let b = book(&t, &dates(10), &limited(Sizing::Value(1000.), cap(true)));
        assert_eq!(b.rows[0].entries.iter().map(|f| f.qty).collect::<Vec<_>>(), vec![16., 4.]);
        let b = book(&t, &dates(10), &limited(Sizing::Value(1000.), cap(false)));
        assert_eq!((b.rows[0].qty, b.rows[0].entries.len(), b.skipped, b.skip_reasons.len()), (16., 1, 0, 0));
    }
    #[test] fn a_later_leg_under_min_shares_is_dropped_and_the_position_kept() {
        let t = [trade(1., &[(1, 100., 0.7), (2, 100., 0.3)], (3, 100.), "close")];
        let b = book(&t, &dates(10), &limited(Sizing::Units(10.), Limits { min_shares: 5., ..Limits::default() }));
        assert_eq!((b.rows[0].entries.len(), b.rows[0].qty, b.skipped, b.skip_reasons.len()), (1, 7., 0, 0));
    }
    #[test] fn previous_bar_equity_marks_a_short_with_the_short_sign() {
        let close = [100., 100., 90., 80., 80., 80.];
        let span = Span { close: &close, volume: None, from: 0, to: 5 };
        let t = [trade(-1., &[(1, 100., 1.)], (3, 80.), "close"), trade(-1., &[(3, 80., 1.)], (5, 80.), "close")];
        let prev = super::book(&t, &dates(6), &limited(Sizing::PctEquity(100.), Limits { prev_bar_equity: true, ..Limits::default() }), &span);
        // bar 2 close: 10,000 + 100 × (100 − 90) = 11,000 → 137 at 80 (12,000 after the exit would give 150)
        assert_eq!((prev.rows[0].qty, prev.rows[1].qty), (100., 137.));
    }
    #[test] fn a_short_commits_its_entry_value_and_earns_interest_on_the_rest() {
        let m = limited(Sizing::Units(50.), Limits { interest_pct: 36.5, ..Limits::default() });
        let b = super::book(&[trade(-1., &[(1, 100., 1.)], (3, 100.), "close")], &dates(5), &m, &Span { close: &[], volume: None, from: 0, to: 4 });
        let mut eq = 10_000. * 1.001; eq += (eq - 5000.) * 0.001; eq += (eq - 5000.) * 0.001; eq *= 1.001;
        assert!((b.final_equity - eq).abs() < 1e-9);
    }

    #[test] fn account_margin_multiplies_buying_power_and_lends_no_idle_cash() {
        let t = [trade(1., &[(1, 100., 1.)], (3, 110.), "close")];
        let half = |interest_pct| limited(Sizing::PctEquity(100.), Limits { margin_pct: 50., shrink: true, interest_pct, ..Limits::default() });
        let b = book(&t, &dates(5), &half(0.));
        assert_eq!((b.rows[0].qty, b.rows[0].pnl), (200., 2000.)); // 10,000 × 100 / 50 buys 20,000 of stock
        assert!(b.rows[0].real.legs[0].w > 1.99, "the position is twice the equity");
        let full = book(&t, &dates(5), &limited(Sizing::PctEquity(100.), Limits { shrink: true, ..Limits::default() }));
        assert_eq!(full.rows[0].qty, 100., "margin 100: no leverage");
        // interest: bar 1 on all cash, bars 2-3 nothing (the 20,000 position exceeds equity; the loan pays no interest), bar 4 again
        let span = Span { close: &[], volume: None, from: 0, to: 4 };
        let b = super::book(&t, &dates(5), &half(36.5), &span);
        let eq = 10_000. * 1.001; let after = eq + 200. * 10.; // bar 1 interest raises the sizing equity: 10,010 × 2 / 100 = 200 shares
        assert!((b.final_equity - after * 1.001).abs() < 1e-9, "{}", b.final_equity);
    }
    #[test] fn margin_rate_charges_the_long_loan_by_calendar_day() {
        let margin = |rate| limited(Sizing::PctEquity(100.), Limits { margin_pct: 50., shrink: true, margin_rate_pct: rate, ..Limits::default() });
        let span = Span { close: &[], volume: None, from: 0, to: 4 };
        let long = [trade(1., &[(1, 100., 1.)], (3, 110.), "close")];
        let b = super::book(&long, &dates(5), &margin(36.5), &span); // 0.1% a day
        // 200 shares (20,000) on 10,000 equity: bar 2 pays 0.1% of the 10,000 loan, bar 3 of 10,010 (the charge lowered equity)
        assert_eq!(b.rows[0].qty, 200.);
        assert!((b.margin_interest - 20.01).abs() < 1e-9 && b.interest_earned == 0., "{}", b.margin_interest);
        assert!((b.final_equity - (10_000. - 20.01 + 2000.)).abs() < 1e-9, "{}", b.final_equity);
        let curve = b.equity(&[100., 100., 105., 110., 110.], &dates(5), 0, 4, &margin(36.5), true);
        assert!((curve[4] * 10_000. - b.final_equity).abs() < 1e-9 && (curve[2] * 10_000. - (10_000. + 200. * 5. - 10.)).abs() < 1e-9, "the charge reaches the curve");
        assert_eq!(super::book(&long, &dates(5), &margin(0.), &span).final_equity, 12_000., "rate 0: no charge");
        let short = super::book(&[trade(-1., &[(1, 100., 1.)], (3, 100.), "close")], &dates(5), &margin(36.5), &span);
        assert_eq!((short.margin_interest, short.final_equity), (0., 10_000.), "a short borrows no cash");
        let cash = super::book(&long, &dates(5), &limited(Sizing::PctEquity(100.), Limits { shrink: true, margin_rate_pct: 36.5, ..Limits::default() }), &span);
        assert_eq!(cash.margin_interest, 0., "no account margin: nothing is borrowed");
    }
    #[test] fn tick_size_rounds_fills_against_the_trader_and_limits_toward_their_limit() {
        let snap = |t: Trade| snap_ticks(vec![t], 0.05).remove(0);
        // AmiBroker: tick size affects only exits by the built-in stops; entries and rule exits keep their prices
        let mut stop = trade(1., &[(1, 100.03, 0.5), (2, 99.97, 0.5)], (3, 98.03), "next_open"); stop.reason = "exit.stop_loss".into();
        let s = snap(stop);
        assert!(s.legs[0].px == 100.03 && s.legs[1].px == 99.97 && (s.exit_px - 98.).abs() < 1e-9, "entries untouched; a stop sells down");
        let mut target = trade(1., &[(1, 99.97, 1.)], (3, 101.02), "limit"); target.reason = "exit.target".into();
        let s = snap(target);
        assert!(s.legs[0].px == 99.97 && (s.exit_px - 101.05).abs() < 1e-9, "a target sells at or above its level");
        let mut short = trade(-1., &[(1, 100.03, 1.)], (3, 101.02), "next_open"); short.reason = "exit.stop_loss".into();
        let s = snap(short);
        assert!(s.legs[0].px == 100.03 && (s.exit_px - 101.05).abs() < 1e-9, "a short's stop covers up");
        let s = snap(trade(1., &[(1, 100.03, 1.)], (3, 101.02), "next_open")); // a rule exit ("rule")
        assert!(s.legs[0].px == 100.03 && s.exit_px == 101.02, "a rule exit is not a stop exit");
        let on_grid = trade(1., &[(1, 100.05, 1.)], (3, 0.15 + 0.15), "close");
        assert_eq!((snap(on_grid.clone()).legs[0].px.to_bits(), snap(on_grid).exit_px.to_bits()), (100.05f64.to_bits(), (0.15f64 + 0.15).to_bits()), "on-grid prices are untouched");
        let mut parts = trade(1., &[(1, 100., 1.)], (4, 120.02), "close");
        parts.exits = vec![(2, 110.02, 0.5), (4, 120.02, 0.5)]; parts.exit_px = 115.02; parts.open = true;
        let s = snap(parts);
        assert!((s.exits[0].1 - 110.05).abs() < 1e-9 && s.exits[1].1 == 120.02, "a take-profit rounds up; the open mark is not a fill");
        assert!((s.exit_px - (s.exits[0].1 * 0.5 + 120.02 * 0.5)).abs() < 1e-12);
    }
    #[test] fn stop_limit_entries_are_limit_fills_but_later_legs_are_market_fills() {
        let t = trade(1., &[(1,100.,0.5),(2,100.,0.5)],(3,100.),"stop_limit");
        let m=money(Sizing::Units(10.),MoneyCosts { slip_bps:10., ..MoneyCosts::default() });
        let b=book(&[t],&dates(5),&m); let r=&b.rows[0];
        assert_eq!((r.entries[0].slippage,r.entries[1].slippage),(0.,0.5));
        assert!((r.costs-1.5).abs()<1e-9);
    }

}
