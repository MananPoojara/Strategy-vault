//! A. W. Cohen, "How to Use the Three-Point Reversal Method of Point & Figure Stock Market Trading" (Chartcraft, 8th rev. ed.
//! 1984; book id `awcpf`; printed pages = PDF page - 2). Reading: claude-continuation-20261020/authors/acohen (agents/awcpf, authors/lanec/admission3/lanec3_ref.py,
//! independent reference). Admitted: trading from buy signal to sell signal on a 3-box-reversal chart of daily highs and lows
//! (p.8-16, 77). Held: formation-specific signals, 45-degree trend lines and counts (judgement), relative strength, breadth,
//! options. The box sizes are dollar amounts, so the rule runs on US instruments only.
use crate::books::lc3::Pos;
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const REVIEWED: &[&str] = &["awcpf_pf3"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "awcpf_pf3", name: "Three-point reversal point & figure: buy on a new X high over the last X column, sell on a new O low", group: "Construction and signals (p.8-22, 77)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "awcpf",
        rules: &["F|\"Under 20, each square stands for 1/2-point ... From 20 to 100, each square stands for 1-point ... (Daily highs and lows are used for determining upmoves and downmoves and not closing prices)\" (p.8); \"Over 100, each square should represent 2 points\" (p.9-10); \"In no vertical column are there ever less than 3 squares\" (p.9)",
            "F|\"If your last chart entry is an X, then look at the daily high. If the stock has gone up enter the additional X or Xs and forget about the lows. If the stock has not gone up then look at the low for a 3-box reversal.\" (p.14)",
            "E|\"A breakout above this double top to 39 registers a buy signal.\" (p.16) = an X column one box above the previous X column",
            "X|\"A breakout below this double bottom to 37 registers a sell signal.\" (p.16); \"If you are long of a stock, the logical point for a stoploss order would be one square below the previous bottom ... which is in effect, trading from signal to signal.\" (p.77) - long and short, each closed and reversed by the opposite signal"],
        notes: "[Declared readings] Boxes form one price ladder: 0.5 below 20, 1 from 20 to 100, 2 above 100; an X is plotted at the highest box at or below the high (p.9: a high of 39-7/8 plots 39), an O at the lowest box at or above the low (a low of 43-1/8 plots 44); a 3-box reversal is three steps on that ladder, also across the 20 and 100 borders. The chart starts from the box of the first close: the first column is the first 3-box move from it. A signal is filled at its box price as a stop order (the book trades 'at the point of the signal', p.76), or at the open when the day opens beyond it. The optional 1/4-point boxes under $5 (p.11) are not offered. Cohen's printed statistics include the trend-line filter and bull/bear-market split (p.74-77), so they are not a test of this bare rule. Dollar boxes: US instruments only. No costs.",
        claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "awcpf_pf3").then_some(("awcpf_pf3", "Cohen point & figure", "3-box reversal"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

/// Box ladder: index of the highest box at or below p, and the price of box k.
pub fn box_floor(p: f64) -> i64 {
    let p = p + 1e-9;
    if p < 20. { (p / 0.5).floor() as i64 } else if p < 100. { 40 + (p - 20.).floor() as i64 } else { 120 + ((p - 100.) / 2.).floor() as i64 }
}
pub fn box_px(k: i64) -> f64 { if k < 40 { k as f64 * 0.5 } else if k < 120 { 20. + (k - 40) as f64 } else { 100. + 2. * (k - 120) as f64 } }
pub fn box_ceil(p: f64) -> i64 { let k = box_floor(p); if box_px(k) < p - 1e-9 { k + 1 } else { k } }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut pos = Pos::new();
    let (mut col, mut top, mut bot) = (0i8, 0i64, 0i64);
    let (mut px_top, mut po_bot): (Option<i64>, Option<i64>) = (None, None);
    let r0 = box_floor(c[0]);
    for i in 1..=to {
        let (hf, lc) = (box_floor(h[i]), box_ceil(l[i]));
        let mut sig = 0i8;
        match col {
            0 => if hf >= r0 + 3 { col = 1; top = hf; bot = r0 + 1; } else if lc <= r0 - 3 { col = -1; bot = lc; top = r0 - 1; },
            1 => if hf > top { let was = top; top = hf; if px_top.is_some_and(|p| top > p && was <= p) { sig = 1; } }
                 else if lc <= top - 3 { px_top = Some(top); col = -1; bot = lc; top -= 1; if po_bot.is_some_and(|p| bot < p) { sig = -1; } },
            _ => if lc < bot { let was = bot; bot = lc; if po_bot.is_some_and(|p| bot < p && was >= p) { sig = -1; } }
                 else if hf >= bot + 3 { po_bot = Some(bot); col = 1; top = hf; bot += 1; if px_top.is_some_and(|p| top > p) { sig = 1; } },
        }
        if i < from || sig == 0 { continue; }
        if sig > 0 { let lv = box_px(px_top.unwrap() + 1); pos.enter(1., i, o[i].max(lv), "stop"); }
        else { let lv = box_px(po_bot.unwrap() - 1); pos.enter(-1., i, o[i].min(lv), "stop"); }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ladder() {
        assert_eq!(box_px(box_floor(39.875)), 39.); assert_eq!(box_px(box_ceil(43.125)), 44.);
        assert_eq!(box_px(box_floor(19.9)), 19.5); assert_eq!(box_px(box_floor(101.)), 100.); assert_eq!(box_px(box_ceil(100.5)), 102.);
    }
}
