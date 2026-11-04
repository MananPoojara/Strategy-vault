//! Capstone Trading Systems, "Seven Trading Systems for the S&P Futures" (e-book, (c) 2010) with the TradeStation code
//! (ts-code.txt; same code in the e-book Appendix). Book id `cpgap`. Source: research drive "USB HDD 1/TradeStation
//! Systems & Indicators/Seven Trading Systems for the S&P Futures (capstonetradingsystems.com)/", copied to
//! Books/Capstone Trading Systems/. Refs: e-book chapter.page of the per-chapter PDFs (Ch3-Ch10) and the Appendix.
//! All seven are day trades on a 1-minute @ES.D chart: entries after the first minute or inside clock windows, dollar
//! stop/target inside the day, exit on the close. Lane K38 (4 Nov 2026, decision #202): Systems 1-3 and 5-7 run on the real
//! SPX 1-minute bars (2008-2026) at the book's own E-mini points (Ch3 rule 4 "10 points", Ch9 rule 8 "40 points ... 22
//! points", Ch10 $800 / $550 at $50 a point = 16 / 11 points), SPX only (decisions #169, #196(2)); then the #177b check on
//! tradable SPY with the points scaled (authors/titles-1030/K38w/check177.out). System 4 needs three more index futures: held.
//! Independent check: authors/titles-1030/K38w/minute_ref.py.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::{day_trade, fill_px, touched, Bar1, Level};

pub const BOOK: &str = "cpgap";

const GF: &str = "needs 1-minute E-mini S&P (@ES.D day session) bars 2005-2010: entry is the close of the first 1-minute bar ('This Bar on Close' on the day's first bar), then the $500 stop (10 ES points) / $200 target (4 points) and the exit on the close are evaluated on the 1-minute path; daily data cannot give the first-minute close or which of stop/target came first. India: would need NIFTY futures 1-minute bars";
const GFR: &str = "needs 1-minute bars of E-mini S&P plus the day-session opens of E-mini Russell, E-mini Midcap and E-mini Dow (data2-data4) and the daily @ES.D for the SMA100 filter (data5), 2005-2010: entry on the first 1-minute bar's close, $500 stop / $200 target on the 1-minute path, exit on the close";
const GC: &str = "needs 1-minute E-mini S&P (@ES.D day session) bars 2005-2010: entries are stop/limit orders placed only inside a clock window (Tim+269 to Tim+669 minutes, or open to Tim+614, or Tim+269 to Tim+569) at OpenD +/- 50% of the gap or the 12/24-bar (minute) extreme, with an intraday dollar stop/target and exit on the close; none of this exists on daily bars";
const NOTE: &str = "Book test: E-mini S&P, 1-minute @ES.D on data1 and daily @ES.D on data2, 05/31/2005-05/28/2010 (Ch3, Ch4, Ch9, Ch10). Dollar stops/targets are per ES contract ($50/point). The SMA100 filter compares 'C of data2' with 'Average(C of data2,100)[1]' on the daily chart.";
const LIVE_NOTE: &str = "Book test: E-mini S&P, 1-minute @ES.D on data1 and daily @ES.D on data2, 05/31/2005-05/28/2010; the dollar stops/targets are per ES contract at $50 a point. [Declared application, lane K38, decision #202] Runs on the real SPX 1-minute bars (cash index, 2014-01 to 2026-08: rules that read the open start where the SPX open is real) at the book's E-mini points, literal (the same index, an exact unit conversion: decisions #169, #196(2)); SPX only, no other instrument has those points. [Readings] TradeStation stamps a bar with its end time: the first bar of the day is 9:31 ET, so Tim+269 = 12:00 ET, Tim+669 = 16:00, Tim+614 = 15:45, Tim+569 = 15:00. OpenD(0) / Open = the day's first minute's open; CloseD(1), HighD(1), LowD(1), Close[1] = yesterday's daily close, high and low; 'C of data2 < Average(C of data2,100)[1]' = yesterday's close against the 100-day average ending the day before. 'This Bar on Close' on the first minute fills at that minute's close; 'Next Bar at X Stop/Limit' is an order for the next minute only (a buy stop fills when the minute's high reaches X, at X or at the open if it opened above; a buy limit when its low reaches X, at X or at the open if it opened below; sells mirrored); an entry order while in a position does nothing; EntriesToday counts the day's entries; Lowest(L,24)[1] early in the day reaches into yesterday's last minutes (a continuous chart; with no minutes for yesterday those minutes place no order). SetStopLoss / SetProfitTarget are the points from the fill; SetExitonClose = the close of the day's last SPX minute (the ES session ran to 16:15 ET). [Minute-path conventions] From the minute after the fill: the first level touched, at the level or at the minute's open when it opened through it; a minute that reaches both without opening through one: the level nearer its open (ambiguous). In the fill minute: the stop counts if touched (unless the minute opened beyond it, then only if it closed beyond it), the target only if the minute closed beyond it; other fill-minute touches are ambiguous. Ambiguous trades are marked in the trade reason; their share is in the gate row (decision #175 limit 5%). Days without minute data make no trade (no daily substitute).";

macro_rules! nd {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $why:expr, $rules:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: Status::NeedsData($why),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: NOTE, claim: $claim }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    // lane K38: every system but the Rank (needs three more index futures) runs on SPX minutes
    let live = |mut d: StrategyDef| { if d.id != "cpgap_gap_fill_rank" { d.status = Status::Live; d.notes = LIVE_NOTE; } d };
    vec![
        nd!("cpgap_gap_fill_1", "Gap Fill I (open inside yesterday's range, fade the gap)", "Ch3 · Trading System 1: Gap Fill I", "Long/Short", GF,
            &["E|Short: \"If Date<>Date[1] and Open>Close[1] + 0 and Open<HighD(1) and C of data2<Average(C of data2,100)[1] Then Sell Short (\"GapFill I SE\") This Bar on Close;\" (ts-code.txt, Appendix)",
              "E|Long: \"If Date<>Date[1] and Open<Close[1] - 0 and Open>LowD(1) and C of data2>Average(C of data2,100)[1] Then Buy (\"GapFill I LE\") This Bar on Close;\"",
              "F|\"If the daily close is above the 100 day moving average then long trades are taken. If the daily close is below the 100 day moving average then short trades are taken.\"; position taken \"after the first minute of trading\" (Ch3)",
              "X|\"SetStopLoss(500); SetProfitTarget(200); SetExitonClose;\" - \"a $500 stop loss (10 points in the E-mini S&P), a $200 profit target (4 points in the E-mini S&P), or exit at the close of the trading day\" (Ch3)",
              "F|Published (Ch3 Figure 11, @ES.D 1 min 06/01/05-05/28/10): 392 trades, 75.00% profitable, avg trade $53.76, profit factor 1.60, net $21,075"],
            claim("392", "75.00%", "$53.76", "intraday", "06/01/2005-05/28/2010, E-mini S&P 1-min")),
        nd!("cpgap_gap_fill_2", "Gap Fill II (open outside yesterday's range, fade the gap)", "Ch4 · Trading System 2: Gap Fill II", "Long/Short", GF,
            &["E|Short: \"If Date<>Date[1] and Open>HighD(1) and C of data2<Average(C of data2,100)[1] Then Sell Short (\"GapFill II SE\") cntrcts contracts This Bar on Close;\" (ts-code.txt)",
              "E|Long: \"If Date<>Date[1] and Open<LowD(1) and C of data2>Average(C of data2,100)[1] Then Buy (\"GapFill II LE\") cntrcts contracts This Bar on Close;\"",
              "X|\"SetStopLoss(500); SetExitonClose;\" - \"a $500 stop loss (10 points in the E-mini S&P) or exit at the close of the trading day\" (Ch4)",
              "F|Published (Ch4 Figure 16, 06/01/05-05/28/10): 165 trades, 50.30% profitable, avg trade $80.38, profit factor 1.41, net $13,262.50; without the SMA100 rule avg trade $29 (Figure 18)"],
            claim("165", "50.30%", "$80.38", "intraday", "06/01/2005-05/28/2010, E-mini S&P 1-min")),
        nd!("cpgap_gap_fill_combo", "Gap Fill Combination (Gap Fill I + II in one system)", "Ch7 · Trading System 3: Gap Fill Combination", "Long/Short", GF,
            &["E|Scenarios One-Four exactly as Gap Fill I and Gap Fill II (ts-code.txt System 3)",
              "X|\"SetStopLoss(500); If OpenD(0)>CloseD(1) and OpenD(0)<HighD(1) Then SetProfitTarget(200); If OpenD(0)<CloseD(1) and OpenD(0)>LowD(1) Then SetProfitTarget(200); SetExitonClose;\" - the $200 target applies only to Gap Fill I entries (Ch7)",
              "F|Published (Ch7 Figure 20, 06/01/05-05/28/10): 557 trades, 67.68% profitable, avg trade $61.65, profit factor 1.51, net $34,337.50"],
            claim("557", "67.68%", "$61.65", "intraday", "06/01/2005-05/28/2010, E-mini S&P 1-min")),
        nd!("cpgap_gap_fill_rank", "Gap Fill Rank (Gap Fill I only when ES has the weakest/strongest open of four index futures)", "Ch8 · Trading System 4: Gap Fill Rank", "Long/Short", GFR,
            &["F|\"D1=(Open-Close[1])/Close[1]\" for ES and D2-D4 for data2-data4 (E-mini Russell, E-mini Midcap, E-mini Dow); \"Condition1=D1<D2 and D1<D3 and D1<D4; Condition2=D1>D2 and D1>D3 and D1>D4;\" (ts-code.txt System 4)",
              "E|Short: Gap Fill I Scenario One \"and C of data5<Average(C of data5,100)[1] and Condition1\"; Long: Scenario Two \"and C of data5>Average(C of data5,100)[1] and Condition2\", This Bar on Close",
              "X|\"SetStopLoss(500); SetProfitTarget(200); SetExitonClose;\"",
              "F|Published (Ch8 Figure 26, 06/01/05-05/28/10): 45 trades, 84.44% profitable, avg trade $123.61, profit factor 4.35, net $5,562.50"],
            claim("45", "84.44%", "$123.61", "intraday", "06/01/2005-05/28/2010, E-mini S&P 1-min")),
        nd!("cpgap_gap_continue_1", "Gap Continue I (open inside range: midpoint stop or 12-minute-low limit, noon-4pm)", "Ch9 · Trading System 5: Gap Continue I", "Long/Short", GC,
            &["E|Long: \"If Time>=Tim+269 and Time<Tim+669 and OpenD(0)>CloseD(1) + PTS and OpenD(0)<HighD(1) and EntriesToday(date)<2 and C<(OpenD(0) + .5*AbsValue(OpenD(0)-CloseD(1))) Then Buy Next Bar at OpenD(0) + .5*AbsValue(OpenD(0)-CloseD(1)) Stop;\" and, if C is above that level, \"Buy Next Bar at Lowest(L,12)[1] Limit\"; PTS = 4 (ts-code.txt System 5)",
              "E|Short mirrored: \"OpenD(0)<CloseD(1) - PTS and OpenD(0)>LowD(1)\", \"Sell Short Next Bar at OpenD(0) - .5*AbsValue(OpenD(0)-CloseD(1)) Stop\" or \"Highest(H,12)[1] Limit\"",
              "X|\"SetProfitTarget(PrfTg); SetStopLoss(StpLs); SetExitonClose;\" PrfTg 1100, StpLs 2000 - \"$2000 stop loss ... 40 points in the E-mini S&P), a $1100 profit target ... 22 points in the E-mini S&P), or exit at the close\" (Ch9 rule 8)",
              "F|Published (Ch9 Fig.32 page, E-mini S&P 05/31/05-05/28/10): 184 trades, 57.07% profitable, avg trade $133.15, profit factor 1.92, net $24,500; E-mini Russell 139 trades, 66.91%, avg $265.25"],
            claim("184", "57.07%", "$133.15", "intraday", "05/31/2005-05/28/2010, E-mini S&P 1-min")),
        nd!("cpgap_gap_continue_2", "Gap Continue II (open outside range: 24-minute extreme limit, open-3:45pm)", "Ch10 · Trading System 6: Gap Continue II", "Long/Short", GC,
            &["E|Long: \"If Time>=Tim and Time<Tim+614 and OpenD(0)>HighD(1) + PTS and EntriesToday(date)<2 and C>OpenD(0) Then Buy (\"GapContinue II LE\") cntrcts contracts Next Bar at Lowest (L,24)[1] Limit;\" PTS = 0 (ts-code.txt System 6)",
              "E|Short: \"OpenD(0)<LowD(1) - PTS and EntriesToday(date)<2 and C<OpenD(0) Then Sell Short ... Next Bar at Highest(H,24)[1] Limit\"",
              "X|\"SetProfitTarget(PrfTg); SetStopLoss(StpLs); SetExitonClose;\" PrfTg 550, StpLs 800 (Ch10)",
              "F|Published (Ch10 Figure 39, E-mini S&P 05/31/05-05/28/10): 315 trades, 59.68% profitable, avg trade $92.14, profit factor 1.73, net $29,025"],
            claim("315", "59.68%", "$92.14", "intraday", "05/31/2005-05/28/2010, E-mini S&P 1-min")),
        nd!("cpgap_gap_continue_2b", "Gap Continue II Bonus (gap held, noon-3pm CST, one entry)", "Ch10 · Trading System 7: Gap Continue II Bonus", "Long/Short", GC,
            &["E|Long: \"If Time>=Tim+269 and Time<Tim+569 and OpenD(0)>HighD(1) + PTS and EntriesToday(date)<1 and C>OpenD(0) and LowD(0)>HighD(1) Then Buy (\"GapContinue IIB LE\") cntrcts contracts Next Bar at Lowest (L,24)[1] Limit;\" (ts-code.txt System 7)",
              "E|Short: \"OpenD(0)<LowD(1) - PTS and EntriesToday(date)<1 and C<OpenD(0) and HighD(0)<LowD(1) Then Sell Short ... Next Bar at Highest(H,24)[1] Limit\"",
              "X|\"SetProfitTarget(PrfTg); SetStopLoss(StpLs); SetExitonClose;\" PrfTg 550, StpLs 800",
              "F|Published (Ch10 Figure 40, E-mini S&P 05/31/05-05/28/10): 122 trades, 62.30% profitable, avg trade $110.25, profit factor 2.27, net $13,450"],
            claim("122", "62.30%", "$110.25", "intraday", "05/31/2005-05/28/2010, E-mini S&P 1-min")),
    ].into_iter().map(live).collect()
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "cpgap_gap_fill_1" => ("cpgap_gap_fill", "Capstone Gap Fill", "I (inside range, $200 target)"),
        "cpgap_gap_fill_2" => ("cpgap_gap_fill", "Capstone Gap Fill", "II (outside range, no target)"),
        "cpgap_gap_fill_combo" => ("cpgap_gap_fill", "Capstone Gap Fill", "Combination"),
        "cpgap_gap_fill_rank" => ("cpgap_gap_fill", "Capstone Gap Fill", "Rank (4 index futures)"),
        "cpgap_gap_continue_1" => ("cpgap_gap_continue", "Capstone Gap Continue", "I (inside range)"),
        "cpgap_gap_continue_2" => ("cpgap_gap_continue", "Capstone Gap Continue", "II (outside range)"),
        "cpgap_gap_continue_2b" => ("cpgap_gap_continue", "Capstone Gap Continue", "II Bonus (gap held)"),
        _ => return None,
    })
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> { (id.starts_with("cpgap_") && id != "cpgap_gap_fill_rank").then(Vec::new) }

/// Exit walk from minute `from` (stop, then target if any), at the level or the minute's open when it opened through it; two in
/// one minute = the level nearer its open (intraday_w2b::first_exit, ambiguous). None of them: the day's last close.
fn walk(b: &[Bar1], from: usize, lv: &[Level], amb: bool) -> (f64, &'static str, bool, usize) {
    match crate::intraday_w2b::first_exit(b, from, lv) {
        Some((k, bar, px, a)) => (px, if k == 0 { "stop" } else { "target" }, amb || a, bar),
        None => (b[b.len() - 1].c, "exit on close", amb, b.len() - 1),
    }
}

/// Stop and target of a position filled at `f` inside minute `kf` (fill-minute convention in the module note), then the walk.
fn exits(b: &[Bar1], kf: usize, d: f64, f: f64, stop: f64, tgt: f64) -> (f64, &'static str, bool, usize) {
    let lv = [Level { up: d < 0., px: f - d * stop }, Level { up: d > 0., px: f + d * tgt }];
    let z = &b[kf];
    let beyond = |v: f64, l: Level| if l.up { v >= l.px } else { v <= l.px };
    if touched(z, lv[0]) && (!beyond(z.o, lv[0]) || beyond(z.c, lv[0])) { return (lv[0].px, "stop", true, kf); }
    if touched(z, lv[1]) && beyond(z.c, lv[1]) { return (lv[1].px, "target", false, kf); }
    walk(b, kf + 1, &lv, touched(z, lv[0]) || touched(z, lv[1]))
}

fn label(why: &str, amb: bool) -> String { if amb { format!("{why} · ambiguous minute") } else { why.to_string() } }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let mut out = vec![];
    if id.starts_with("cpgap_gap_fill") {
        let (t_fill1, t_fill2) = (id != "cpgap_gap_fill_2", id != "cpgap_gap_fill_1");
        for i in from.max(101)..=to {
            let Some(b) = crate::intraday_w2b::day(x, i) else { continue };
            if b.is_empty() { continue; }
            let sma = x.c[i - 101..i - 1].iter().sum::<f64>() / 100.; // Average(C of data2,100)[1]
            let (pc, ph, pl, o, c0) = (x.c[i - 1], x.h[i - 1], x.l[i - 1], b[0].o, b[0].c);
            let (dn, up) = (pc < sma, pc > sma);
            let (s1, s2, s3, s4) = (o > pc && o < ph && dn, o < pc && o > pl && up, o > ph && dn, o < pl && up);
            let (d, tgt, why) = if t_fill1 && (s1 || s2) { (if s1 { -1. } else { 1. }, true, "Gap Fill I") }
                else if t_fill2 && (s3 || s4) { (if s3 { -1. } else { 1. }, false, "Gap Fill II") } else { continue };
            let lv = [Level { up: d < 0., px: c0 - d * 10. }, Level { up: d > 0., px: c0 + d * 4. }];
            let (xp, r, amb, _) = walk(&b, 1, &lv[..if tgt { 2 } else { 1 }], false);
            out.push(day_trade(d, i, c0, i, xp, &label(&format!("{why}: {r}"), amb), "close", false));
        }
        return Some(out);
    }
    // Gap Continue I / II / II Bonus: (signal minutes from, to), entries a day, look-back, PTS, target, stop
    let (lo, hi, cap, n, pts, tg, st) = match id {
        "cpgap_gap_continue_1" => (719u16, 959u16, 2usize, 12usize, 4., 22., 40.),
        "cpgap_gap_continue_2" => (570, 944, 2, 24, 0., 11., 16.),
        _ => (719, 899, 1, 24, 0., 11., 16.),
    };
    let sys5 = id == "cpgap_gap_continue_1";
    for i in from.max(1)..=to {
        let Some(b) = crate::intraday_w2b::day(x, i) else { continue };
        if b.is_empty() { continue; }
        let prev = crate::intraday_w2b::day(x, i - 1).unwrap_or_default();
        let off = prev.len();
        let all: Vec<Bar1> = prev.into_iter().chain(b.iter().copied()).collect();
        let (o0, pc, ph, pl) = (b[0].o, x.c[i - 1], x.h[i - 1], x.l[i - 1]);
        let (up, dn) = if sys5 { (o0 > pc + pts && o0 < ph, o0 < pc - pts && o0 > pl) } else { (o0 > ph + pts, o0 < pl - pts) };
        if !(up || dn) { continue; }
        let d = if up { 1. } else { -1. };
        let (mut entries, mut busy, mut lowd, mut highd) = (0usize, 0usize, f64::INFINITY, f64::NEG_INFINITY);
        for k in 0..b.len() - 1 {
            let z = b[k];
            lowd = lowd.min(z.l); highd = highd.max(z.h);
            if k < busy || entries >= cap || !(lo..hi).contains(&z.m) { continue; }
            let g = off + k;
            // Lowest(L,n)[1] / Highest(H,n)[1]: the n minutes before this one
            let ext = || if d > 0. { all[g - n..g].iter().map(|y| y.l).fold(f64::INFINITY, f64::min) } else { all[g - n..g].iter().map(|y| y.h).fold(f64::NEG_INFINITY, f64::max) };
            let (lvl, stop_order) = if sys5 {
                let mid = o0 + d * 0.5 * (o0 - pc).abs();
                if (z.c - mid) * d < 0. { (mid, true) } else if (z.c - mid) * d > 0. && g >= n { (ext(), false) } else { continue }
            } else {
                let ok = (z.c - o0) * d > 0. && (id != "cpgap_gap_continue_2b" || if d > 0. { lowd > ph } else { highd < pl });
                if !ok || g < n { continue; }
                (ext(), false)
            };
            let lv = Level { up: (d > 0.) == stop_order, px: lvl };
            let z1 = b[k + 1];
            if !touched(&z1, lv) { continue; }
            let fill = fill_px(&z1, lv);
            entries += 1;
            let (xp, r, amb, ke) = exits(&b, k + 1, d, fill, st, tg);
            out.push(day_trade(d, i, fill, i, xp, &label(r, amb), if stop_order { "stop" } else { "limit" }, false));
            busy = ke;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogue_owned_rank_held() {
        let c = catalogue();
        assert_eq!(c.len(), 7);
        for s in &c {
            assert!(s.id.starts_with("cpgap_") && s.book == BOOK && family_of(s.id).is_some());
            assert_eq!(matches!(s.status, Status::NeedsData(_)), s.id == "cpgap_gap_fill_rank", "{}", s.id);
        }
    }
    fn b(m: u16, o: f64, h: f64, l: f64, c: f64) -> Bar1 { Bar1 { m, o, h, l, c } }
    #[test]
    fn fill_minute_and_later_minute_conventions() {
        // long filled at 100 in minute 0: stop 84 / target 111 (Gap Continue II points)
        let w = [b(600, 101., 101., 99., 100.), b(601, 100., 112., 99., 111.5)];
        assert_eq!(exits(&w, 0, 1., 100., 16., 11.), (111., "target", false, 1));
        // fill minute reaches the target but closes under it: not taken there (ambiguous), taken in the next minute
        let w = [b(600, 101., 112., 99., 105.), b(601, 105., 111.5, 104., 110.)];
        assert_eq!(exits(&w, 0, 1., 100., 16., 11.), (111., "target", true, 1));
        // fill minute touches the stop: filled then stopped
        let w = [b(600, 101., 101., 83., 90.)];
        assert_eq!(exits(&w, 0, 1., 100., 16., 11.), (84., "stop", true, 0));
        // later minute reaches both without opening through one: the level nearer the open (stop 84 is 2 away, target 111 is 25)
        let w = [b(600, 100., 100., 100., 100.), b(601, 86., 112., 83., 100.)];
        assert_eq!(exits(&w, 0, 1., 100., 16., 11.), (84., "stop", true, 1));
        // nothing reached: the last close
        let w = [b(600, 100., 100., 100., 100.), b(959, 101., 102., 99., 101.5)];
        assert_eq!(exits(&w, 0, 1., 100., 16., 11.), (101.5, "exit on close", false, 1));
    }
}
