//! Joe Krutsinger systems whose TradeStation code was found in RAR archives on the research drive (rar-strategy-code.tsv
//! rows A0592, A0596, A0599, A0600, A0615, A1446; read with the read-only `library/rarsys/tools/larc.py`, data only) and in
//! the vendor sheets beside them. They were in the vault as NotTestable "rules not disclosed" (jkbk_* / jkzap_*). This
//! module holds the code-exact versions; the old ids are left untouched for the integrator. Book id `upg18`.
//! Notes: library/upg18/README.md. Engine: books::rrsim (TradeStation daily-bar order engine; not jkbwts::sim).
//! `T:` = research/USB HDD 1/TradeStation Systems & Indicators.
use crate::books::rrsim::{g, o, pd, run_ts, sdef, A, P, POINT};
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "upg18";
/// Admitted ids (authors/krutsinger/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["upg18_english_channel", "upg18_english_channel_rar", "upg18_prince", "upg18_parrot", "upg18_safer2005", "upg18_trend_walker"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let mut v = vec![
        sdef(BOOK, "upg18_english_channel", "English Channel (vendor sheet code, Length 8)", "Krutsinger English Channel", "Long/Short", Live, &[
            "F|\"Input: Length(8),trd(1); Condition1=Range<Average(Range, Length*2);\" (EnglishChannel.pdf)",
            "E|\"IF MarketPosition<>1 and TradesToday(date)<trd and Condition1 and c >= c[2] then Buy Next Bar at Highest(High,Length)+1 point Stop;\"",
            "E|\"IF MarketPosition<>-1 and TradesToday(date)<trd and Condition1 and c < c[2] then Sell Short Next Bar at Lowest(Low,Length)-1 point Stop;\"",
            "X|No exit orders: a position closes only when the opposite stop entry reverses it",
            "F|TradesToday(date) = entries filled on the bar just closed (daily bars: after an entry day no orders go out for the next day); 'trd leave at 1' (joe_krut.pdf p.23)",
        ], "Source: T:/Joe Krutsinger - English Channel System (joekrut.com)/From other source/EnglishChannel.pdf (vendor code + checklists). \
'An Overnight or day trading system for most markets: 60 min bars or Daily Bars' (joe_krut.pdf p.23) -> run on daily bars. The sheet's first test is 'with Stop set at off'; \
the optional stop loss / percent trail are left to the user ('as you wish') and have no values, so none is applied. 1 point = 0.01 (PriceScale 100). India: instrument swap; index shorts hypothetical."),
        sdef(BOOK, "upg18_english_channel_rar", "English Channel (English_Channel.rar code, Length 4)", "Krutsinger English Channel", "Long/Short", Live, &[
            "F|\"Input: Length(4); Condition1=Range<Average(Range, Length*2);\" (English_Channel.txt)",
            "E|\"IF MarketPosition<>1 and TradesToday(date)<1 and Condition1 then Buy Next Bar at Highest(High,Length)+1 point Stop;\"",
            "E|\"IF MarketPosition<>-1 and TradesToday(date)<1 and Condition1 then Sell Short Next Bar at Lowest(Low,Length)-1 point Stop;\"",
            "X|No exit orders: a position closes only when the opposite stop entry reverses it",
        ], "Source: T:/Joe Krutsinger - English Channel System (joekrut.com)/From other source/English_Channel.rar -> English_Channel.txt (A1446). \
A second version of the code: Length 4 and no close-vs-close[2] filter, so both stops go out every qualifying day. Daily bars as for the sheet version."),
        sdef(BOOK, "upg18_prince", "Prince 2002 (code)", "Krutsinger Prince", "Long/Short", Live, &[
            "E|\"If MarketPosition<>1 and C< C[2] then Buy (\\\"LE1\\\")next bar at c[2] stop;\"",
            "E|\"If MarketPosition<>-1 and C>= C[2] then Sell Short (\\\"SE1\\\") next bar at c[2] stop;\"",
            "X|\"If MarketPosition=1 and o of tomorrow > Entryprice then Sell ( \\\"prolx\\\" )next bar at market;\"",
            "X|\"If MarketPosition=-1 and o of tomorrow < Entryprice then Buy to Cover ( \\\"prosx\\\" )next bar at market;\"",
        ], "Source: T:/Joe Krutsinger - Prince 2002 (joekrut.com)/Open Code.rar -> Open Code.txt (A0596); the JoeKrut Diff / Switcher indicators in the same file are \
equity-curve plots, not orders. 'Exit on FIRST profitable open', 'no stop', 'trades overnight' (ZAP slides 28-29). 'o of tomorrow' = the next bar's open: the exit fills at \
that open when it is beyond the entry price (no other use of future data). Daily bars (QQQ daily slide 31; prince_summary.xls 'ND1600-Daily'). India: instrument swap."),
        sdef(BOOK, "upg18_traders_dream", "Trader's Dream (code)", "Krutsinger Trader's Dream", "Long/Short", Live, &[
            "E|\"IF C < C[2] Then Buy Next Bar H Stop;\"",
            "E|\"IF C >= C[2] Then Sell Short Next Bar L Stop;\"",
            "X|\"Inputs:STP(300); SetStopLoss(STP); SetStopContract;\" = $300 per contract: 1.2 points of the @SP contract ($250 per point) the system was sold and reported on (ZAP slide 20)",
            "X|\"SetExitOnClose;\" (exit at the close of the entry day; 'no trades overnight', slide 17)",
            "X|[intraday wave 2b] Default p.path=1: real SPX 1-minute bars decide the entry fill (at yesterday's high/low, or the minute's open when it opened through it) and whether the 1.2-point stop from the fill was reached after it. Fill minute that also reaches the stop: filled then stopped, unless it opened beyond the stop, then stopped only if it closed beyond it [convention]. SPX only (the book's fixed $300 = 1.2 @SP points; no conversion is invented); from 2014, where SPX daily opens are real. p.path=0 = the old daily bar path [Not from the book].",
        ], "Source: T:/Joe Krutsinger - Traders Dream for TS (joekrut.com)/Open Code.rar -> Open Code.txt (A0600). Daily bars (@SP daily 5/9/2001-5/10/2002, slide 20). \
The only conversion: $300 / $250 per @SP point = 1.2 index points (param stop_pts; the big S&P contract was $500/point before Nov 1997, which would be 0.6). \
The stop works from the actual fill (a gap through H or L fills at the open, stop = fill -/+ 1.2), on the entry bar too, path O -> nearer extreme -> other extreme -> C. \
India: 1.2 NIFTY points is kept as the literal instrument swap (it is tiny for NIFTY)."),
        sdef(BOOK, "upg18_parrot", "Parrot (code, ParabolicSAR stop and reverse)", "Krutsinger Parrot", "Long/Short", Live, &[
            "F|\"inputs: AfStep( 0.01); Value1 = ParabolicSAR( AfStep, .1, oParCl, oParOp, oPosition, oTransition ) ;\" (Parrot.txt)",
            "E|\"if oPosition = -1 then Buy ( \\\"ParLE1\\\" ) next bar at oParOp stop ;\"",
            "E|\"if oPosition = 1 then Sell Short ( \\\"ParSE1\\\" ) next bar at oParOp stop ;\"",
            "X|No exit orders: always in, reversed at the parabolic stop",
        ], "Source: T:/Joe Krutsinger - Parrot Trading System (joekrut.com)/Parrot.rar -> Parrot.txt (A0615). '60-minute or daily bars' (joe_krut.pdf Parrot sheet) -> daily. \
ParabolicSAR = TradeStation's built-in series function (body not in the archive): first bar long, SAR = Low, oParOp = SAR + AF*(HH-SAR) capped at Low; then HH/LL running \
extremes, reversal when Low <= oParOp (long) / High >= oParOp (short) with SAR = prior extreme, AF reset; AF += AfStep (max AfLimit) on a new extreme; oParOp capped by \
this and the previous bar's Low (long) / High (short). Reconstructed, cross-checked against the TradeBlazer port of the same function (sy.cxh99.com/2014/05/29/20973.shtml). \
The SAR runs from the first bar of the data. GearShift.pdf prints a Parrot BBE variant with AfStep 0.0122 (upg18_parrot_bbe)."),
        sdef(BOOK, "upg18_parrot_bbe", "Parrot BBE (move-to-break-even version)", "Krutsinger Parrot", "Long/Short",
            NotTestable("the break-even exit triggers on maxcontractprofit > MovetoBE(100), a dollar profit per contract of an unstated contract (60-min RU2000 / any index), so it has no size in index points"), &[
            "F|\"Inputs: AfStep( 0.0122 ); Value1 = ParabolicSAR( AfStep, .1, oParCl, oParOp, oPosition, oTransition );\" ParLE1 / ParSE1 as Parrot (GearShift.pdf p.6)",
            "X|\"Inputs:MovetoBE(100),PT(24); If marketposition = 1 then begin If maxcontractprofit > MovetoBE then Sell(\\\"joekrut belx\\\") Next Bar entryprice + pt points stop;\" (mirror for shorts)",
        ], "Source: T:/Joe Krutsinger - GearShift 2004 for TS2000i (joekrut.com)/GearShift.pdf p.6-7. The operation sheet lists 'Move to BE 500, PT 24' (joe_krut.pdf)."),
        sdef(BOOK, "upg18_gearshift", "GearShift 2004 (code)", "Krutsinger GearShift", "Long/Short",
            NeedsData("60-minute bars of the index (GearShift-Stock Indexes sheet: '60 min bars'; 'Turning Point comes into play each hour'); the daily-bar option is only on the Forex sheet (GBPUSD, no forex data); also SetStopLoss(Amount 500) is $ per contract of an unstated contract"), &[
            "F|\"Condition1 = RATEOFCHANGE(CLOSE, LenRate) > 0; Condition2 = RATEOFCHANGE(CLOSE, LenRate) < 0; Condition3 = TradesToday(DATE) <= 1;\" LenRate 8, LenAvg 10, LenBol1 9, LenBol2 3, LenBol3 2",
            "E|\"IF Condition2 AND Condition3 AND LOW <= AVERAGE(LOW, LenAvg) THEN BUY (\\\"TPLE\\\") TOMORROW BOLLINGERBAND(LOW, LenBol1, LenBol3) STOP\"",
            "E|\"IF Condition1 AND Condition3 AND HIGH >= AVERAGE(HIGH, LenAvg) THEN SELLSHORT (\\\"TPSE\\\") TOMORROW BOLLINGERBAND(HIGH, LenBol1, -LenBol3) STOP\"",
            "E|\"IF Condition3 THEN BUY (\\\"BOLE\\\") TOMORROW BOLLINGERBAND(HIGH, LenBol1, LenBol2) STOP; IF Condition3 THEN SELLSHORT (\\\"BOSE\\\") TOMORROW BOLLINGERBAND(LOW, LenBol1, -LenBol2) STOP\"",
            "X|\"Use $500 as a stop loss\": Inputs PositionBasis(False), Amount(500); SetStopContract; SetStopLoss(Amount) (GearShift.pdf p.1-2)",
        ], "Source: T:/Joe Krutsinger - GearShift 2004 for TS2000i (joekrut.com)/Open Code.rar -> Open Code.txt (A0592) and GearShift.pdf p.1-2; sheets joe_krut.pdf (GearShift-Stock Indexes, GearShift-Forex)."),
        sdef(BOOK, "upg18_dealers_choice_pro", "Dealer's Choice Pro (code)", "Krutsinger Dealer's Choice Pro", "Long/Short",
            NeedsData("7-minute bars ('Dealer Choice Pro - 7min time frame'), session times (LastEntry 1200, Sess1EndTime-100), and $ amounts per contract (StpAmt 2300 SetStopLoss and the maxpositionprofit trail) of an unstated contract"), &[
            "E|\"IF Time < LastEntry and Time > Sess1StartTime and TradesToday(date) < NumTrds and Time < (Sess1EndTime-100) then begin IF c < c[2] and MarketPosition <> 1 Then Buy (\\\"LEDCP\\\") at open of tomorrow + (Range*0.01*BkOutPct) Stop; IF c >= c[2] and MarketPosition <>-1 Then Sell Short(\\\"SEDCP\\\") at open of tomorrow - (Range*0.01*BkOutPct) Stop; end;\" Numtrds 4, BkOutPct 10, LastEntry 1200",
            "X|\"SetStopContract; SetStopLoss(StpAmt); SetExitOnClose;\" StpAmt 2300",
            "X|\"XLongVal = entryprice + (maxpositionprofit - StpAmt)/bigpointvalue; If marketposition = 1 and C <= XLongVal then Sell (\\\"XTS lx\\\") Next Bar market;\" (mirror for shorts)",
        ], "Source: T:/Joe Krutsinger - Dealers Choice Pro System for TS (joekrut.com)/Open Code.rar -> Open Code.txt (A0599)."),
    ];
    v.push(sdef(BOOK, "upg18_safer2005", "Safer 2005 (vendor sheet code)", "Krutsinger Safer 2005", "Long/Short", Status::Live, &[
        "E|\"If C < c[2] and date>date[1] then Buy (\\\"BuyL1\\\") next bar at c+ JKBuyer stop;\" - checklist: buy next bar at today's close plus today's RANGE times 38.2% on a buy stop (Safer2005.pdf)",
        "E|\"If C > c[2] and date>date[1] then Sell Short(\\\"SellS1\\\") next bar at c- JKSeller stop;\" - checklist: today's close minus today's RANGE times 61.8% on a sell short stop",
        "X|\"If MarketPosition = 1 then sell(\\\"LXBail21\\\") this bar at close;\" (mirror for shorts): 'Exit all trades on close of entry day'",
    ], "Source: research Trading Knowledge/.../Joe Krutsinger - Safer 2005 Trading System (joekrut.com)/From other source/Safer2005.pdf (one-page vendor sheet: code + checklists; scan, OCR 25 Oct 2026, authors/krutsinger). \
JKBuyer / JKSeller = today's range x 38.2% / 61.8% (the checklist). Equal closes: no order. Entry and exit on the same daily bar: one stop then the close, so the daily bar decides it without intraday data. Daily bars; sold for stock indexes (JK_Systems Intro). India: instrument swap."));
    v.push(sdef(BOOK, "upg18_trend_walker", "Trend Walker (vendor sheet code)", "Krutsinger Trend Walker", "Long/Short", Status::Live, &[
        "F|\"Input: BuyBo(2),SellBo(7), Walk(6),Trn(10);\" (TrendWalker.pdf)",
        "E|\"IF C <= C[2] and C > Average(c,trn)[1] Then Buy (\\\"LE1\\\") Next Bar at C + (Range * 0.10*BuyBo) Stop;\"",
        "E|\"IF C > C[2] and C < Average(c,trn)[1] Then Sell Short(\\\"SE1\\\") Next Bar C - (Range * 0.10*SellBo) Stop;\"",
        "X|\"IF MarketPosition=1 and BarsSinceEntry >=Walk Then sell(\\\"LX\\\") Next Bar at Market;\" (mirror for shorts): 'Exit all trades on the open after being in the trade 6 days'",
    ], "Source: research .../Joe Krutsinger - Trend Walker System for TS (joekrut.com)/TrendWalker.pdf (one-page vendor sheet: code + checklists; scan, OCR 25 Oct 2026, authors/krutsinger). \
The OCR reads the input as 'Trm(10)' and the code as 'trn'; the checklist says 'the average of the last 10 days'. Average(c,trn)[1] = yesterday's 10-day average. A reversal stop fills while the other side is held (TradeStation stop-and-reverse). Daily bars; India: instrument swap."));
    v[3].claim = claim("90", "45.56%", "$486.67", "same day", "@SP daily 5/9/2001-5/10/2002 (ZAP slide 20)");
    v[2].claim = claim("63", "77.78%", "$443.81", "", "QQQ daily 5/2001-5/2002, 1,000 shares (ZAP slide 31)");
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "upg18_english_channel" => ("upg18_english_channel", "Krutsinger English Channel (code)", "vendor sheet code"),
        "upg18_english_channel_rar" => ("upg18_english_channel", "Krutsinger English Channel (code)", "RAR code"),
        "upg18_parrot" => ("upg18_parrot", "Krutsinger Parrot (code)", "Parrot.rar code"),
        "upg18_parrot_bbe" => ("upg18_parrot", "Krutsinger Parrot (code)", "BBE (not testable)"),
        "upg18_prince" => ("upg18_prince", "Krutsinger Prince (code)", "code"),
        "upg18_traders_dream" => ("upg18_traders_dream", "Krutsinger Trader's Dream (code)", "code"),
        "upg18_safer2005" => ("upg18_safer2005", "Krutsinger Safer 2005 (code)", "code"),
        "upg18_trend_walker" => ("upg18_trend_walker", "Krutsinger Trend Walker (code)", "code"),
        "upg18_gearshift" => ("upg18_gearshift", "Krutsinger GearShift (code)", "code"),
        "upg18_dealers_choice_pro" => ("upg18_dealers_choice_pro", "Krutsinger Dealer's Choice Pro (code)", "code"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    Some(match id {
        "upg18_english_channel" => vec![pd("length", "Length (channel bars; range average over 2x)", 8., 2., 9., 1.)],
        "upg18_english_channel_rar" => vec![pd("length", "Length (channel bars; range average over 2x)", 4., 2., 9., 1.)],
        "upg18_prince" => vec![pd("lag", "Close compared with the close N bars ago", 2., 1., 10., 1.)],
        "upg18_traders_dream" => vec![pd("stop_pts", "Stop loss in points ($300 / $250 per @SP point)", 1.2, 0.2, 20., 0.2), pd("lag", "Close compared with the close N bars ago", 2., 1., 10., 1.), pd("path", "1 = real 1-minute bars (default; SPX only; days without minute data make no trade), 0 = [Not from the book] daily bar path", 1., 0., 1., 1.) /* intraday wave 2b */],
        "upg18_safer2005" => vec![pd("buy_pct", "JKBuyer: % of today's range above the close", 38.2, 5., 100., 0.1), pd("sell_pct", "JKSeller: % of today's range below the close", 61.8, 5., 100., 0.1)],
        "upg18_trend_walker" => vec![pd("buybo", "BuyBo (tenths of the range)", 2., 0., 20., 1.), pd("sellbo", "SellBo (tenths of the range)", 7., 0., 20., 1.), pd("walk", "Walk: exit after N bars in the trade", 6., 1., 30., 1.), pd("trn", "Trn: average length", 10., 2., 100., 1.)],
        "upg18_parrot" => vec![pd("afstep", "AfStep (acceleration increment)", 0.01, 0.0005, 0.05, 0.0005), pd("aflimit", "AfLimit (maximum acceleration)", 0.1, 0.05, 0.5, 0.01)],
        _ => return None,
    })
}

/// TradeStation ParabolicSAR(AfStep, AfLimit): per bar (oParOp = stop for the next bar, oPosition).
pub fn parabolic_sar(h: &[f64], l: &[f64], step: f64, limit: f64) -> (Vec<f64>, Vec<i8>) {
    let n = h.len();
    let (mut op, mut ps) = (vec![f64::NAN; n], vec![0i8; n]);
    if n == 0 { return (op, ps); }
    let (mut pos, mut af, mut hh, mut ll) = (1i8, step, h[0], l[0]);
    let mut par_op = (ll + af * (hh - ll)).min(l[0]);
    op[0] = par_op; ps[0] = pos;
    for i in 1..n {
        let (hh1, ll1) = (hh, ll);
        hh = hh.max(h[i]); ll = ll.min(l[i]);
        if pos == 1 {
            if l[i] <= par_op {
                pos = -1; let cl = hh; hh = h[i]; ll = l[i]; af = step;
                par_op = (cl + af * (ll - cl)).max(h[i]).max(h[i - 1]);
            } else {
                let cl = par_op;
                if hh > hh1 && af < limit { af = (af + step).min(limit); }
                par_op = (cl + af * (hh - cl)).min(l[i]).min(l[i - 1]);
            }
        } else if h[i] >= par_op {
            pos = 1; let cl = ll; hh = h[i]; ll = l[i]; af = step;
            par_op = (cl + af * (hh - cl)).min(l[i]).min(l[i - 1]);
        } else {
            let cl = par_op;
            if ll < ll1 && af < limit { af = (af + step).min(limit); }
            par_op = (cl + af * (ll - cl)).max(h[i]).max(h[i - 1]);
        }
        op[i] = par_op; ps[i] = pos;
    }
    (op, ps)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("upg18_") || from >= to || to >= x.r.len() { return None; }
    let (h, l, c, op) = (&x.h[..], &x.l[..], &x.c[..], &x.o[..]);
    match id {
        "upg18_english_channel" | "upg18_english_channel_rar" => {
            let sheet = id == "upg18_english_channel";
            let n = g(p, "length", if sheet { 8. } else { 4. }).max(1.) as usize;
            let rng: Vec<f64> = (0..c.len()).map(|i| h[i] - l[i]).collect();
            let avg = crate::books::rrsim::sma(&rng, 2 * n);
            Some(run_ts(x, from, to, |i, st, _now, next| {
                if !(rng[i] < avg[i]) || i + 1 < n { return; } // NaN average -> false
                let traded_today = st.pos != 0 && st.entry_i == i; // no exits: an entry on bar i leaves its position open
                if traded_today { return; }
                let (up, dn) = if sheet { (i >= 2 && c[i] >= c[i - 2], i >= 2 && c[i] < c[i - 2]) } else { (true, true) };
                if st.pos != 1 && up { next.push(o(A::Buy, P::Stop(crate::books::rrsim::highest(h, n, i) + POINT), "LE")); }
                if st.pos != -1 && dn { next.push(o(A::Sell, P::Stop(crate::books::rrsim::lowest(l, n, i) - POINT), "SE")); }
            }))
        }
        "upg18_prince" => {
            let k = g(p, "lag", 2.).max(1.) as usize;
            Some(run_ts(x, from, to, |i, st, _now, next| {
                if i < k || i + 1 >= c.len() { return; }
                if st.pos != 1 && c[i] < c[i - k] { next.push(o(A::Buy, P::Stop(c[i - k]), "LE1")); }
                if st.pos != -1 && c[i] >= c[i - k] { next.push(o(A::Sell, P::Stop(c[i - k]), "SE1")); }
                // "o of tomorrow": decided with the next bar's open, filled at that open
                if st.pos == 1 && op[i + 1] > st.entry_px { next.push(o(A::Xl, P::Mkt, "prolx")); }
                if st.pos == -1 && op[i + 1] < st.entry_px { next.push(o(A::Xs, P::Mkt, "prosx")); }
            }))
        }
        // ---- intraday wave 2b block: real SPX 1-minute bars (path = 1, default)
        "upg18_traders_dream" if g(p, "path", 1.) >= 0.5 => {
            use crate::intraday::{day_trade, stop_entry_day, Level};
            let (stp, k) = (g(p, "stop_pts", 1.2), g(p, "lag", 2.).max(1.) as usize);
            let (h, l) = (&x.h, &x.l);
            let mut out = vec![];
            for j in from + 1..=to {
                let i = j - 1;
                if i < k { continue; }
                let d = if c[i] < c[i - k] { 1.0 } else { -1.0 };
                let Some(b) = crate::intraday_w2b::day(x, j) else { continue };
                let lv = if d > 0.0 { h[i] } else { l[i] };
                let Some((_, px, st)) = stop_entry_day(&b, 0, &[Level { up: d > 0.0, px: lv }], &|_, f| Some(Level { up: d < 0.0, px: f - d * stp })) else { continue };
                out.push(match st { Some(xp) => day_trade(d, j, px, j, xp, "stop loss", "stop", false), None => day_trade(d, j, px, j, c[j], "exit on close", "stop", false) });
            }
            Some(out)
        }
        // ---- end intraday wave 2b block ----
        "upg18_traders_dream" => {
            let (stp, k) = (g(p, "stop_pts", 1.2), g(p, "lag", 2.).max(1.) as usize);
            Some(run_ts(x, from, to, |i, _st, now, next| {
                now.push(o(A::Xl, P::Mkt, "exit on close")); now.push(o(A::Xs, P::Mkt, "exit on close"));
                if i < k || i + 1 >= c.len() { return; }
                // SetStopLoss works from the actual entry price: a stop entry the open gaps through fills at the open
                if c[i] < c[i - k] { let f = op[i + 1].max(h[i]); next.push(o(A::Buy, P::Stop(h[i]), "LE")); next.push(o(A::Xl, P::Stop(f - stp), "stop loss")); }
                else { let f = op[i + 1].min(l[i]); next.push(o(A::Sell, P::Stop(l[i]), "SE")); next.push(o(A::Xs, P::Stop(f + stp), "stop loss")); }
            }))
        }
        "upg18_safer2005" => {
            let (bp, sp) = (g(p, "buy_pct", 38.2) / 100., g(p, "sell_pct", 61.8) / 100.);
            Some(run_ts(x, from, to, |i, st, now, next| {
                now.push(o(A::Xl, P::Mkt, "LXBail21")); now.push(o(A::Xs, P::Mkt, "SXBail21"));
                if i < 2 || st.pos != 0 { return; } // "no trades yet today": a position held into the close is closed there
                let r = h[i] - l[i];
                if c[i] < c[i - 2] { next.push(o(A::Buy, P::Stop(c[i] + r * bp), "BuyL1")); }
                if c[i] > c[i - 2] { next.push(o(A::Sell, P::Stop(c[i] - r * sp), "SellS1")); }
            }))
        }
        "upg18_trend_walker" => {
            let (bb, sb, walk, n) = (g(p, "buybo", 2.), g(p, "sellbo", 7.), g(p, "walk", 6.).max(1.) as usize, g(p, "trn", 10.).max(1.) as usize);
            let ma = crate::books::rrsim::sma(c, n);
            Some(run_ts(x, from, to, |i, st, _now, next| {
                if i < 2 { return; }
                let r = h[i] - l[i];
                if c[i] <= c[i - 2] && c[i] > ma[i - 1] { next.push(o(A::Buy, P::Stop(c[i] + r * 0.10 * bb), "LE1")); }
                if st.pos == 1 && st.bse(i) >= walk { next.push(o(A::Xl, P::Mkt, "LX")); }
                if c[i] > c[i - 2] && c[i] < ma[i - 1] { next.push(o(A::Sell, P::Stop(c[i] - r * 0.10 * sb), "SE1")); }
                if st.pos == -1 && st.bse(i) >= walk { next.push(o(A::Xs, P::Mkt, "SX")); }
            }))
        }
        "upg18_parrot" => {
            let (sar, ps) = parabolic_sar(h, l, g(p, "afstep", 0.01), g(p, "aflimit", 0.1));
            Some(run_ts(x, from, to, |i, _st, _now, next| {
                if ps[i] == -1 { next.push(o(A::Buy, P::Stop(sar[i]), "ParLE1")); }
                if ps[i] == 1 { next.push(o(A::Sell, P::Stop(sar[i]), "ParSE1")); }
            }))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sar_reverses_on_low_through_stop() {
        // bar0 long, SAR = low 9 capped; bar1 makes a new high; bar2 low 8 < SAR -> short, SAR = highest high 13
        let h = [10., 13., 11.]; let l = [9., 10., 8.];
        let (op, ps) = parabolic_sar(&h, &l, 0.02, 0.2);
        assert_eq!(ps, vec![1, 1, -1]);
        assert!((op[0] - 9.0).abs() < 1e-12); // 9 + .02*(10-9) = 9.02 capped at Low 9
        assert!((op[1] - 9.0).abs() < 1e-12); // 9 + .04*(13-9) = 9.16 capped at L[1] = 9
        assert!((op[2] - 13.0).abs() < 1e-12); // 13 + .02*(8-13) = 12.9 raised to H[1] = 13
    }
}
