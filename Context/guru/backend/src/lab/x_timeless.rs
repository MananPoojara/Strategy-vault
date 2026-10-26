//! Timeless (price-only) chart types as bar-aligned lab series (2026-10-24 timeless-charts design). Renko lives in x_atlas.rs. Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str],
             definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: "Price functions", params, outputs, placement: Placement::Pane, needs: Needs::NONE, definition, first, sparse: false, compute: Some(f), note: "" }
}
const LINE_BREAK: &[ParamSpec] = &[int("lines", "Lines to break", 3., 2., 10.), int("rule", "Reversal level in a short run (0 AmiBroker, 1 Sierra Chart)", 0., 0., 1.)];

const PNF: &[ParamSpec] = &[int("mode", "Box size (0 ATR, 1 fixed points, 2 % of close)", 0., 0., 2.), len("length", "ATR length", 14.),
    real("box", "Box size (points, mode 1)", 1., 0.0001, 1e9, 0.0001), real("percent", "Box size (% of the bar's close, mode 2)", 1., 0.001, 100., 0.001),
    int("reversal", "Reversal (boxes)", 3., 1., 5.), int("source", "Price (0 close, 1 AmiBroker / StockCharts high/low)", 0., 0., 1.),
    int("step_back", "Reversal from a one-box column (0 new column, 1 TradingView One Step Back, reversal 1 only)", 0., 0., 1.)];
const KAGI: &[ParamSpec] = &[int("mode", "Reversal amount (0 ATR, 1 fixed points, 2 % of the line end)", 0., 0., 2.), len("length", "ATR length", 14.),
    real("amount", "Reversal (points, mode 1)", 1., 0.0001, 1e9, 0.0001), real("percent", "Reversal (% of the line end, mode 2)", 4., 0.001, 100., 0.001)];
const INTRADAY: Needs = Needs { intraday: true, ..Needs::NONE };
// listed only, like listed.rs: these bar types need the price path inside a day
const fn intraday(id: &'static str, name: &'static str, definition: &'static str) -> IndicatorDef {
    IndicatorDef { id, name, category: "Intraday & order flow", params: &[], outputs: &["value"], placement: Placement::Pane, needs: INTRADAY, definition,
                   first: &[usize::MAX], sparse: false, compute: None, note: "not available yet — needs intraday (lower-timeframe/tick) bars" }
}

pub const DEFS: &[IndicatorDef] = &[
    def("line_break", "Line Break (bar-aligned)", LINE_BREAK,
        &["line_open", "line_close", "direction", "new_line", "flip", "run", "rev_level", "cont_level"],
        "N-line break on closes (MetaStock / TradingView / Sierra Chart Three Line Break; AmiBroker: E.M. Pottasch Formula 565, after Nison): seed line first close → first close; a close above the up level adds an up line, a close below the down level adds a down line, otherwise nothing (strict, at most one line per bar); a continuation line opens at the last line's close, a reversal line at the last line's open. Continuation level = the last line's close. Reversal level after a run of r same-colour lines: rule 0 (AmiBroker Formula 565) the close of the line min(r, lines) lines back — the extreme of the last N lines once r > N, the far end of the last opposite line while r ≤ N; rule 1 (Sierra Chart study 131, Must Break Consecutive Boxes < n = No) the open of the last line while r < N, else the extreme of the last N lines (the MetaStock/StockCharts N-line rule). TradingView's rule for a short run is not documented, so it is not offered. Outputs: last line open and close (the seed line before the first line), direction +1/−1 (0 before the first line), new_line +1/−1 on a bar that forms a line, flip 1 when that line reverses the colour (not the first line), run = signed count of consecutive same-colour lines, rev_level and cont_level = the prices the next close must pass (strictly) to reverse or continue (both the first close before the first line)",
        &[0, 0, 0, 0, 0, 0, 0, 0], line_break),
    def("pnf", "Point & Figure (bar-aligned)", PNF,
        &["col_dir", "col_high", "col_low", "boxes", "new_col", "boxes_added", "rev_level", "buy_sig", "sell_sig", "signal"],
        "Point & Figure (StockCharts / MetaStock / TradingView ticker.pointfigure; AmiBroker: G. Kavanagh P&F AFL, close 2004 and high/low 2003 \"based on code in AB help files\"): box s = ATR(length) Wilder (ta.atr) at the bar being tested (mode 0; causal — AmiBroker's LastValue(ATR) uses one value for the whole chart and StockCharts recomputes its ATR box every time the chart is built, so both repaint history; TradingView's ATR box is not documented as fixed or per bar) or fixed box points (mode 1) or percent × that bar's close / 100 (mode 2; causal, unlike TradingView's last-price Percentage). First column (AmiBroker): a one-box down column at ceil(price/s)·s on the first bar (price = the close, or the low with price 1), so with a fixed box every column edge is a multiple of the box (X boxes fill at floor, O boxes at ceil: StockCharts box 12 = 12–12.99 in an X column, 11.01–12 in an O column); with an ATR or % box only the first column is on the grid. An X column adds floor((price − top)/s) boxes when price ≥ top + s; otherwise it reverses when price ≤ top − reversal·s into an O column from top − s down to top − floor((top − price)/s)·s, so a new column holds at least reversal boxes (MetaStock: box 3 × reversal 2 = a 6-point reversal); O columns mirror this; moves are whole boxes with a 1e-6-box tolerance (AmiBroker's library code first rounds the price to the nearest box (close) or 0.1 box (high/low); not copied). Price 1 (AmiBroker high/low AFL, StockCharts High-Low, MetaStock): the continuation extreme is tested first (the high in an X column, the low in an O column) and the other extreme only for a reversal when no box was added, one action per bar (Sierra Chart's open-nearer-the-high order is not offered). Step back 1 (TradingView One Step Back, only with reversal 1): a reversal from a one-box column goes into that column, so a column never holds fewer than 2 figures. Outputs: column direction +1 X / −1 O, column top and bottom prices, boxes = figures in the column, new_col 1 on a bar that starts a reversal column, boxes_added = signed boxes added this bar (−1 on the first bar, which draws the one-box first column), rev_level = top − reversal·s (X) or bottom + reversal·s (O) at this bar's box, buy_sig 1 on the first bar of an X column whose top is at least one box (this bar's box) above the top of the previous X column (double top breakout, Kaufman), sell_sig 1 on the mirror below the bottom of the previous O column (double bottom breakdown); at most once per column, and with an ATR or % box it can fire on a bar that adds no box when the box shrinks; signal +1 after the last buy, −1 after the last sell, 0 before either (StockCharts: a buy signal is in effect until a sell signal). A bar with a missing price after the first column keeps the column (events 0, rev_level at the last box). The first column counts as an O column; a column turned by One Step Back counts as its final direction. Chart patterns (triple tops and others) are not part of this indicator",
        &[13, 13, 13, 13, 13, 13, 13, 13, 13, 13], pnf),
    // sparse: the price objectives are NaN until the first count is fixed
    IndicatorDef { sparse: true, ..def("pnf_patterns", "Point & Figure patterns (bar-aligned)", PNF,
        &["triple_top", "triple_bottom", "quad_top", "quad_bottom", "asc_triple_top", "desc_triple_bottom", "bull_triangle", "bear_triangle",
          "bull_trap", "bear_trap", "long_tail", "spike_up", "spike_down", "target_up", "target_dn"],
        "Point & Figure chart patterns (StockCharts ChartSchool P&F classic patterns and P&F Pattern Alerts; Kaufman, New Trading Systems and Methods ch. 5) on the columns of the Point & Figure indicator with the same parameters (same box, reversal, price and One Step Back rules). Events are 1 on the bar the pattern completes, 0 otherwise, NaN before the first column. X1, X2, X3 = tops of the last three finished X columns (newest first), O1, O2, O3 = bottoms of the last three finished O columns; the box is this bar's box; equal = within half a box; above = at least one box higher (1e-6-box tolerance). Breakout patterns fire on the Point & Figure buy_sig bar (double top breakout: the X column first reaches a box above X1) or sell_sig bar (mirror): triple_top when X2 is within half a box of X1 and X3 is not (two equal reaction highs, five columns), quad_top when X2 and X3 are both within half a box of X1 (seven columns), asc_triple_top when X1 is above X2 (three rising tops: two double top breakouts back to back), bull_triangle when the last four finished columns are X, O, X, O (oldest first) with the second X top a box or more below the first and the second O bottom a box or more above the first (lower highs and higher lows over five columns, resolved by the double top breakout; a trend-line break alone does not count); triple_bottom, quad_bottom, desc_triple_bottom and bear_triangle mirror these on O bottoms. bull_trap: a triple or quadruple top breakout column whose final top is one box above the broken level (within half a box, at the box of the bar it reverses), followed by the next O column; fires on the bar that O column first holds 3 or more boxes (bear_trap mirrors, X column of 3 or more boxes after a one-box triple or quadruple bottom breakdown). long_tail (Long Tail Down, Dorsey per C. Twiggs, StockCharts: prices drop 20 boxes or more): fires on the bar of the first X column after an O column of 20 or more boxes. spike_up / spike_down (C. Twiggs, Incredible Charts; one author's rule): the first bar of a column whose top is 10 boxes or more above X1 (bottom 10 boxes or more below O1), once per column. Price objectives (StockCharts vertical count, declared choice of count column: the column that gave the double top buy / double bottom sell, since StockCharts names breakout and breakdown columns as count columns): fixed on the bar that column reverses (a count is not fixed until the reversal), target_up = its bottom + its boxes × box × reversal, target_dn = its top − its boxes × box × reversal × 2/3 (StockCharts and Dorsey use 2/3 of the reversal for bearish counts); the box is the box at the fixing bar; each level holds until the next count of the same side replaces it, NaN before the first. A column turned by One Step Back clears its pending breakout, trap and count. Left out because the sources leave a rule undefined: spread triple top/bottom (no lookback cap), bullish/bearish catapult (the pattern low is not defined), bullish/bearish signal reversed (minimum run not fixed), high/low pole (whether the 50% retrace is of the whole column or the part above the previous high), bullish shakeout (Mullooly and Investors Intelligence disagree), 45° bullish support / bearish resistance lines (the important low/high is not defined), breakout/reversal count (the measured column is unconfirmed), horizontal count (the base edges are discretionary); Bullish Percent Index needs a stock universe. Double top breakout / double bottom breakdown are buy_sig / sell_sig of the Point & Figure indicator",
        &[13; 15], pnf_patterns) },
    def("kagi", "Kagi (bar-aligned)", KAGI,
        &["tip", "direction", "reversal", "shoulder", "waist", "yang", "yang_flip", "yin_flip", "rev_amt"],
        "Kagi on closes (MetaStock Kagi, StockCharts Kagi (Close), TradingView Kagi chart; Sierra Chart study 129; AmiBroker has only a members-only library formula, not compared): reversal amount r = ATR(length) Wilder (ta.atr) at the bar being tested (mode 0; causal — StockCharts and TradingView ATR reversals use the ATR when the chart is built), fixed points (mode 1, MetaStock / StockCharts points), or percent × the current line end / 100 (mode 2, ChartIQ; causal, unlike TradingView's last-price Percentage; Sierra Chart's % of the first column's end price is not offered). Start = the first close (MetaStock); the first line runs from it to the first different close (any amount; an equal close draws nothing), thick (yang) if up, thin (yin) if down. Each later close: in the line's direction, it extends the line end by any amount; against it by r or more (StockCharts: rising to 1951 with a 20-point reversal, it reverses at 1931 or lower; MetaStock \"at least\"; ChartIQ's strictly-more rule is not offered), the line turns: the old end becomes a shoulder (up → down) or a waist (down → up) and a new line runs to the close; otherwise nothing. Thickness (StockCharts / MetaStock / Sierra Chart): a thin line turns thick when its end goes strictly above the last shoulder, a thick line turns thin when its end goes strictly below the last waist; before the first shoulder or waist the start price stands in for it. Outputs: tip = line end (the start before the first line), direction +1/−1 (0 before the first line), reversal 1 on a bar that turns the line (not the first line), shoulder and waist = the last ones (the start until one forms), yang +1 thick / −1 thin (0 before the first line), yang_flip 1 on a thin → thick bar, yin_flip 1 on a thick → thin bar (the first line sets its thickness with no event), rev_amt = the reversal amount after the bar (this bar's ATR, the points, or percent × tip). A bar with no close (or no ATR yet) is skipped (NaN). Rising shoulders, falling waists, multi-level breaks and Nison's discretionary patterns (nine shoulders, three Buddha, windows, tweezers) are not part of this indicator; the high/low price field (StockCharts) is not offered",
        &[13; 9], kagi),
    intraday("range_bars", "Range bars", "Range bars (TradingView Range chart, built from 1-minute bars; AmiBroker nR range intervals, tick data recommended by the AmiBroker knowledge base; Sierra Chart Range Per Bar in six variants; also thinkorswim, cTrader, ChartIQ, MotiveWave): a bar ends when high − low reaches the range and the next opens at or near the prior close (the variants differ); where a bar ends depends on the price path inside the day, so it needs intraday (tick or 1-minute) data and is not built from daily bars"),
    intraday("reversal_bars", "Reversal bars", "Reversal bars (Sierra Chart Reversal Bar \"ReversalAmount-MinimumLength\", e.g. 4-0; also MotiveWave, Quantower, ATAS): a new bar starts when price reverses the set ticks from the bar's high or low after the minimum length; needs tick data"),
    intraday("momentum_bars", "Momentum bars", "Momentum bars (TradeStation .MBar, price range per bar on an intraday interval; Sierra Chart staff: not offered): the construction rule is not published; needs intraday data"),
    intraday("kase_bars", "Kase bars", "Kase bars (TradeStation .KBar, target true range on an intraday interval): the construction rule is not published; needs intraday data"),
    intraday("flex_renko", "Flex Renko / Hybrid Renko", "Flex Renko (Sierra Chart Flex Renko Bar \"BarSize-TrendOffset-ReversalOffset\", both offsets 0 = classic Renko; MotiveWave Hybrid Renko = N-N/2-N/2): Sierra Chart states these \"only work with Intraday chart data\"; needs tick data. Classic Renko on daily closes is the renko indicator"),
    intraday("flex_renko_inverse", "Flex Renko Inverse", "Flex Renko Inverse (Sierra Chart Flex Renko Inverse Settings: Trend Bar Offset, minimum 1, and Reversal Bar Trigger, minimum 2): intraday chart data only (Sierra Chart); needs tick data"),
    intraday("volume_bars", "Volume bars", "Volume bars (Sierra Chart Volume Per Bar, NinjaTrader 8 Volume, MotiveWave Constant Volume, AmiBroker N-volume bars, ATAS, Quantower): a new bar after a set traded volume; needs tick data"),
    intraday("tick_bars", "Tick bars (trades per bar)", "Tick bars (Sierra Chart Number of Trades Per Bar, NinjaTrader 8 Tick, AmiBroker N-tick bars, ATAS): a new bar after a set number of trades; needs tick data"),
    intraday("delta_bars", "Delta volume bars", "Delta volume bars (Sierra Chart Delta Volume Per Bar, ATAS Delta): a new bar after a set ask − bid volume difference; needs tick data with the trade side"),
    intraday("price_change_bars", "Price change bars", "Price change bars (Sierra Chart Price Change Bar): a new bar after a set number of price changes; needs tick data"),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("line_break", "TradingView: Line Break chart (ticker.linebreak); MetaStock: Three Line Break; Sierra Chart: Three Line Break (study 131); StockCharts: Three Line Break", "LineBreak (E.M. Pottasch AFL library, Formula 565)"),
    ("pnf_patterns", "StockCharts: P&F classic patterns, P&F Pattern Alerts and vertical count price objectives; Incredible Charts: P&F spikes", ""),
    ("pnf", "TradingView: Point & Figure chart (ticker.pointfigure); StockCharts: Point & Figure (Close / High-Low); MetaStock: Point & Figure; Sierra Chart: Point and Figure (study 140)", "PointFigure (G. Kavanagh AFL library: close 2004, high/low 2003)"),
    ("kagi", "TradingView: Kagi chart (ticker.kagi); StockCharts: Kagi (Close); MetaStock: Kagi; Sierra Chart: Kagi Chart (study 129)", ""),
    ("range_bars", "TradingView: Range chart; AmiBroker: nR range intervals; Sierra Chart: Range Per Bar", ""),
    ("reversal_bars", "Sierra Chart: Reversal Bar; MotiveWave: Reversal; ATAS: Reversal", ""),
    ("momentum_bars", "TradeStation: Momentum Bar (.MBar)", ""),
    ("kase_bars", "TradeStation: Kase Bar (.KBar)", ""),
    ("flex_renko", "Sierra Chart: Flex Renko Bar; MotiveWave: Hybrid Renko", ""),
    ("flex_renko_inverse", "Sierra Chart: Flex Renko Inverse Settings", ""),
    ("volume_bars", "Sierra Chart: Volume Per Bar; NinjaTrader 8: Volume; AmiBroker: N-volume bars", ""),
    ("tick_bars", "Sierra Chart: Number of Trades Per Bar; NinjaTrader 8: Tick; AmiBroker: N-tick bars", ""),
    ("delta_bars", "Sierra Chart: Delta Volume Per Bar; ATAS: Delta", ""),
    ("price_change_bars", "Sierra Chart: Price Change Bar", ""),
];

fn kagi(b: &Bars, prm: &Params) -> Output {
    let (len, pct) = (b.len(), n(prm, "mode") == 2);
    let size = match n(prm, "mode") { 1 => vec![p(prm, "amount"); len], 2 => vec![p(prm, "percent"); len], _ => ta::rma(&ta::tr(b, true), n(prm, "length")) };
    let amt = |s: f64, tip: f64| if pct { tip * s / 100. } else { s };
    let mut out = vec![vec![NA; len]; 9];
    // sh/wa = last shoulder/waist, the start price until one forms
    let (mut tip, mut dir, mut yang, mut sh, mut wa) = (NA, 0., 0., NA, NA);
    for i in 0..len {
        let (c, s) = (b.close[i], size[i]);
        if c.is_nan() || s.is_nan() { continue; }
        let (mut turn, mut thick, mut thin) = (false, false, false);
        // back > 0 keeps an equal close from turning the line when r is 0 (flat ATR)
        let back = (tip - c) * dir;
        if tip.is_nan() { (tip, sh, wa) = (c, c, c); }
        else if dir == 0. { if c != tip { dir = if c > tip { 1. } else { -1. }; (yang, tip) = (dir, c); } }
        else if (c - tip) * dir > 0. { tip = c; }
        else if back > 0. && back >= amt(s, tip) { if dir > 0. { sh = tip; } else { wa = tip; } (dir, tip, turn) = (-dir, c, true); }
        if yang < 0. && tip > sh { (yang, thick) = (1., true); } else if yang > 0. && tip < wa { (yang, thin) = (-1., true); }
        out[0][i] = tip;
        out[1][i] = dir;
        out[2][i] = turn as u8 as f64;
        out[3][i] = sh;
        out[4][i] = wa;
        out[5][i] = yang;
        out[6][i] = thick as u8 as f64;
        out[7][i] = thin as u8 as f64;
        out[8][i] = amt(s, tip);
    }
    Ok(out)
}

fn line_break(b: &Bars, prm: &Params) -> Output {
    let (len, nl, sierra) = (b.len(), n(prm, "lines"), n(prm, "rule") == 1);
    let mut out = vec![vec![NA; len]; 8];
    // lines[0] is the seed (first close, first close); run is signed
    let (mut lines, mut run, mut rev): (Vec<(f64, f64)>, i64, f64) = (Vec::new(), 0, NA);
    for i in 0..len {
        let c = b.close[i];
        if c.is_nan() { continue; }
        let mut new = 0.;
        if let Some(&(o, last)) = lines.last() {
            let (up, dn) = match run { 0 => (last, last), r if r > 0 => (last, rev), _ => (rev, last) };
            if c > up { lines.push((if run < 0 { o } else { last }, c)); new = 1.; }
            else if c < dn { lines.push((if run > 0 { o } else { last }, c)); new = -1.; }
        } else { lines.push((c, c)); rev = c; }
        let flip = new != 0. && run != 0 && (run > 0) != (new > 0.);
        if new != 0. { run = if flip || run == 0 { new as i64 } else { run + new as i64 }; }
        let (j, r) = (lines.len() - 1, run.unsigned_abs() as usize);
        if new != 0. { rev = if !sierra { lines[j - r.min(nl)].1 } else if r < nl { lines[j].0 } else { lines[j + 1 - nl].0 }; }
        let (o, cl) = lines[j];
        out[0][i] = o;
        out[1][i] = cl;
        out[2][i] = run.signum() as f64;
        out[3][i] = new;
        out[4][i] = if flip { 1. } else { 0. };
        out[5][i] = run as f64;
        out[6][i] = rev;
        out[7][i] = cl;
    }
    Ok(out)
}

fn pnf_size(b: &Bars, prm: &Params) -> Vec<f64> {
    match n(prm, "mode") { 1 => vec![p(prm, "box"); b.len()], 2 => b.close.iter().map(|c| c * p(prm, "percent") / 100.).collect(), _ => ta::rma(&ta::tr(b, true), n(prm, "length")) }
}

fn pnf(b: &Bars, prm: &Params) -> Output {
    let (len, size) = (b.len(), pnf_size(b, prm));
    let (rev, hl) = (p(prm, "reversal"), p(prm, "source") == 1.);
    let osb = rev == 1. && p(prm, "step_back") == 1.;
    // whole boxes in a move; the tolerance absorbs float drift of top/bottom sums
    let boxes = |m: f64, s: f64| (m / s + 1e-6).floor();
    let mut out = vec![vec![NA; len]; 10];
    // column direction, top, bottom, figures; px/po = top of the last finished X / bottom of the last finished O column
    // ls = last usable box (rev_level on a gap bar); fired = this column already gave its buy/sell
    let (mut dir, mut hi, mut lo, mut nb, mut px, mut po, mut sig, mut ls, mut fired) = (0., NA, NA, 0., NA, NA, 0., NA, false);
    for i in 0..len {
        let (c, mut s) = (b.close[i], size[i]);
        let (u, d) = if hl { (b.high[i], b.low[i]) } else { (c, c) };
        let gap = c.is_nan() || u.is_nan() || d.is_nan() || s.is_nan();
        if dir == 0. && (gap || !(s > 0.)) { continue; }
        // after the first column a gap bar keeps the column and gives no events
        if gap { s = ls; } else { ls = s; }
        let (d0, mut k, mut new) = (dir, 0., false);
        if dir == 0. { lo = (d / s - 1e-6).ceil() * s; hi = lo; dir = -1.; nb = 1.; k = 1.; }
        else if !gap && s > 0. {
            let (cont, back) = if dir > 0. { (boxes(u - hi, s), boxes(hi - d, s)) } else { (boxes(lo - d, s), boxes(u - lo, s)) };
            if cont >= 1. { k = cont; nb += k; if dir > 0. { hi += k * s } else { lo -= k * s } }
            else if back >= rev {
                (k, new) = (back, !(osb && nb == 1.));
                nb = if new { k } else { nb + k };
                if dir > 0. { if new { px = hi; } lo = hi - k * s; if new { hi -= s; } }
                else { if new { po = lo; } hi = lo + k * s; if new { lo += s; } }
                dir = -dir;
            }
        }
        if dir != d0 { fired = false; }
        // first bar of the column whose top/bottom is a box (this bar's box) past the previous X top / O bottom; once per column, as an ATR or % box changes size
        let hit = !gap && s > 0. && !fired && if dir > 0. { boxes(hi - px, s) >= 1. } else { boxes(po - lo, s) >= 1. };
        let (buy, sell) = (hit && dir > 0., hit && dir < 0.);
        fired |= hit;
        if buy { sig = 1.; } else if sell { sig = -1.; }
        out[0][i] = dir;
        out[1][i] = hi;
        out[2][i] = lo;
        out[3][i] = nb;
        out[4][i] = if new { 1. } else { 0. };
        out[5][i] = k * dir;
        out[6][i] = if dir > 0. { hi - rev * s } else { lo + rev * s };
        out[7][i] = if buy { 1. } else { 0. };
        out[8][i] = if sell { 1. } else { 0. };
        out[9][i] = sig;
    }
    Ok(out)
}

fn pnf_patterns(b: &Bars, prm: &Params) -> Output {
    let (o, size, rev, hl) = (pnf(b, prm)?, pnf_size(b, prm), p(prm, "reversal"), p(prm, "source") == 1.);
    let mut out = vec![vec![NA; b.len()]; 15];
    let boxes = |m: f64, s: f64| (m / s + 1e-6).floor();
    let eq = |a: f64, c: f64, s: f64| (a - c).abs() < 0.5 * s;
    // finished columns (dir, top, bottom, figures), oldest first; per current column: tt = triple level it broke, cnt = ±1 it gave a buy/sell,
    // spiked = spike given; trap = ±1 a one-box triple breakout column just finished (waiting for 3 boxes of the next column)
    let (mut cols, mut tt, mut cnt, mut spiked, mut trap, mut up, mut dn): (Vec<[f64; 4]>, f64, f64, bool, f64, f64, f64) = (Vec::new(), NA, 0., false, 0., NA, NA);
    for i in 0..b.len() {
        let (dir, hi, lo, nb, s) = (o[0][i], o[1][i], o[2][i], o[3][i], size[i]);
        if dir.is_nan() { continue; }
        let gap = !(s > 0.) || b.close[i].is_nan() || hl && (b.high[i].is_nan() || b.low[i].is_nan());
        let mut e = [0.; 13];
        if o[4][i] == 1. {
            let f = [o[0][i - 1], o[1][i - 1], o[2][i - 1], o[3][i - 1]];
            if cnt > 0. { up = f[2] + f[3] * s * rev; } else if cnt < 0. { dn = f[1] - f[3] * s * rev * 2. / 3.; }
            trap = if tt.is_nan() { 0. } else if f[0] > 0. && eq(f[1] - tt, s, s) { 1. } else if f[0] < 0. && eq(tt - f[2], s, s) { -1. } else { 0. };
            if dir > 0. && f[3] >= 20. { e[10] = 1.; }
            cols.push(f);
        }
        if i > 0 && dir != o[0][i - 1] { (tt, cnt, spiked) = (NA, 0., false); if o[4][i] != 1. { trap = 0.; } }
        if trap * dir < 0. && nb >= 3. { e[if trap > 0. { 8 } else { 9 }] = 1.; trap = 0.; }
        let side = |d: f64| { let mut v = cols.iter().rev().filter(|c| c[0] == d).map(|c| if d > 0. { c[1] } else { c[2] }); [v.next().unwrap_or(NA), v.next().unwrap_or(NA), v.next().unwrap_or(NA)] };
        let (x, ob) = (side(1.), side(-1.));
        if o[7][i] == 1. || o[8][i] == 1. {
            let (bull, m) = (o[7][i] == 1., cols.len());
            // sign flips O bottoms so "above" means beyond the pattern in its direction
            let (v, g) = if bull { (x, 1.) } else { (ob, -1.) };
            // X2 and X3 each compared with X1: a chain X1≈X2≈X3 lets X1 and X3 drift a box apart with an ATR or % box
            let (tri, quad) = (eq(v[0], v[1], s) && !eq(v[0], v[2], s), eq(v[0], v[1], s) && eq(v[0], v[2], s));
            let k = if bull { 0 } else { 1 };
            e[k] = tri as u8 as f64; e[2 + k] = quad as u8 as f64;
            e[4 + k] = (boxes(g * (v[0] - v[1]), s) >= 1.) as u8 as f64;
            let c = |j: usize| cols[m - j];
            if m >= 4 && c(1)[0] == -g && c(2)[0] == g && c(3)[0] == -g && c(4)[0] == g {
                let (t, r) = if bull { (1, 2) } else { (2, 1) };
                e[6 + k] = (boxes(g * (c(4)[t] - c(2)[t]), s) >= 1. && boxes(g * (c(1)[r] - c(3)[r]), s) >= 1.) as u8 as f64;
            }
            if tri || quad { tt = v[0]; }
            cnt = g;
        }
        if !gap && !spiked && if dir > 0. { boxes(hi - x[0], s) >= 10. } else { boxes(ob[0] - lo, s) >= 10. } { e[if dir > 0. { 11 } else { 12 }] = 1.; spiked = true; }
        for (k, v) in e.iter().enumerate() { out[k][i] = *v; }
        out[13][i] = up;
        out[14][i] = dn;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, closes, compiles, from_ohlc, go};

    #[test] fn line_break_amibroker_rule_needs_the_last_opposite_line_end_in_a_short_run() {
        // up 100→105→110→115 (3 lines), 98 breaks the 3-line low 100: down line 110→98; AmiBroker then needs > 115 (line before the run)
        let o = go("line_break", &closes(&[100., 105., 110., 115., 98., 112., 116.]), &[]);
        assert_series(&o[0], &[100., 100., 105., 110., 110., 110., 110.]);
        assert_series(&o[1], &[100., 105., 110., 115., 98., 98., 116.]);
        assert_series(&o[2], &[0., 1., 1., 1., -1., -1., 1.]);
        assert_series(&o[3], &[0., 1., 1., 1., -1., 0., 1.]);
        assert_series(&o[4], &[0., 0., 0., 0., 1., 0., 1.]);
        assert_series(&o[5], &[0., 1., 2., 3., -1., -1., 1.]);
        assert_series(&o[6], &[100., 100., 100., 100., 115., 115., 98.]);
        assert_series(&o[7], &[100., 105., 110., 115., 98., 98., 116.]);
    }
    #[test] fn line_break_sierra_rule_breaks_only_the_current_line_in_a_short_run() {
        // same path: Sierra Chart's level after one down line is its open 110, so 112 reverses up (line 110→112)
        let o = go("line_break", &closes(&[100., 105., 110., 115., 98., 112., 116.]), &[("rule", 1.)]);
        assert_series(&o[1], &[100., 105., 110., 115., 98., 112., 116.]);
        assert_series(&o[3], &[0., 1., 1., 1., -1., 1., 1.]);
        // up run < 3 lines: level = the last line's open (100, then 105); 3 lines: lowest of the last 3 = 100 (rule 0 gives 100 too)
        assert_series(&o[6], &[100., 100., 105., 100., 110., 110., 112.]);
    }
    #[test] fn line_break_full_run_uses_the_extreme_of_the_last_n_lines() {
        // 4 up lines 100→…→120: the low of the last 3 lines is 105 under both rules; 106 and 105 (strict) do nothing, 104 reverses
        for rule in [0., 1.] {
            let o = go("line_break", &closes(&[100., 105., 110., 115., 120., 106., 105., 104.]), &[("rule", rule)]);
            assert_series(&o[6][4..7], &[105., 105., 105.]);
            assert_series(&o[3], &[0., 1., 1., 1., 1., 0., 0., -1.]);
            assert_series(&o[..2].iter().map(|s| s[7]).collect::<Vec<_>>(), &[115., 104.]);
            // after one down line: AmiBroker needs the last up line's close 120, Sierra Chart that down line's open 115
            assert_eq!(o[6][7], if rule == 0. { 120. } else { 115. });
        }
        // AmiBroker at a run of exactly N = 3 lines that began with a reversal: the level is the close of the line before the run
        let o = go("line_break", &closes(&[100., 110., 95., 90., 85., 105., 111.]), &[]);
        assert_series(&o[5], &[0., 1., -1., -2., -3., -3., 1.]);
        assert_series(&o[6][4..], &[110., 110., 85.]);
        // Sierra Chart: the highest of the last 3 down lines is the reversal line's open 100, so 105 reverses
        let o = go("line_break", &closes(&[100., 110., 95., 90., 85., 105., 111.]), &[("rule", 1.)]);
        assert_series(&o[6][4..], &[100., 90., 105.]);
        assert_series(&o[3][5..], &[1., 1.]);
    }
    #[test] fn line_break_first_line_waits_for_a_different_close() {
        // equal closes add nothing; the first line runs from the first close and is not a flip; a gap bar is skipped
        let o = go("line_break", &closes(&[50., 50., f64::NAN, 49., 50.]), &[("lines", 2.)]);
        assert_series(&o[0], &[50., 50., f64::NAN, 50., 50.]);
        assert_series(&o[1], &[50., 50., f64::NAN, 49., 49.]);
        assert_series(&o[2], &[0., 0., f64::NAN, -1., -1.]);
        assert_series(&o[4], &[0., 0., f64::NAN, 0., 0.]);
        // a reversal up needs a close above the seed close 50 (strict)
        assert_series(&o[6], &[50., 50., f64::NAN, 50., 50.]);
        assert!(go("line_break", &closes(&[]), &[]).iter().all(|s| s.is_empty()));
    }
    #[test] fn rules_reach_line_break_and_new_renko_outputs() {
        for out in ["new_line", "flip", "rev_level"] { compiles("line_break", out).unwrap(); }
        for out in ["brick_high", "flip", "run", "confirm2"] { compiles("renko", out).unwrap(); }
    }
    const BOX1: [(&str, f64); 2] = [("mode", 1.), ("box", 1.)];
    #[test] fn pnf_close_fixed_box_three_box_reversal_and_breakout_signals() {
        // box 1: first column O at ceil(100.5) = 101; 104.9 is 3 boxes up → X 102–104; 101 reverses (104 − 3) → O 103–101;
        // 105 → X 102–105 tops the prior X 104 (buy); 102 → O 105–102; 100 falls below the prior O bottom 101 (sell)
        let o = go("pnf", &closes(&[100.5, 101.2, 103.4, 104.9, 102., 101.9, 101., 105., 106., 102., 100., 99.]), &BOX1);
        assert_series(&o[0], &[-1., -1., -1., 1., 1., 1., -1., 1., 1., -1., -1., -1.]);
        assert_series(&o[1], &[101., 101., 101., 104., 104., 104., 103., 105., 106., 105., 105., 105.]);
        assert_series(&o[2], &[101., 101., 101., 102., 102., 102., 101., 102., 102., 102., 100., 99.]);
        assert_series(&o[3], &[1., 1., 1., 3., 3., 3., 3., 4., 5., 4., 6., 7.]);
        assert_series(&o[4], &[0., 0., 0., 1., 0., 0., 1., 1., 0., 1., 0., 0.]);
        assert_series(&o[5], &[-1., 0., 0., 3., 0., 0., -3., 4., 1., -4., -2., -1.]);
        assert_series(&o[6], &[104., 104., 104., 101., 101., 101., 104., 102., 103., 105., 103., 102.]);
        assert_series(&o[7], &[0., 0., 0., 0., 0., 0., 0., 1., 0., 0., 0., 0.]);
        assert_series(&o[8], &[0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 1., 0.]);
        assert_series(&o[9], &[0., 0., 0., 0., 0., 0., 0., 1., 1., 1., -1., -1.]);
        // One Step Back needs reversal 1: with reversal 3 it changes nothing
        assert_eq!(go("pnf", &closes(&[100.5, 101.2, 103.4, 104.9, 102., 101.9, 101., 105., 106., 102., 100., 99.]), &[BOX1[0], BOX1[1], ("step_back", 1.)]), o);
        // MetaStock: box 3 × reversal 2 = 6 points; 104.9 is 3.1 below the top 108 (1 box), 102 is 6 below (reversal)
        let o = go("pnf", &closes(&[99., 105., 110.9, 104.9, 102.]), &[("mode", 1.), ("box", 3.), ("reversal", 2.)]);
        assert_series(&o[1], &[99., 105., 108., 108., 105.]);
        assert_series(&o[4], &[0., 1., 0., 0., 1.]);
    }
    #[test] fn pnf_first_bar_adds_the_one_box_first_column() {
        // the seed bar draws the first O box, so a first column nothing extends still has a bar with boxes_added ≠ 0
        let o = go("pnf", &closes(&[100.5, 100.7, 104.]), &BOX1);
        assert_series(&o[5], &[-1., 0., 3.]);
        assert_series(&o[4], &[0., 0., 1.]);
    }
    #[test] fn pnf_high_low_tests_the_continuation_extreme_first() {
        // O column at ceil(low 99.6) = 100. Bar 1: the low adds an O (99), so the high's 3-box reversal is ignored;
        // bar 2: no new low, the high 102.5 reverses (99 + 3); bar 3: the high adds an X, the low 98.5 is ignored; bar 4 reverses on the low
        let b = from_ohlc(&[(100., 100.6, 99.6, 100.), (100., 103.2, 98.7, 101.), (101., 102.5, 99.5, 102.), (102., 103., 98.5, 99.),
            (99., 102.9, 99., 100.), (100., 101., 98., 98.5)]);
        let o = go("pnf", &b, &[BOX1[0], BOX1[1], ("source", 1.)]);
        assert_series(&o[0], &[-1., -1., 1., 1., -1., -1.]);
        assert_series(&o[1], &[100., 100., 102., 103., 102., 102.]);
        assert_series(&o[2], &[100., 99., 100., 100., 99., 98.]);
        assert_series(&o[5], &[-1., -1., 3., 1., -4., -1.]);
        assert_series(&o[8], &[0., 0., 0., 0., 0., 1.]);
        assert_series(&o[9], &[0., 0., 0., 0., 0., -1.]);
        // on closes the same bars only extend the first O column to 99 (98.5 fills the O box at ceil = 99, not 98)
        let o = go("pnf", &b, &BOX1);
        assert_series(&o[2], &[100., 100., 100., 99., 99., 99.]);
        assert_series(&o[4], &[0.; 6]);
    }
    #[test] fn pnf_one_box_reversal_and_one_step_back() {
        let b = closes(&[100., 101., 100., 99., 99.5, 101.]);
        // reversal 1: every 1-box turn is a new column; 99 falls below the previous O bottom 100 (sell)
        let o = go("pnf", &b, &[BOX1[0], BOX1[1], ("reversal", 1.)]);
        assert_series(&o[0], &[-1., 1., -1., -1., -1., 1.]);
        assert_series(&o[1], &[100., 101., 100., 100., 100., 101.]);
        assert_series(&o[2], &[100., 101., 100., 99., 99., 100.]);
        assert_series(&o[3], &[1., 1., 1., 2., 2., 2.]);
        assert_series(&o[4], &[0., 1., 1., 0., 0., 1.]);
        assert_series(&o[8], &[0., 0., 0., 1., 0., 0.]);
        // TradingView One Step Back: the X at 101 joins the one-box first column (100–101, 2 figures, now X), so no O column precedes 99
        let o = go("pnf", &b, &[BOX1[0], BOX1[1], ("reversal", 1.), ("step_back", 1.)]);
        assert_series(&o[0], &[-1., 1., -1., -1., -1., 1.]);
        assert_series(&o[1], &[100., 101., 100., 100., 100., 101.]);
        assert_series(&o[2], &[100., 100., 100., 99., 99., 100.]);
        assert_series(&o[3], &[1., 2., 1., 2., 2., 2.]);
        assert_series(&o[4], &[0., 0., 1., 0., 0., 1.]);
        assert_series(&o[5], &[-1., 1., -1., -1., 0., 2.]);
        assert_series(&o[8], &[0.; 6]);
    }
    #[test] fn pnf_percent_box_uses_each_bars_close() {
        // 1 % box: 1 at 100 (column 100), 1.035 at 103.5 (3 boxes up: X 101.035–103.105), 1.046 at 104.6 (+1: 104.151),
        // 1.013 at 101.3: 2.81 boxes below the top, short of 3
        let o = go("pnf", &closes(&[100., 103.5, 104.6, 101.3]), &[("mode", 2.)]);
        assert_series(&o[1], &[100., 103.105, 104.151, 104.151]);
        assert_series(&o[2], &[100., 101.035, 101.035, 101.035]);
        assert_series(&o[5], &[-1., 3., 1., 0.]);
        assert_series(&o[6], &[103., 100., 101.013, 101.112]);
        assert!(go("pnf", &closes(&[]), &[]).iter().all(|s| s.is_empty()));
    }
    #[test] fn pnf_changing_box_fires_once_per_column_on_the_first_breakout_bar() {
        let pct = [("mode", 2.), ("percent", 10.), ("reversal", 1.)];
        // duplicate: X 28.1 then X to 31.6 = 28.1 + 3.5 (buy); 45 adds two 4.5 boxes (40.6), and 31.6 sat below 28.1 + 4.5: still one buy
        let o = go("pnf", &closes(&[100., 72., 42., 13., 29., 14., 35., 45.]), &pct);
        assert_series(&o[1][5..], &[26.7, 31.6, 40.6]);
        assert_series(&o[7], &[0., 0., 0., 0., 0., 0., 1., 0.]);
        // miss: O 73.2–66.7 sits 4.9 under the previous O bottom 71.6 (< box 6.5); 36 extends it to 37.9 with box 3.6, a breakout
        let o = go("pnf", &closes(&[100., 71., 81., 65., 36.]), &pct);
        assert_series(&o[2], &[100., 71.6, 79.7, 66.7, 37.9]);
        assert_series(&o[8], &[0., 0., 0., 0., 1.]);
        assert_series(&o[9], &[0., 0., 0., 0., -1.]);
        // the box shrinks from 5.1 to 4.9: the O bottom 52.6 is now a box under 57.6 although 49 adds no box
        let o = go("pnf", &closes(&[100., 77., 54., 77., 51., 49.]), &pct);
        assert_series(&o[5], &[-1., -2., -5., 2., -4., 0.]);
        assert_series(&o[8], &[0., 0., 0., 0., 0., 1.]);
    }
    #[test] fn pnf_gap_bar_after_the_first_column_keeps_the_column_and_gives_no_events() {
        let c = [100.5, 101.2, 103.4, 104.9, 102., 101.9, 101., 105., 106., f64::NAN, 102., 100., 99.];
        let o = go("pnf", &closes(&c), &BOX1);
        let want = [1., 106., 102., 5., 0., 0., 103., 0., 0., 1.];
        for (j, w) in want.iter().enumerate() { assert_eq!(o[j][9], *w, "output {j}"); }
        // the other bars match the run without the gap bar
        let full = go("pnf", &closes(&[100.5, 101.2, 103.4, 104.9, 102., 101.9, 101., 105., 106., 102., 100., 99.]), &BOX1);
        for j in 0..10 { assert_series(&[&o[j][..9], &o[j][10..]].concat(), &full[j]); }
        // high/low: a missing high is a gap bar too, and a gap before the first column stays NaN
        let b = from_ohlc(&[(1., f64::NAN, 99.6, 100.), (100., 100.6, 99.6, 100.), (100., f64::NAN, 98.7, 101.), (101., 103.2, 99.5, 102.)]);
        let o = go("pnf", &b, &[BOX1[0], BOX1[1], ("source", 1.)]);
        assert_series(&o[2], &[f64::NAN, 100., 100., 101.]);
        assert_series(&o[5], &[f64::NAN, -1., 0., 3.]);
        assert_series(&o[9], &[f64::NAN, 0., 0., 0.]);
    }
    const N: f64 = f64::NAN;
    #[test] fn pnf_patterns_triple_top_bull_trap_ascending_top_and_bullish_count() {
        // box 1, reversal 3: X tops 103, 103; 104 breaks the triple top by one box; 101 reverses into a 3-box O (trap)
        // and fixes the count 101 + 4 boxes × 1 × 3 = 113; 105 tops 104 > 103 (ascending triple top)
        let o = go("pnf_patterns", &closes(&[100., 103., 100., 103., 100., 104., 101., 105.]), &BOX1);
        assert_series(&o[0], &[0., 0., 0., 0., 0., 1., 0., 0.]);
        assert_series(&o[2], &[0.; 8]);
        assert_series(&o[4], &[0., 0., 0., 0., 0., 0., 0., 1.]);
        assert_series(&o[8], &[0., 0., 0., 0., 0., 0., 1., 0.]);
        assert_series(&o[13], &[N, N, N, N, N, N, 113., 113.]);
        assert_series(&o[14], &[N; 8]);
        // a two-box breakout (105) is no trap; count 101 + 5 × 3 = 116
        let o = go("pnf_patterns", &closes(&[100., 103., 100., 103., 100., 105., 102.]), &BOX1);
        assert_series(&o[0], &[0., 0., 0., 0., 0., 1., 0.]);
        assert_series(&o[8], &[0.; 7]);
        assert_series(&o[13][6..], &[116.]);
    }
    const PCT10: [(&str, f64); 3] = [("mode", 2.), ("percent", 10.), ("reversal", 1.)];
    #[test] fn pnf_patterns_triple_and_quadruple_compare_x2_and_x3_with_x1() {
        // 10 % box: X3 139.2, X2 131.4, X1 134.1; 154 breaks out with box 15.4 (half 7.7): X1−X2 2.7, X1−X3 5.1 → quadruple
        // although X2−X3 is 7.8 (a chain X1≈X2≈X3 would call it a triple)
        let o = go("pnf_patterns", &closes(&[100., 117., 132., 143., 108., 138., 110., 137., 119., 130., 154.]), &PCT10);
        assert_series(&o[2][10..], &[1.]);
        assert_series(&o[0][10..], &[0.]);
        // X3 86.5, X2 88.3, X1 93.3; 117 breaks out with box 11.7 (half 5.85): X1−X2 5.0, X2−X3 1.8, but X1−X3 6.8 → triple, not quadruple
        let o = go("pnf_patterns", &closes(&[100., 75., 90., 74., 92., 71., 75., 96., 81., 90., 117.]), &PCT10);
        assert_series(&o[0][10..], &[1.]);
        assert_series(&o[2][10..], &[0.]);
    }
    #[test] fn pnf_patterns_trap_needs_one_box_within_half_a_box() {
        // 10 % box: quadruple top X1 88.2 (X2 86.2, X3 92.7) broken at 103 (top 100.8); 74 reverses into a 3-box O with box 7.4:
        // 100.8 − 88.2 = 1.70 boxes, not one box (whole boxes of 1.70 would be 1), so no trap; the count 90.5 + 2 × 7.4 × 1 is fixed
        let o = go("pnf_patterns", &closes(&[100., 83., 93., 76., 87., 94., 73., 93., 80., 103., 74.]), &PCT10);
        assert_series(&o[2][9..], &[1., 0.]);
        assert_series(&o[8], &[0.; 11]);
        assert_series(&o[13][10..], &[105.3]);
    }
    #[test] fn pnf_patterns_quadruple_top_is_not_a_triple_top() {
        let o = go("pnf_patterns", &closes(&[100., 103., 100., 103., 100., 103., 100., 104.]), &BOX1);
        assert_series(&o[2], &[0., 0., 0., 0., 0., 0., 0., 1.]);
        assert_series(&o[0], &[0.; 8]);
    }
    #[test] fn pnf_patterns_triple_bottom_bear_trap_descending_bottom_and_bearish_count() {
        // O bottoms 107 (first column), 107; 106 breaks by one box; 109 reverses into a 3-box X (trap) and fixes the count
        // 109 − 4 × 1 × 3 × 2/3 = 101; 105 falls below 106 < 107 (descending triple bottom)
        let o = go("pnf_patterns", &closes(&[110., 107., 110., 107., 110., 106., 109., 105.]), &BOX1);
        assert_series(&o[1], &[0., 0., 0., 0., 0., 1., 0., 0.]);
        assert_series(&o[3], &[0.; 8]);
        assert_series(&o[5], &[0., 0., 0., 0., 0., 0., 0., 1.]);
        assert_series(&o[9], &[0., 0., 0., 0., 0., 0., 1., 0.]);
        assert_series(&o[14], &[N, N, N, N, N, N, 101., 101.]);
        assert_series(&o[13], &[N; 8]);
    }
    #[test] fn pnf_patterns_trap_waits_for_three_boxes_with_a_one_box_reversal() {
        // reversal 1: X tops 101, 101, then 102 (one-box triple top breakout); the O at 101 has 1 box, 99 makes it 3 (trap),
        // and 99 is also a quadruple bottom breakdown (O bottoms 100, 100, 100)
        let o = go("pnf_patterns", &closes(&[100., 101., 100., 101., 100., 102., 101., 99.]), &[BOX1[0], BOX1[1], ("reversal", 1.)]);
        assert_series(&o[0], &[0., 0., 0., 0., 0., 1., 0., 0.]);
        assert_series(&o[8], &[0., 0., 0., 0., 0., 0., 0., 1.]);
        assert_series(&o[3], &[0., 0., 0., 0., 0., 0., 0., 1.]);
        // count of the breakout column 101–102: 101 + 2 × 1 × 1
        assert_series(&o[13][6..], &[103., 103.]);
    }
    #[test] fn pnf_patterns_triangles_need_lower_highs_and_higher_lows() {
        // X 110, O 100, X 108 (lower high), O 103 (higher low), 109 breaks above 108
        let o = go("pnf_patterns", &closes(&[100., 110., 100., 108., 103., 109.]), &BOX1);
        assert_series(&o[6], &[0., 0., 0., 0., 0., 1.]);
        assert_series(&o[4], &[0.; 6]);
        // mirror: O 100, X 110, O 102 (higher low), X 108 (lower high), 101 breaks below 102
        let o = go("pnf_patterns", &closes(&[110., 100., 110., 102., 108., 101.]), &BOX1);
        assert_series(&o[7], &[0., 0., 0., 0., 0., 1.]);
        assert_series(&o[6], &[0.; 6]);
        // equal lows are no triangle
        let o = go("pnf_patterns", &closes(&[100., 110., 100., 108., 100., 109.]), &BOX1);
        assert_series(&o[6], &[0.; 6]);
    }
    #[test] fn pnf_patterns_long_tail_fires_on_the_first_reversal_after_20_boxes() {
        // 130 → 111: an O column of 20 figures; 114 reverses
        assert_series(&go("pnf_patterns", &closes(&[130., 111., 114.]), &BOX1)[10], &[0., 0., 1.]);
        assert_series(&go("pnf_patterns", &closes(&[130., 112., 115.]), &BOX1)[10], &[0., 0., 0.]);
    }
    #[test] fn pnf_patterns_spikes_need_ten_boxes_past_the_previous_column() {
        // X top 103, then 113 is 10 boxes above it (once per column); 112 is 9
        assert_series(&go("pnf_patterns", &closes(&[100., 103., 100., 113., 114.]), &BOX1)[11], &[0., 0., 0., 1., 0.]);
        assert_series(&go("pnf_patterns", &closes(&[100., 103., 100., 112.]), &BOX1)[11], &[0.; 4]);
        // first column bottom 100, then an O to 90
        assert_series(&go("pnf_patterns", &closes(&[100., 103., 90.]), &BOX1)[12], &[0., 0., 1.]);
    }
    #[test] fn pnf_patterns_gap_and_empty_inputs() {
        // a gap bar inside the pattern changes nothing and gives no event
        let o = go("pnf_patterns", &closes(&[100., 103., 100., 103., 100., N, 104., 101.]), &BOX1);
        assert_series(&o[0], &[0., 0., 0., 0., 0., 0., 1., 0.]);
        assert_series(&o[8], &[0., 0., 0., 0., 0., 0., 0., 1.]);
        assert!(go("pnf_patterns", &closes(&[]), &[]).iter().all(|s| s.is_empty()));
        assert!(go("pnf_patterns", &closes(&[N, 100.]), &BOX1).iter().all(|s| s[0].is_nan()));
    }
    #[test] fn rules_reach_pnf_pattern_outputs() {
        for out in ["triple_top", "bull_trap", "long_tail", "spike_down", "target_up", "target_dn"] { compiles("pnf_patterns", out).unwrap(); }
    }
    #[test] fn kagi_stockcharts_20_point_reversal_needs_20_or_more() {
        // rising to 1951: 1931.5 is 19.5 back (nothing), 1931 is 20 back (turn, shoulder 1951); falling to 1900: 1919 nothing, 1920 turns (waist 1900)
        let o = go("kagi", &closes(&[1870., 1951., 1940., 1931.5, 1931., 1900., 1919., 1920.]), &[("mode", 1.), ("amount", 20.)]);
        assert_series(&o[0], &[1870., 1951., 1951., 1951., 1931., 1900., 1900., 1920.]);
        assert_series(&o[1], &[0., 1., 1., 1., -1., -1., -1., 1.]);
        assert_series(&o[2], &[0., 0., 0., 0., 1., 0., 0., 1.]);
        assert_series(&o[3], &[1870., 1870., 1870., 1870., 1951., 1951., 1951., 1951.]);
        assert_series(&o[4], &[1870., 1870., 1870., 1870., 1870., 1870., 1870., 1900.]);
        // first line up = thick; the line never breaks the start (waist) 1870, so no thickness change
        assert_series(&o[5], &[0., 1., 1., 1., 1., 1., 1., 1.]);
        assert_series(&o[6], &[0.; 8]);
        assert_series(&o[7], &[0.; 8]);
        assert_series(&o[8], &[20.; 8]);
    }
    #[test] fn kagi_thickness_changes_past_the_last_shoulder_and_waist() {
        // amount 2: up to 15, turn at 12 (shoulder 15), 9 breaks the start waist 10 (thin), turn at 11 (waist 9), 16 passes the shoulder 15 (thick)
        let o = go("kagi", &closes(&[10., 15., 12., 9., 11., 13., 16., 14.]), &[("mode", 1.), ("amount", 2.)]);
        assert_series(&o[0], &[10., 15., 12., 9., 11., 13., 16., 14.]);
        assert_series(&o[2], &[0., 0., 1., 0., 1., 0., 0., 1.]);
        assert_series(&o[3], &[10., 10., 15., 15., 15., 15., 15., 16.]);
        assert_series(&o[4], &[10., 10., 10., 10., 9., 9., 9., 9.]);
        assert_series(&o[5], &[0., 1., 1., -1., -1., -1., 1., 1.]);
        assert_series(&o[6], &[0., 0., 0., 0., 0., 0., 1., 0.]);
        assert_series(&o[7], &[0., 0., 0., 1., 0., 0., 0., 0.]);
        // a down first line is thin; reaching the shoulder (start) 10 exactly is not above it
        let o = go("kagi", &closes(&[10., 8., 10., 10.5]), &[("mode", 1.), ("amount", 2.)]);
        assert_series(&o[5], &[0., -1., -1., 1.]);
        assert_series(&o[6], &[0., 0., 0., 1.]);
    }
    #[test] fn kagi_percent_of_the_line_end_and_atr_at_the_bar() {
        // 4 %: tip 110 needs 4.4 back; 105.7 is 4.3 (nothing), 105.5 is 4.5 (turn); rev_amt then 4 % of 105.5
        let o = go("kagi", &closes(&[100., 110., 105.7, 105.5]), &[("mode", 2.)]);
        assert_series(&o[2], &[0., 0., 0., 1.]);
        assert_series(&o[8], &[4., 4.4, 4.4, 4.22]);
        // ATR mode: the start waits for the first ATR; rev_amt is that bar's Wilder ATR
        let b = crate::lab::testdata::walk(80, 3, false);
        let (o, atr) = (go("kagi", &b, &[("length", 5.)]), go("atr", &b, &[("length", 5.)]));
        assert!(o[0][3].is_nan() && !o[0][4].is_nan());
        assert_series(&o[8][4..], &atr[0][4..]);
    }
    #[test] fn kagi_equal_closes_and_gaps_change_nothing() {
        // the first line waits for a different close; a gap bar is skipped; an equal close never turns the line
        let o = go("kagi", &closes(&[50., 50., f64::NAN, 49., 49.]), &[("mode", 1.), ("amount", 1.)]);
        assert_series(&o[1], &[0., 0., f64::NAN, -1., -1.]);
        assert_series(&o[2], &[0., 0., f64::NAN, 0., 0.]);
        // flat prices: ATR 0, still no turn
        let o = go("kagi", &closes(&[5.; 6]), &[("length", 2.)]);
        assert_series(&o[1], &[f64::NAN, 0., 0., 0., 0., 0.]);
        assert!(go("kagi", &closes(&[]), &[]).iter().all(|s| s.is_empty()));
    }
    #[test] fn rules_reach_kagi_outputs_and_intraday_bar_types_are_listed_only() {
        for out in ["tip", "reversal", "shoulder", "yang", "yang_flip", "yin_flip", "rev_amt"] { compiles("kagi", out).unwrap(); }
        for id in ["range_bars", "reversal_bars", "momentum_bars", "kase_bars", "flex_renko", "flex_renko_inverse", "volume_bars", "tick_bars", "delta_bars", "price_change_bars"] {
            let d = crate::lab::catalogue::get(id).unwrap();
            assert!(d.needs.intraday && d.compute.is_none(), "{id}");
            assert_eq!(compiles(id, "value").unwrap_err().status(), "needs_data", "{id}");
        }
    }
    #[test] fn rules_reach_pnf_outputs() {
        for out in ["col_dir", "new_col", "rev_level", "buy_sig", "sell_sig", "signal"] { compiles("pnf", out).unwrap(); }
    }
}
