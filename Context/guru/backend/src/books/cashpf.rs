//! CASHPROOF system (Cashproof Advisors, 2003), TradeStation open code. Research share USB HDD 1/TradeStation Systems &
//! Indicators/CashProof System for TS (crashproofadvisors.com)/Open Code.txt (copy authors/batch-j/J1w/cashproof.txt).
//! Book id `cashpf`. Lane K17 (titles loop, 2 Nov 2026), to-code row 307, lead decision 57; independent check
//! authors/titles-1030/K17w/k17_ref.py. Orders run in `rrsim`.
use crate::connors::{dmi, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{g, o, pd, sdef, xavg, St, A, O, P};

pub const REVIEWED: &[&str] = &["cashpf_adx"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed code, line by line, with its variables carried from bar to bar (it runs from the third bar). ADX14 = Wilder's ADX(14) as the vault computes it (TradeStation's built-in source is not in the share); XMA20 = XAverage(C,20) seeded with the first close. MP = the position at the close (after the bar's fills). Setups: ADX > 30 and rising -> SETBUY when the low is above XMA20 (SETSELL when the high is below it); cleared by a bar on the other side of XMA20 or by a position on that side. While SETBUY, a bar whose low touches XMA20 sets the buy stop BBUY = High + ENTRYADD, re-placed every bar until cleared, and REBUY = BBUY (sell side mirrored with SSELL = Low - ENTRYADD). Initial stops: while not long, LXSTOP = lowest low of 3 bars - EXITADD; on the bar the buy stop is reached it becomes min(Low - EXITADD, LXSTOP); 'Sell(\"InitialStop\") next bar LXSTOP stop' (short mirrored with SXSTOP). While flat, the 'HG Re-Buy' stop at REBUY stays unless the last trade made money (PositionProfit(1) > 0) or the low makes a 2-bar new low under XMA20 (re-sell mirrored); a re-buy level that has been reached is dropped. ENTRYADD and EXITADD are '1 POINTS' = one tick of the instrument (0.01 US, 0.05 NSE; lead decision 57; p.ticks = 0: no offset). The code's 99999 / 0 'no order' markers are read as no order (some NSE prices exceed 99999). Exit orders act only on a position held at the close they are placed (TradeStation). Next-bar stops fill at their level or the open if the open is through it, along open -> nearer extreme -> other extreme. One unit. The vendor names no market or test; daily indices, ETFs and US/India stocks are a declared application; the vault's standard costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef("cashpf", "cashpf_adx", "CASHPROOF: strong rising ADX(14) above 30, buy (sell) a pullback to the 20-day EMA on a stop above the high (below the low); 3-bar stops, re-entry stops",
        "CASHPROOF (Cashproof Advisors, TradeStation code 2003)", "Long/Short", Status::Live, &[
            "F|\"INPUT:ADXLEN(14),CUTOFF(30),XMALEN(20), ENTRYADD(1 POINTS),EXITADD(1 POINTS);\" \"ADX14=ADX(14); XMA20=XAVERAGE(C,20);\"",
            "F|\"IF ADX14>CUTOFF AND ADX14>ADX14[1] THEN BEGIN IF L>XMA20 THEN SETBUY=TRUE; IF H<XMA20 THEN SETSELL=TRUE; END;\"",
            "E|\"IF SETBUY AND L<=XMA20 THEN BBUY=H+ENTRYADD;\" \"IF BBUY<99999 THEN BEGIN Buy(\\\"Cashproof\\\") Next Bar BBUY STOP; REBUY=BBUY; END;\" (sell side mirrored)",
            "E|\"IF MP=0 THEN BEGIN IF POSITIONPROFIT(1)>0 OR (L<L[2] AND L<L[1] AND H<XMA20) THEN BEGIN REBUY=99999; GOREBUY=FALSE; END; ... IF REBUY<99999 THEN Buy(\\\"HG Re-Buy\\\") Next Bar REBUY STOP;\"",
            "X|\"IF MP<1 THEN LXSTOP=MINLIST(L,L[1],L[2])-EXITADD;\" \"IF BBUY[1]<99999 AND H>=BBUY[1] THEN BEGIN LXSTOP=MINLIST(L-EXITADD,LXSTOP); GOREBUY=TRUE; END;\" \"Sell(\\\"InitialStop\\\") Next Bar LXSTOP STOP;\" (short mirrored)"],
        NOTE)]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("cashpf_adx", "CASHPROOF ADX pullback", "ADX 14 > 30, EMA 20"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![pd("ticks", "ENTRYADD / EXITADD in ticks of the instrument ('1 POINTS'; 0 = no offset)", 1., 0., 5., 1.)])
}

const NONE_HI: f64 = f64::INFINITY; // the code's 99999 'no order' marker
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let add = g(p, "ticks", 1.) * x.market_tick();
    let adx = dmi(x.r, 14).1;
    let xma = xavg(&x.c, 20);
    let (h, l) = (&x.h, &x.l);
    let (mut bbuy, mut ssell, mut rebuy, mut resell) = (NONE_HI, 0.0f64, NONE_HI, 0.0f64);
    let (mut setbuy, mut setsell, mut gorebuy, mut goresell) = (false, false, false, false);
    let (mut lxstop, mut sxstop, mut mp_prev) = (0.0f64, NONE_HI, 0i8);
    Some(run_pp(x, from, to, |i, st, pp, next| {
        if i < 2 { return; }
        let mp = st.pos;
        let (bbuy1, ssell1, rebuy1, resell1, gorebuy1, goresell1) = (bbuy, ssell, rebuy, resell, gorebuy, goresell);
        if mp < 1 { lxstop = l[i].min(l[i - 1]).min(l[i - 2]) - add; }
        if mp > -1 { sxstop = h[i].max(h[i - 1]).max(h[i - 2]) + add; }
        if bbuy1 < NONE_HI && h[i] >= bbuy1 { lxstop = (l[i] - add).min(lxstop); gorebuy = true; }
        if ssell1 > 0. && l[i] <= ssell1 { sxstop = (h[i] + add).max(sxstop); goresell = true; }
        if mp == 1 { next.push(o(A::Xl, P::Stop(lxstop), "InitialStop")); }
        if mp == -1 { next.push(o(A::Xs, P::Stop(sxstop), "InitialStop ")); }
        if adx[i] > 30. && adx[i] > adx[i - 1] {
            if l[i] > xma[i] { setbuy = true; }
            if h[i] < xma[i] { setsell = true; }
        }
        if l[i] > xma[i] { setsell = false; resell = 0.; }
        if h[i] < xma[i] { setbuy = false; rebuy = NONE_HI; }
        if mp == 1 { setbuy = false; }
        if mp == -1 { setsell = false; }
        if !setsell { ssell = 0.; }
        if !setbuy { bbuy = NONE_HI; }
        if mp_prev == 0 {
            if l[i] <= resell1 && goresell1 { resell = 0.; }
            if h[i] >= rebuy1 && gorebuy1 { rebuy = NONE_HI; }
        }
        if mp == 0 {
            if pp > 0. || (h[i] > h[i - 2] && h[i] > h[i - 1] && l[i] > xma[i]) { resell = 0.; goresell = false; }
            if resell > 0. { next.push(o(A::Sell, P::Stop(resell), "HG Re-Sell")); }
            if pp > 0. || (l[i] < l[i - 2] && l[i] < l[i - 1] && h[i] < xma[i]) { rebuy = NONE_HI; gorebuy = false; }
            if rebuy < NONE_HI { next.push(o(A::Buy, P::Stop(rebuy), "HG Re-Buy")); }
        }
        if setbuy && l[i] <= xma[i] { bbuy = h[i] + add; }
        if setsell && h[i] >= xma[i] { ssell = l[i] - add; }
        if bbuy < NONE_HI { next.push(o(A::Buy, P::Stop(bbuy), "Cashproof")); rebuy = bbuy; }
        if ssell > 0. { next.push(o(A::Sell, P::Stop(ssell), "Cashproof ")); resell = ssell; }
        mp_prev = mp;
    }))
}

// ------------------------------------------------------------------ rrsim's order engine, plus PositionProfit(1)
// ponytail: a copy of rrsim::run_ts (same fills, line for line) that also hands the code the last closed trade's profit;
// rrsim is shared by live modules and is not edited. Only next-bar orders (this code places no close orders).
struct Eng<'a> { x: &'a Ctx<'a>, pos: i8, cur: Option<Trade>, out: Vec<Trade> }
impl<'a> Eng<'a> {
    fn close_pos(&mut self, i: usize, px: f64, why: &str, sig: usize) {
        if let Some(mut t) = self.cur.take() { t.exit_i = i; t.exit_px = px; t.reason = why.into(); t.signal_i = sig; self.out.push(t); }
        self.pos = 0;
    }
    fn exec(&mut self, od: &O, i: usize, px: f64, kind: &'static str, sig: usize) {
        match od.a {
            A::Buy | A::Sell => {
                let d: i8 = if od.a == A::Buy { 1 } else { -1 };
                if self.pos == -d { self.close_pos(i, px, &format!("reversed by {}", od.tag), sig); }
                self.pos = d;
                self.cur = Some(Trade { dir: d as f64, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px,
                    reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] });
            }
            A::Xl | A::Xs => self.close_pos(i, px, od.tag, sig),
        }
    }
    fn valid(&self, od: &O) -> bool { match od.a { A::Buy => self.pos != 1, A::Sell => self.pos != -1, A::Xl => self.pos == 1, A::Xs => self.pos == -1 } }
    fn fill_bar(&mut self, i: usize, ords: &[O]) {
        if ords.is_empty() { return; }
        let x = self.x;
        let (op, hi, lo, cl) = (x.o[i], x.h[i], x.l[i], x.c[i]);
        let mut done = vec![false; ords.len()];
        for pass in 0..2 {
            for (k, od) in ords.iter().enumerate() {
                let is_exit = matches!(od.a, A::Xl | A::Xs);
                if matches!(od.p, P::Mkt) && (pass == 0) == is_exit && !done[k] && self.valid(od) { done[k] = true; self.exec(od, i, op, "open", i - 1); }
            }
        }
        let rising = |a: A| matches!(a, A::Buy | A::Xs);
        loop {
            let hit = ords.iter().enumerate().find(|(k, od)| !done[*k] && self.valid(od) && match od.p {
                P::Stop(s) => if rising(od.a) { op >= s } else { op <= s },
                P::Lim(s) => if rising(od.a) { op <= s } else { op >= s },
                P::Mkt => false,
            }).map(|(k, _)| k);
            match hit { Some(k) => { done[k] = true; let od = ords[k]; self.exec(&od, i, op, kind(&od), i - 1); } None => break }
        }
        let path = if hi - op < op - lo { [hi, lo, cl] } else { [lo, hi, cl] };
        let (mut cur, mut seg) = (op, 0);
        while seg < 3 {
            let t = path[seg];
            let up = t > cur;
            let mut best: Option<(usize, f64)> = None;
            for (k, od) in ords.iter().enumerate() {
                if done[k] || !self.valid(od) { continue; }
                let lv = match od.p { P::Stop(s) if rising(od.a) == up => s, P::Lim(s) if rising(od.a) != up => s, _ => continue };
                let inside = if up { lv > cur && lv <= t } else { lv < cur && lv >= t };
                if inside && best.map_or(true, |(_, b)| (lv - cur).abs() < (b - cur).abs()) { best = Some((k, lv)); }
            }
            match best {
                Some((k, lv)) => { done[k] = true; let od = ords[k]; self.exec(&od, i, lv, kind(&od), i - 1); cur = lv; }
                None => { cur = t; seg += 1; }
            }
        }
    }
}
fn kind(od: &O) -> &'static str { match od.p { P::Mkt => "open", P::Stop(_) => "stop", P::Lim(_) => "limit" } }
/// rrsim::run_ts with `pp` = PositionProfit(1), the last closed trade's profit in price points (0 before the first).
fn run_pp(x: &Ctx, from: usize, to: usize, mut f: impl FnMut(usize, &St, f64, &mut Vec<O>)) -> Vec<Trade> {
    let mut e = Eng { x, pos: 0, cur: None, out: vec![] };
    let mut pending: Vec<O> = vec![];
    for i in 0..=to {
        if i > from { e.fill_bar(i, &pending); }
        pending.clear();
        let st = St { pos: e.pos, entry_px: e.cur.as_ref().map_or(0.0, |t| t.legs[0].px), entry_i: e.cur.as_ref().map_or(0, |t| t.legs[0].i) };
        let pp = e.out.last().map_or(0.0, |t| t.dir * (t.exit_px - t.legs[0].px));
        let mut next = vec![];
        f(i, &st, pp, &mut next);
        if i >= from && i < to { pending = next; }
    }
    if let Some(mut t) = e.cur.take() { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; e.out.push(t); }
    e.out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cashproof() { super::super::nijat::tests::sane(run, REVIEWED); }
}

// Instrument admission: CASHPROOF 2026-11-02 (titles lane K17).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "cashpf" && reviewed(def.id),
    label: "Vendor TradeStation code with no market or test named; runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "CASHPROOF Open Code.txt (USB HDD 1/TradeStation Systems & Indicators/CashProof System for TS (crashproofadvisors.com)); independent check authors/titles-1030/K17w/k17_ref.py",
};
