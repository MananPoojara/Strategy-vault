//! DirecTrend (marshallcapitalgroup.com): `T:/DirecTrend System for TS (marshallcapitalgroup.com)/Code.rar` -> `Code.txt`
//! (TradeStation code; no manual in the folder). `T:` = research drive `USB HDD 1/TradeStation Systems & Indicators`.
//! Book id "dtrend". Notes: library/dtrend/README.md.
use crate::books::ajmisc::{ord, Sim, Ty, K};
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "dtrend";
const ID: &str = "dtrend_directrend";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "DirecTrend (swing p1-p2-p3 projection, quarter-way stop entry, RSI(3) filter)", group: "DirecTrend (Code.txt)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
        "F|Swing points: \"if SwingHighbar(1,h,1,5)=1 and seq[0]<>1 then ... px[0]=h[1]; seq[0]=1\"; \"if SwingLowbar(1,l,1,5)=1 and seq[0]<>0 then ... px[0]=l[1]; seq[0]=0\"; two highs in a row insert a 'phantom valley' (lowest low between them, bars 2.. back), two lows a 'phantom peak'",
        "F|After a high (seq[0]=1): sl = lowest low since; \"p4l=px[0]+sl-px[1]; avgp3p4=(sl+p4l)/2; ed=(p4l-sl)/4; entr=sl+ed; stopl=sl-.05\"; after a low: sh = highest high since; \"p4s=px[0]+sh-px[1]; entr=sh+(p4s-sh)/4; stoph=sh+.05\"",
        "F|\"if seq[0]=1 and rsi(c,3)<30 and low<avgp3p4 then rsil=true\"; \"if seq[0]=0 and rsi(c,3)>70 and high>avgp3p4 then rsis=true\" (reset when the swing side changes)",
        "E|\"if marketposition=0 and seq[0]=1 and open of tomorrow < p4l and entr-stopl>stp_marg and rsil=true then Buy Next Bar at entr stop\" (stp_marg .25, rsi_filt 1)",
        "E|\"if marketposition=0 and seq[0]=0 and open of tomorrow > p4s and stoph-entr>stp_marg and rsis=true then Sell Short Next Bar at entr stop\"",
        "X|\"Buy to Cover(\"short stop\") Next Bar at stoph stop; Sell(\"long stop\") Next Bar at stopl stop\" (stop_fil 1)",
        "X|\"Sell(\"long obj\") Next Bar at p4l limit; Buy to Cover(\"short obj\") Next Bar at p4s limit\" (targ_fil 1)"],
        notes: "SwingHighBar(1,h,1,5)=1 = yesterday's high >= the high before it and > today's high (TradeStation SwingHighBar: strength 1, at least as high as the bar on the left, higher than the bar on the right; help.tradestation.com); lows mirror. \
The .05 stop offset and .25 minimum risk are fixed price amounts in the code (kept literal on index points). The exit orders are re-sent every bar from the last computed stopl/p4l (long) and stoph/p4s (short); they act only on a position held at the open. \
RSI = Wilder RSI (TradeStation's RSI smoothing). BarNumber = bar index. No market or bar size named: daily cash indices (instrument swap). Fills: vault TradeStation daily-bar convention (ajmisc.rs header). India: instrument swap; index shorts hypothetical.",
        claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some((ID, "DirecTrend (marshallcapitalgroup.com)", "code")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("stp_marg", "stp_marg (minimum entry-stop distance)", 0.25, 0., 50., 0.25), p("rsi_filt", "rsi_filt", 1., 0., 1., 1.),
        p("stop_fil", "stop_fil", 1., 0., 1., 1.), p("targ_fil", "targ_fil", 1., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (marg, rf, sf, tf) = (g("stp_marg", 0.25), g("rsi_filt", 1.) >= 0.5, g("stop_fil", 1.) >= 0.5, g("targ_fil", 1.) >= 0.5);
    let (h, l) = (&x.h, &x.l);
    let r3 = rsi(&x.c, 3);
    let (mut px, mut barx, mut seq) = ([0f64; 5], [0usize; 5], [0i32; 5]);
    let (mut lwpan, mut hipan, mut sh, mut sl, mut new) = (9999f64, 0f64, 0f64, 9999f64, 0i64);
    let (mut p4l, mut p4s, mut avg34, mut entr, mut stopl, mut stoph) = (0f64, 0f64, 0f64, 0f64, 0f64, 0f64);
    let (mut rsil, mut rsis) = (false, false);
    let mut s = Sim::new();
    let shift = |px: &mut [f64; 5], barx: &mut [usize; 5], seq: &mut [i32; 5]| for ct in (1..=2).rev() { px[ct] = px[ct - 1]; barx[ct] = barx[ct - 1]; seq[ct] = seq[ct - 1]; };
    let st = (from.max(2)..x.c.len()).find(|&i| r3[i].is_finite()).unwrap_or(x.c.len());
    for i in st..to {
        let new_prev = new;
        let shb = h[i - 1] >= h[i - 2] && h[i - 1] > h[i];
        let slb = l[i - 1] <= l[i - 2] && l[i - 1] < l[i];
        if shb && seq[0] != 1 { shift(&mut px, &mut barx, &mut seq); px[0] = h[i - 1]; barx[0] = i - 1; seq[0] = 1; new += 1; }
        if slb && seq[0] != 0 { shift(&mut px, &mut barx, &mut seq); px[0] = l[i - 1]; barx[0] = i - 1; seq[0] = 0; new += 1; }
        if shb && seq[0] == 1 && new == new_prev {
            for ct in 2..i.saturating_sub(barx[0]) { if l[i - ct] < lwpan { lwpan = l[i - ct]; } }
            px[2] = px[1]; px[1] = lwpan; px[0] = h[i - 1]; barx[0] = i - 1;
            seq[2] = 1; seq[1] = 0; seq[0] = 1; lwpan = 9999.; new += 1; sl = 99999.;
        }
        if slb && seq[0] == 0 && new == new_prev {
            for ct in 2..i.saturating_sub(barx[0]) { if h[i - ct] > hipan { hipan = h[i - ct]; } }
            px[2] = px[1]; px[1] = hipan; px[0] = l[i - 1]; barx[0] = i - 1;
            seq[2] = 0; seq[1] = 1; seq[0] = 0; hipan = 0.; new += 1; sh = 0.;
        }
        if seq[0] == 0 { sl = 99999.; if h[i] > sh { sh = h[i]; } }
        if seq[0] == 1 { sh = 0.; if l[i] < sl { sl = l[i]; } }
        if seq[0] == 1 { p4l = px[0] + sl - px[1]; avg34 = (sl + p4l) / 2.; entr = sl + (p4l - sl) / 4.; stopl = sl - 0.05; rsis = false; }
        if seq[0] == 0 { p4s = px[0] + sh - px[1]; avg34 = (sh + p4s) / 2.; entr = sh + (p4s - sh) / 4.; stoph = sh + 0.05; rsil = false; }
        if seq[0] == 1 && r3[i] < 30. && l[i] < avg34 { rsil = true; }
        if seq[0] == 0 && r3[i] > 70. && h[i] > avg34 { rsis = true; }
        let (o1, mp) = (x.o[i + 1], s.pos);
        let mut q = vec![];
        if mp == 0 && seq[0] == 1 && o1 < p4l && entr - stopl > marg && (rsil || !rf) { q.push(ord(K::Buy, Ty::Stop(entr), "entry stop")); }
        if mp == 0 && seq[0] == 0 && o1 > p4s && stoph - entr > marg && (rsis || !rf) { q.push(ord(K::Short, Ty::Stop(entr), "entry stop")); }
        if sf { q.push(ord(K::XS, Ty::Stop(stoph), "short stop")); q.push(ord(K::XL, Ty::Stop(stopl), "long stop")); }
        if tf { q.push(ord(K::XL, Ty::Lim(p4l), "long obj")); q.push(ord(K::XS, Ty::Lim(p4s), "short obj")); }
        s.bar(x, i + 1, &q);
    }
    Some(s.done(x, to))
}
