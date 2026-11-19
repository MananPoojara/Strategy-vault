//! YT Strategies · Gap Down: the team's AmiBroker AFLs in "Old Topics/Gap Down" that trade individual stocks on
//! daily bars. Coded from the AFLs as written, with the AFLs' own default settings; there is no book PDF and nothing
//! here is checked against the team's workbooks.
//!
//! Coded (one stock at a time, each trade opened and closed on the same daily bar, `AllowSameBarExit` on):
//! - INTRADAY GAP DOWN BUY.afl: the open is more than GAP % (1) below the previous bar's low (option: close); buy at
//!   the open + 0.25% and sell at that bar's close (`Sell = BUYSIG AND NOT LOCIRCUIT`, true on every entry bar);
//! - INTRADAY GAP DOWN BUY_With SMA & ROC filter.afl: the same with the previous close above its 200-day average;
//! - INTRADAY GAP DOWN SHORT WHEN PREVIOUS BULLISH CANDLE.afl (Shock system): after a bullish bar (close in the top
//!   20% of its range, open in the bottom 20%), short at the open - 0.25% when the open is below the previous low
//!   + 1%, with the stock's own monthly SuperTrend(20, 1, mid point) down; cover at the close (or at the optional
//!   trailing level, off by default) on the same bar (`Cover = ... OR shortsig`).
//!
//! AmiBroker behaviour followed (see `ytsw.rs` / `ytwh.rs` / `ytnr.rs`):
//! - `Ref(x, -1)` is Null on the first bar (no trade there); `MA(C, 200)` is Null until 200 bars exist, and the
//!   array comparison with it is false there;
//! - the lower-circuit test is `Prec(O,2) == Prec(C,2) AND Prec(O,2) == Prec(L,2)` on the entry bar itself and the
//!   penny filter is `C > 0` on the entry bar (both read that bar's close, as written);
//! - trade prices are kept inside the bar's low-high range (AmiBroker's price bound check);
//! - the optional `ApplyStop(stopTypeLoss, percent)` (default 0 = off) runs with `ActivateStopsImmediately`, so it
//!   is live on the entry bar and is taken before the close exit, at entry x (1 - SL%) (kept within the bar);
//! - the monthly SuperTrend is `books::ytsw::st_filter` (the same "Harshit" ST loop; ATR(1) Null on the first
//!   bar; `TimeFrameExpand` expandLast); the trend starts at 1 (up), so no short until the monthly bands exist;
//! - the turnover filter (off) is close x volume against the AFL's year table; a missing volume fails it.
//! Not modelled: PositionScore ranking (`Ref(100 - ROC(C, 1), -1)` in the SMA AFL), Maximum Open Positions (20) and
//! position sizing, which only choose among stocks.
//!
//! Not coded:
//! - Single and Double Confirmation - FO Intraday Candle/Intraday GapDown Buy ... ('-' Nick Radge ROC).afl at the
//!   folder root: only the standard "Price" chart section (no trading rules).
//! The one-minute "Intraday GapDown Buy with Time Condition" and the positive/negative ROC confirmation variants
//! have provisional held implementations, including the no-filter version. Their forward-referenced exit can
//! execute on the entry minute; portfolio ranking is not modelled. The intraday Shock AFL has a provisional
//! implementation including its optional stops and filters.
#![allow(dead_code)]
use crate::books::ytd7::{ma, prec2};
use crate::books::ytsw::{st_filter, tf_of};
use crate::books::ytwh::bound;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Inputs already expanded to one-minute bars for the AFL "Intraday GapDown Buy with Time Condition".
/// The daily expansion and its possible look-ahead must be reviewed before this draft can be run publicly.
#[derive(Clone, Copy)]
struct MinuteGapInputs {
    date: i32, time: u16, open: f64, close: f64,
    previous_daily_close: Option<f64>, previous_daily_low: Option<f64>, today_open: Option<f64>,
    expanded_dma: Option<f64>, preceding_close: Option<f64>, preceding_dma: Option<f64>,
    turnover: bool,
}

/// Source-loop signals and actual fill prices, including the source's independent buy and sell `if`s.
/// Consequently a purchase may be sold on the same minute. Future references are explicit and are not
/// replaced by a guessed 15:20 exit. Missing next bars make the corresponding comparison unavailable.
fn minute_gap_signals(b: &[MinuteGapInputs], gap_pct: f64, penny: f64) -> Vec<(Option<f64>, Option<f64>)> {
    let mut out = vec![(None, None); b.len()];
    let mut position = false;
    for i in 1..b.len() {
        let bar = b[i];
        let gap = bar.previous_daily_close.zip(bar.today_open)
            .is_some_and(|(prev, open)| open < prev * (1. - gap_pct / 100.));
        let dma = bar.preceding_close.zip(bar.preceding_dma).is_some_and(|(close, ma)| close > ma);
        if !position && gap && dma && bar.expanded_dma.is_some_and(|v| v > 0.)
            && bar.time == 920 && bar.close > penny && bar.turnover {
            out[i].0 = Some(bar.open);
            position = true;
        }
        let exit = b.get(i + 2).is_some_and(|future| bar.date != future.date)
            || b.get(i + 1).is_some_and(|future| bar.time != future.time);
        if position && exit {
            out[i].1 = Some(bar.close);
            position = false;
        }
    }
    out
}

fn confirmation_signals(b:&[MinuteGapInputs],gap:f64,use_low:bool,double:bool,max_price:f64,id:&str)->Vec<(Option<f64>,Option<f64>)> {
    let mut out=vec![(None,None);b.len()];let mut position=false;
    for i in 1..b.len() {
        let bar=b[i];let base=if use_low {bar.previous_daily_low}else{bar.previous_daily_close};
        let threshold=base.map(|v|v*(1.-gap/100.));
        let is_gap=threshold.zip(bar.today_open).is_some_and(|(t,o)|o<t);
        let price_ok=bar.previous_daily_close.is_some_and(|v|v>0. && v<max_price);
        let filter=if id==CONFIRM_NONE {true} else {
            let close=if id==CONFIRM_POS {bar.previous_daily_close}else{bar.preceding_close};
            close.zip(bar.preceding_dma).is_some_and(|(c,m)|c>m) && bar.expanded_dma.is_some_and(|m|m>0.)
        };
        if !position && is_gap && price_ok && filter && bar.turnover && bar.time==920
            && (!double || threshold.is_some_and(|t|bar.open<t)) {
            out[i].0=Some(bar.open);position=true;
        }
        let exit=b.get(i+2).is_some_and(|next|bar.date!=next.date) || b.get(i+1).is_some_and(|next|bar.time!=next.time);
        if position && exit {out[i].1=Some(bar.close);position=false;}
    }
    out
}

pub const BOOK: &str = "yt_gd";
pub const BUY: &str = "ytgd_buy";
pub const BUY_SMA: &str = "ytgd_buy_sma";
pub const SHOCK: &str = "ytgd_shock_short";
pub const MINUTE_TIME: &str = "ytgd_minute_time";
pub const CONFIRM_POS: &str = "ytgd_confirm_pos";
pub const CONFIRM_NEG: &str = "ytgd_confirm_neg";
pub const CONFIRM_NONE: &str = "ytgd_confirm_no_filter";
pub const SHOCK_IEOD: &str = "ytgd_shock_intraday";
pub const INTRADAY_IDS: [&str;5] = [MINUTE_TIME,CONFIRM_POS,CONFIRM_NEG,CONFIRM_NONE,SHOCK_IEOD];
pub const IDS: [&str; 3] = [BUY, BUY_SMA, SHOCK];

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Gap Down", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

fn confirm_def(id:&'static str,name:&'static str)->StrategyDef {
    StrategyDef {id,name,group:"YT Strategies · Gap Down",session:0,dir:"Long",status:Status::Live,needs_volume:false,needs_vix:false,needs_series:&[],book:BOOK,
        rules:&["F|Gap below prior close or low, with optional second confirmation", "E|Buy at 09:20 open", "X|Sell when the AFL forward-reference exit condition is true"],
        notes:"Single or double confirmation FO intraday AFL; the positive/negative ROC ranking selects among a portfolio and is not modelled in a single-symbol run. Default single confirmation and previous-close gap. The source exit is a future reference that sells at the close of the entry minute, as written. One-minute bars of the selected ieodfo F&O futures (the team's IEOD database). Not compared with the team's trade sheets.",claim:None}
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef { id: MINUTE_TIME, name: "Gap Down Buy with Minute Time Condition", group: "YT Strategies · Gap Down", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["F|Open below prior daily close by gap percentage; daily moving-average filter", "E|Buy at 09:20 open", "X|Sell at close when the AFL's forward-reference exit condition is true"],
            notes: "Gap down strategy/GAP DOWN_Intra day time frame/Intraday GapDown Buy with Time Condition.afl. One-minute bars of the selected ieodfo F&O futures (the team's IEOD database); TimeFrameGetPrice gives the day's open and the previous close. The AFL uses independent buy and sell conditions and its forward-reference exit sells at the close of the 09:20 entry minute, as written. Not compared with the team's trade sheets.", claim: None },
        confirm_def(CONFIRM_POS,"Gap Down Single or Double Confirmation, Positive ROC"),
        confirm_def(CONFIRM_NEG,"Gap Down Single or Double Confirmation, Negative ROC"),
        confirm_def(CONFIRM_NONE,"Gap Down Single or Double Confirmation, No Filter"),
        StrategyDef{id:SHOCK_IEOD,name:"Intraday Gap Down Shock Short",group:"YT Strategies · Gap Down",session:0,dir:"Short",status:Status::Live,needs_volume:false,needs_vix:false,needs_series:&[],book:BOOK,
            rules:&["F|Prior daily candle is bullish and price gaps below the specified level", "E|Short at the first candle open less buffer", "X|Cover on the source time or end-of-day conditions"],
            notes:"GAP DOWN SHORT WHEN PREVIOUS BULLISH CANDLE with intraday trailing sl.afl. Monthly/weekly/daily ST and no-filter modes and the source hard and trailing stops are modelled. The AFL states no analysis interval: the candle minutes setting is the Analysis periodicity (default 1 minute). A source ST period of zero is not supported. Not compared with the team's trade sheets.",claim:None},
        def(BUY, "Intraday Gap Down Buy", "Long", &[
            "F|Open < previous Low − gap % (1) of it (gap calculation option: previous Close)",
            "F|Entry bar is not a lower circuit (Open = Close = Low to 2 decimals; option, on)",
            "F|Entry bar Close > penny price (0); optional turnover filter (off)",
            "E|Buy at the Open + 0.25% (kept within the bar)",
            "X|Optional hard stop % (off), live on the entry bar: at entry × (1 − SL%)",
            "X|Sell at the same bar's Close",
        ], concat!("INTRADAY GAP DOWN BUY.afl (folder root), coded as the AFL is written. Same rules: Gap down strategy/INTRADAY GAP DOWN BUY (No filter).afl and GAP DOWN_With & without filter/INTRADAY GAP DOWN BUY (Wo filter).afl (Param instead of Optimize, same defaults); FO Intraday - Without Filter/INTRADAY GAP DOWN BUY (No filter).afl comments out the lower-circuit test (parameter lower-circuit filter = 0).",
            " Runs on one stock at a time; the team ran it over stock lists (F&O stocks); Maximum Open Positions (20) and position sizing are not modelled. The lower-circuit and penny tests read the entry bar's own close, as written. Prices are kept within the bar's range.")),
        def(BUY_SMA, "Intraday Gap Down Buy, 200-day SMA Filter", "Long", &[
            "F|Previous Close > its 200-day SMA (as of the previous bar)",
            "F|Open < previous Low − gap % (1) of it (gap calculation option: previous Close)",
            "F|Entry bar is not a lower circuit (Open = Close = Low to 2 decimals; option, on)",
            "F|Entry bar Close > penny price (0); optional turnover filter (off)",
            "E|Buy at the Open + 0.25% (kept within the bar)",
            "X|Optional hard stop % (off), live on the entry bar: at entry × (1 − SL%)",
            "X|Sell at the same bar's Close",
        ], concat!("Gap down strategy/INTRADAY GAP DOWN BUY_With SMA & ROC filter.afl, coded as the AFL is written (same rules: the copy in GAP DOWN_With & without filter, Param instead of Optimize). The ROC(1) only feeds PositionScore (100 - ROC of the previous bar), which ranks stocks and is not modelled.",
            " Runs on one stock at a time; the team ran it over stock lists (F&O stocks); Maximum Open Positions (20) and position sizing are not modelled. The lower-circuit and penny tests read the entry bar's own close, as written. Prices are kept within the bar's range.")),
        def(SHOCK, "Gap Down Short after a Bullish Candle (Shock System)", "Short", &[
            "F|Previous bar bullish: Close > High − 20% of its range and Open < Low + 20% of its range",
            "F|Open < previous Low + gap range % (1) of it (gap calculation option: previous Close)",
            "F|The stock's own monthly SuperTrend (period 20, factor 1, mid point) is down (filter option: none)",
            "E|Short at the Open − 0.25% (kept within the bar)",
            "X|Cover at the same bar's Close",
            "X|Optional trailing SL % (0 = off): High ≥ previous Low + SL% covers at that level",
        ], concat!("INTRADAY GAP DOWN SHORT WHEN PREVIOUS BULLISH CANDLE (Shock system)/INTRADAY GAP DOWN SHORT WHEN PREVIOUS BULLISH CANDLE.afl, coded as the AFL is written on daily bars. The trailing SL % default 0 lies below its declared range 2-6 and is used as written (off). The gap test uses + 1% (the 'GAP RANGE PERCENTAGE' default) above the previous low, as written. The SuperTrend runs on the traded stock itself (no SetForeign).",
            " Runs on one stock at a time; Maximum Open Positions (20) and position sizing are not modelled. Prices are kept within the bar's range.")),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if INTRADAY_IDS.contains(&id) {
        if id==SHOCK_IEOD {return Some(vec![p("tf_min","Candle minutes (the AFL states no interval; the Analysis periodicity)",1.,1.,60.,1.),p("buffer_pct","Entry buffer percent",0.25,0.,2.,0.05),
            p("filter","Filter (0 ST, 1 off)",0.,0.,1.,1.),p("filter_tf","ST timeframe (0 monthly, 1 weekly, 2 daily)",0.,0.,2.,1.),p("st_period","ST period",20.,1.,20.,1.),p("st_factor","ST factor",1.,0.,20.,1.),p("st_point","ST point (1 midpoint, 2 close, 3 pivot)",1.,1.,3.,1.),
            p("candle_pct","Bullish candle range percent",20.,1.,100.,1.),p("gap_calc","Gap calculation (0 low, 1 close)",0.,0.,1.,1.),p("gap_pct","Gap range percent",1.,-20.,20.,0.05),p("trail_pct","Trailing stop percent (0 off)",0.,0.,3.,0.5),p("hard_pct","Hard stop percent (0 off)",0.,0.,3.,0.5),crate::intraday::align_param()]);}
        let mut v=vec![p("gap_pct", "Gap percent", 1., 0., 10., 0.5), p("ma_period", "Daily SMA period", 200., 1., 300., 1.),p("roc_period","Ranking ROC period",1.,1.,100.,1.),p("turnover","Turnover filter (1 on)",0.,0.,1.,1.)];
        if id!=MINUTE_TIME {v.extend([p("gap_low","Calculate gap from prior low (1)",0.,0.,1.,1.),p("double_confirm","Require second confirmation (1)",0.,0.,1.,1.),p("max_price","Previous close maximum",10000.,1000.,100000.,100.)]);}
        return Some(v);
    }
    match id {
        BUY | BUY_SMA => {
            let mut v = vec![
                p("gap_calc", "Gap calculation (0 previous Low, 1 previous Close)", 0.0, 0.0, 1.0, 1.0),
                p("gap_pct", "Gap percentage", 1.0, 1.0, if id == BUY { 3.0 } else { 7.0 }, 0.5),
                p("buy_buff", "Buy price buffer %", 0.25, 0.0, 5.0, 0.05),
                p("lc_flt", "Lower-circuit filter (0 off, 1 on)", 1.0, 0.0, 1.0, 1.0),
                p("hard_sl", "Hard stop loss % (0 off)", 0.0, 0.0, 5.0, 0.5),
                p("penny", "Penny stock price", 0.0, 0.0, 100.0, 1.0),
                p("turnover_flt", "Turnover filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            ];
            if id == BUY_SMA {
                v.insert(0, p("ma_period", "MA periods", 200.0, 10.0, 300.0, 1.0));
            }
            Some(v)
        }
        SHOCK => Some(vec![
            p("range_pct", "Candle range %", 20.0, 1.0, 100.0, 1.0),
            p("gap_calc", "Gap calculation (0 previous Low, 1 previous Close)", 0.0, 0.0, 1.0, 1.0),
            p("gap_pct", "Gap range percentage", 1.0, -2.0, 2.0, 1.0),
            p("trail_pct", "Trailing SL % (0 off)", 0.0, 0.0, 6.0, 1.0),
            p("short_buff", "Short price buffer %", 0.25, 0.0, 2.0, 0.05),
            p("st_flt", "Filter (1 SuperTrend, 0 none)", 1.0, 0.0, 1.0, 1.0),
            p("flt_tf", "Filter time frame (1 monthly, 2 weekly, 3 daily)", 1.0, 1.0, 3.0, 1.0),
            p("st_point", "ST calculation point (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
            p("st_period", "Filter ST period", 20.0, 1.0, 20.0, 1.0),
            p("st_factor", "Filter ST factor", 1.0, 0.0, 20.0, 1.0),
        ]),
        _ => None,
    }
}

/// The AFLs' turnover thresholds by calendar year.
fn volsig(date: &str) -> f64 {
    let y: i64 = date.get(..4).and_then(|s| s.parse().ok()).unwrap_or(0);
    match y {
        ..=1999 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

fn trade(dir: f64, i: usize, px: f64, exit_px: f64, why: &str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px, reason: why.into(), entry_kind: "open", signal_i: i, open: false, exits: Vec::new() }
}

const BIG: f64 = 9_999_999.0;

fn run_minute_gap(id:&str, x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    minute_gap_capture(id,x,from,to,p,&mut Vec::new())
}

/// Candle indices accompany the ordinary trades so the research adapter never parses display text.
fn minute_gap_capture(id:&str, x: &Ctx, from: usize, to: usize, p: &Params, capture:&mut Vec<(usize,usize,bool)>) -> Vec<Trade> {
    if x.r.is_empty() || from > to || to >= x.r.len() { return Vec::new(); }
    let ticker = format!("{}{}", crate::intraday::BASKET_PREFIX, x.r[0].ticker);
    let Some(minutes) = crate::intraday::minutes(&ticker) else { return Vec::new(); };
    let (bars, di) = crate::books::ytsti::candles_for(&minutes, x, 1,crate::intraday::Align::Midnight);
    let period = p.get("ma_period").copied().filter(|v| v.is_finite()).unwrap_or(200.).round().clamp(1., 300.) as usize;
    let gap = p.get("gap_pct").copied().filter(|v| v.is_finite()).unwrap_or(1.).clamp(if id==CONFIRM_NONE {0.}else{1.}, 10.);
    let sma = ma(&x.c, period);
    // TimeFrameExpand's default expandLast exposes the current daily value only at the session's last bar.
    let expanded: Vec<Option<usize>> = (0..bars.len()).map(|i| {
        if bars.last_of_day(i) { Some(di[i]) } else { di[i].checked_sub(1) }
    }).collect();
    let inputs: Vec<MinuteGapInputs> = (0..bars.len()).map(|i| {
        let prior_daily = di[i].checked_sub(1);
        let prev_expand = i.checked_sub(1).and_then(|k| expanded[k]);
        MinuteGapInputs { date: bars.day[i] as i32, time: bars.hhmm[i], open: bars.o[i], close: bars.c[i],
            previous_daily_close: prior_daily.map(|d| x.c[d]), previous_daily_low: prior_daily.map(|d|x.l[d]), today_open: Some(x.o[di[i]]),
            expanded_dma: expanded[i].and_then(|d| sma[d]), preceding_close: prev_expand.map(|d| x.c[d]),
            preceding_dma: prev_expand.and_then(|d| sma[d]), turnover: !p.get("turnover").is_some_and(|v|v.is_finite()&&*v>=0.5)
                || bars.v[i]*bars.c[i]>volsig(&minutes.dates[bars.day[i] as usize]) }
    }).collect();
    let signals = if id==MINUTE_TIME {minute_gap_signals(&inputs,gap,0.)} else {
        let on=|key:&str|p.get(key).is_some_and(|v|v.is_finite() && *v>=0.5);
        let max=p.get("max_price").copied().filter(|v|v.is_finite()).unwrap_or(10000.).clamp(1000.,100000.);
        confirmation_signals(&inputs,gap,on("gap_low"),on("double_confirm"),max,id)
    };
    let mut trades = Vec::new(); let mut entry: Option<(usize, f64)> = None;
    for (i, &(buy, sell)) in signals.iter().enumerate() {
        if di[i] > to { break; }
        if let Some(px) = buy { if di[i] >= from && entry.is_none() { entry = Some((i, px)); } }
        if let (Some(px), Some((e, entry_px))) = (sell, entry) {
            trades.push(Trade { dir: 1., legs: vec![Leg { i: di[e], px: entry_px, w: 1. }], exit_i: di[i], exit_px: px,
                reason: format!("Minute gap entry {:04}, exit {:04}", bars.hhmm[e], bars.hhmm[i]), entry_kind: "open", signal_i: di[e], open: false, exits: Vec::new() });
            capture.push((e,i,false));
            entry = None;
        }
    }
    if let Some((e, entry_px)) = entry {
        if let Some(i) = (e..bars.len()).rev().find(|&i| di[i] <= to) {
            trades.push(Trade { dir: 1., legs: vec![Leg { i: di[e], px: entry_px, w: 1. }], exit_i: di[i], exit_px: bars.c[i], reason: "Open position at end of selected period".into(), entry_kind: "open", signal_i: di[e], open: true, exits: Vec::new() });
            capture.push((e,i,true));
        }
    }
    let pairs:Vec<(usize,usize)>=capture.iter().map(|&(e,z,_)|(e,z)).collect();
    crate::intraday::record(&minutes,&bars,1,crate::intraday::Align::Midnight,&trades,&pairs);
    trades
}

/// Build typed, source-scored observations for the one-minute Gap Down variants.
/// This is a research adapter only; callers must enforce instrument and readiness admission.
/// The event clock is trading date * 2880 + minute-of-day * 2 + phase (open=0, close=1).
/// Every returned mark is the observed open or close of that same source minute. No daily
/// quote is promoted to an intraday mark and a missing ranking score is an error.
pub(crate) fn gap_portfolio_observations(id:&str,x:&Ctx,from:usize,to:usize,p:&Params,first_id:u64)
    -> Result<(Vec<crate::yt_intraday_portfolio::Signal>,Vec<crate::yt_intraday_portfolio::Mark>),String> {
    use crate::yt_intraday_portfolio::{Signal,Mark};
    if ![MINUTE_TIME,CONFIRM_POS,CONFIRM_NEG,CONFIRM_NONE].contains(&id) {return Err("source-defined ranking is unavailable for this gap strategy".into());}
    crate::intraday::validate_research_params(&params(id).ok_or("unknown gap strategy")?,p)?;
    if x.r.is_empty() || from>to || to>=x.r.len() {return Err("invalid selected daily coverage".into());}
    let ticker=format!("{}{}",crate::intraday::BASKET_PREFIX,x.r[0].ticker);
    crate::intraday::available(&ticker)?;
    let minutes=crate::intraday::minutes(&ticker).ok_or("intraday bars are unavailable")?;
    crate::intraday::validate_daily_carrier(&minutes,x.r,to)?;
    let (bars,di)=crate::books::ytsti::candles_for(&minutes,x,1,crate::intraday::Align::Midnight);
    let indices:Vec<usize>=(0..bars.len()).filter(|&i|di[i]>=from&&di[i]<=to).collect();
    if indices.len()>1_000_000 {return Err("selected source exceeds the bounded intraday mark limit".into());}
    let stamp=|i:usize,phase:i64|->Result<i64,String>{
        let date=&x.r[di[i]].date;
        if !crate::baskets::valid_date(date) {return Err("invalid source date".into());}
        let time=bars.hhmm[i];if time/100>23 || time%100>59{return Err("invalid source minute".into());}
        Ok(crate::day_number(date)*2880+i64::from(time/100*60+time%100)*2+phase)
    };
    let mut marks=Vec::with_capacity(indices.len()*2);
    for &i in &indices {
        marks.push(Mark{time:stamp(i,0)?,symbol:x.r[0].ticker.clone(),price:bars.o[i]});
        marks.push(Mark{time:stamp(i,1)?,symbol:x.r[0].ticker.clone(),price:bars.c[i]});
    }
    let mut capture=Vec::new();let trades=minute_gap_capture(id,x,from,to,p,&mut capture);
    if capture.len()!=trades.len(){return Err("timestamped source trade count mismatch".into());}
    let period=p.get("roc_period").copied().filter(|v|v.is_finite()).unwrap_or(1.).round().clamp(1.,100.) as usize;
    let mut signals=Vec::with_capacity(trades.len());
    for (index,(trade,(entry,exit,open))) in trades.into_iter().zip(capture).enumerate() {
        let score=if id==CONFIRM_NONE {
            // The AFL has no PositionScore and defaults to one share per candidate.
            // Equal default sizes are tied; the research selector uses alphabetical order.
            // https://www.amibroker.com/kb/2015/09/28/symbol-selection-when-positionscore-is-not-defined/
            0.
        } else {
            let previous=entry.checked_sub(1).ok_or("ranking requires a previous source bar")?;
            let day=if bars.last_of_day(previous){Some(di[previous])}else{di[previous].checked_sub(1)}.ok_or("ranking requires an expanded prior daily ROC")?;
            let base=day.checked_sub(period).ok_or("ranking requires earlier daily ROC history")?;
            let roc=100.*(x.c[day]/x.c[base]-1.);
            if !roc.is_finite(){return Err("ranking ROC is not finite".into());}
            if id==CONFIRM_NEG {100.-roc}else{100.+roc}
        };
        let signal_id=first_id.checked_add(index as u64).ok_or("portfolio signal ID overflow")?;
        signals.push(Signal{id:signal_id,symbol:x.r[0].ticker.clone(),entry_time:stamp(entry,0)?,entry_price:trade.legs[0].px,
            score,direction:1,exit:if open {None}else{Some((stamp(exit,1)?,trade.exit_px))}});
    }
    Ok((signals,marks))
}

fn run_intraday_shock(x:&Ctx,from:usize,to:usize,p:&Params,capture:&mut Vec<(usize,usize)>)->Vec<Trade> {
    if x.r.is_empty() || from>to || to>=x.r.len(){return Vec::new();}
    let ticker=format!("{}{}",crate::intraday::BASKET_PREFIX,x.r[0].ticker);
    let Some(minutes)=crate::intraday::minutes(&ticker) else{return Vec::new();};
    let get=|k:&str,d:f64|p.get(k).copied().filter(|v|v.is_finite()).unwrap_or(d);
    let tf=get("tf_min",1.).round().clamp(1.,60.) as u16;
    let buffer=get("buffer_pct",0.25).clamp(0.,2.)/100.;
    let (b,di)=crate::books::ytsti::candles_for(&minutes,x,tf,crate::intraday::align_of(p));
    let filter_tf=match get("filter_tf",0.).round() as i64 {1=>'W',2=>'D',_=>'M'};
    let trend=st_filter(x,filter_tf,get("st_point",1.).round().clamp(1.,3.) as i64,get("st_period",20.).round().clamp(1.,20.) as usize,get("st_factor",1.).clamp(0.,20.));
    let expanded=|i:usize|if b.last_of_day(i){Some(di[i])}else{di[i].checked_sub(1)};
    let mut pos:Option<(usize,f64)>=None;let mut out=Vec::new();
    let mut hard:f64=99_999_999.;let mut trail:f64=99_999_999.;let mut lowest:f64=99_999_999.;let mut bars_held=0usize;
    for i in 1..b.len() {
        if di[i]>to {break;}
        let end=i+1<b.len() && b.day[i]!=b.day[i+1];
        let timed=b.hhmm[i]==1515 || b.hhmm[i]==1525;
        if pos.is_none() && di[i]>=from && b.first_of_day(i) {
            let previous=expanded(i-1);let current=expanded(i);
            if let (Some(d),Some(f))=(previous,current) {
                let range=x.h[d]-x.l[d];let pct=get("candle_pct",20.).clamp(1.,100.)/100.;
                let bullish=x.c[d]>x.h[d]-pct*range && x.o[d]<x.l[d]+pct*range;
                let gap_base=if get("gap_calc",0.)>=0.5{x.c[f]}else{x.l[f]};
                let gap=b.o[i]<gap_base*(1.+get("gap_pct",1.).clamp(-20.,20.)/100.);
                if bullish && gap && (get("filter",0.)>=0.5 || trend[f]==-1) {
                    let px=(b.o[i]*(1.-buffer)).clamp(b.l[i],b.h[i]);pos=Some((i,px));bars_held=1;
                    let pct=get("hard_pct",0.).clamp(0.,3.);hard=if pct>0. {px*(1.+pct/100.)}else{99_999_999.};
                    trail=99_999_999.;lowest=99_999_999.;
                }
            }
        }
        if let Some((entry,px))=pos {
            let prev_trail=trail;
            if i>entry {
                bars_held+=1;
                let t=get("trail_pct",0.).clamp(0.,3.);
                if bars_held>=2 && t>0. {lowest=lowest.min(b.l[i]);trail=lowest*(1.+t/100.);}
            }
            if b.c[i]>prev_trail || b.h[i]>hard || end || timed || i+1==b.len() {
                let exit_px=if b.h[i]>hard {hard.clamp(b.l[i],b.h[i])} else {b.c[i]};
                out.push(Trade{dir:-1.,legs:vec![Leg{i:di[entry],px,w:1.}],exit_i:di[i],exit_px,reason:format!("Shock short {:04} to {:04}; {tf}-minute candles",b.hhmm[entry],b.hhmm[i]),entry_kind:"open",signal_i:di[entry],open:false,exits:Vec::new()});capture.push((entry,i));pos=None;
            }
        }
    }
    if let Some((entry,px))=pos {if let Some(i)=(entry..b.len()).rev().find(|&i|di[i]<=to) {
        out.push(Trade{dir:-1.,legs:vec![Leg{i:di[entry],px,w:1.}],exit_i:di[i],exit_px:b.c[i],reason:"Open shock short at end of selected period".into(),entry_kind:"open",signal_i:di[entry],open:true,exits:Vec::new()});capture.push((entry,i));
    }}
    crate::intraday::record(&minutes,&b,tf,crate::intraday::align_of(p),&out,capture);
    out
}

/// The shock source has a modelled buffered-open entry, not an observed execution.
/// Only the close-exit path is admitted here; stop-enabled intrabar timing is unresolved.
pub(crate) fn shock_portfolio_observations(x:&Ctx,from:usize,to:usize,p:&Params,first_id:u64)
    ->Result<(Vec<crate::yt_intraday_portfolio::Signal>,Vec<crate::yt_intraday_portfolio::Mark>),String> {
    use crate::yt_intraday_portfolio::Signal;
    crate::intraday::validate_research_params(&params(SHOCK_IEOD).ok_or("missing shock parameters")?,p)?;
    if ["hard_pct","trail_pct"].iter().any(|key|p.get(*key).is_some_and(|v|*v>0.)) {
        return Err("needs_data: shock stop execution timing within a candle is not established".into());
    }
    if x.r.is_empty() || from>to || to>=x.r.len(){return Err("invalid selected daily coverage".into());}
    let ticker=format!("{}{}",crate::intraday::BASKET_PREFIX,x.r[0].ticker);
    crate::intraday::available(&ticker)?;
    let minutes=crate::intraday::minutes(&ticker).ok_or("intraday bars are unavailable")?;
    let tf=p.get("tf_min").copied().unwrap_or(1.).round().clamp(1.,60.) as u16;
    let (bars,_)=crate::books::ytsti::candles_for(&minutes,x,tf,crate::intraday::align_of(p));
    let (_,marks)=crate::yt_intraday_portfolio::close_observations(x,from,to,&minutes,&bars,Vec::new(),0)?;
    let mut indices=Vec::new();let trades=run_intraday_shock(x,from,to,p,&mut indices);
    if indices.len()!=trades.len(){return Err("timestamped source trade count mismatch".into());}
    let stamp=|i:usize,phase:i64|->Result<i64,String>{
        if i>=minutes.len(){return Err("invalid minute index".into());}
        let date=minutes.date(i);let t=minutes.hhmm[i];
        if !crate::baskets::valid_date(date) || t/100>23 || t%100>59{return Err("invalid source date or minute".into());}
        Ok(crate::day_number(date)*2880+i64::from(t/100*60+t%100)*2+phase)
    };
    let buffer=p.get("buffer_pct").copied().unwrap_or(0.25)/100.;
    let mut out=Vec::with_capacity(trades.len());
    for (i,(t,(e,z))) in trades.into_iter().zip(indices).enumerate() {
        let expected=(bars.o[e]*(1.-buffer)).clamp(bars.l[e],bars.h[e]);
        if (t.legs[0].px-expected).abs()>0.00011 || (t.exit_px-bars.c[z]).abs()>0.00011 {return Err("shock source fill disagrees with its declared price model".into());}
        out.push(Signal{id:first_id.checked_add(i as u64).ok_or("portfolio signal ID overflow")?,symbol:x.r[0].ticker.clone(),entry_time:stamp(bars.start[e],0)?,entry_price:t.legs[0].px,score:0.,direction:-1,
            exit:if t.open {None}else{Some((stamp(bars.end[z],1)?,t.exit_px))}});
    }
    Ok((out,marks))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id==SHOCK_IEOD {return Some(run_intraday_shock(x,from,to,p,&mut Vec::new()));}
    if INTRADAY_IDS.contains(&id) { return Some(run_minute_gap(id, x, from, to, p)); }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let on = |k: &str| g(k).round() as i64 == 1;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let cal: &[f64] = if on("gap_calc") { c } else { l };
    let gap = g("gap_pct") / 100.0;
    let mut out = Vec::new();
    if id == SHOCK {
        let flt: Vec<bool> = if on("st_flt") {
            let tf = tf_of(g("flt_tf").round() as i64, true);
            st_filter(x, tf, g("st_point").round() as i64, g("st_period").round().max(1.0) as usize, g("st_factor")).into_iter().map(|t| t == -1).collect()
        } else {
            vec![true; n]
        };
        let hr = g("range_pct") / 100.0;
        let bull = |j: usize| { let r20 = (h[j] - l[j]) * hr; c[j] > h[j] - r20 && o[j] < l[j] + r20 };
        let (buff, trail_pct) = (g("short_buff") / 100.0, g("trail_pct"));
        for i in from.max(1)..=to {
            if !(bull(i - 1) && o[i] < cal[i - 1] + cal[i - 1] * gap && flt[i]) {
                continue;
            }
            let epx = bound(x, i, o[i] - o[i] * buff);
            // TRAILSL of the previous bar: off (9999999) at 0 %, else that (bullish) bar's low + SL%
            let trail = if trail_pct == 0.0 { BIG } else { l[i - 1] + l[i - 1] * trail_pct / 100.0 };
            let (px, why) = if h[i] >= trail { (trail, "Trailing SL") } else { (c[i], "Cover at close") };
            out.push(trade(-1.0, i, epx, bound(x, i, px), why));
        }
        return Some(out);
    }
    let ma_f: Option<Vec<Option<f64>>> = (id == BUY_SMA).then(|| ma(c, g("ma_period").round().max(1.0) as usize));
    let (buff, hs, penny) = (g("buy_buff") / 100.0, g("hard_sl"), g("penny"));
    let (lc_flt, tflt) = (on("lc_flt"), on("turnover_flt"));
    for i in from.max(1)..=to {
        let buysig = o[i] < cal[i - 1] - cal[i - 1] * gap;
        let locirc = lc_flt && prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i]);
        let ma_ok = ma_f.as_ref().map_or(true, |m| m[i - 1].map_or(false, |m| c[i - 1] > m));
        let turnover = !tflt || x.r[i].volume.unwrap_or(f64::NAN) * c[i] > volsig(&x.r[i].date);
        if !(buysig && ma_ok && !locirc && c[i] > penny && turnover) {
            continue;
        }
        let epx = bound(x, i, o[i] + o[i] * buff);
        let stop = epx * (1.0 - hs / 100.0);
        let (px, why) = if hs > 0.0 && l[i] <= stop { (stop, "Hard stop (ApplyStop)") } else { (c[i], "Sell at close") };
        out.push(trade(1.0, i, epx, bound(x, i, px), why));
    }
    Some(out)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Gap Down"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_gap_portfolio_bridge_uses_observed_minute_phases_when_configured() {
        let Ok(csv)=std::env::var("TEST_INTRADAY_DAILY") else {return};
        if std::env::var_os("INTRADAY_DATA_DIR").is_none(){return;}
        let text=std::fs::read_to_string(csv).unwrap();
        let rows:Vec<crate::Row>=text.lines().skip(1).map(|line| {
            let v:Vec<_>=line.split(',').collect();
            crate::Row{ticker:"BANKNIFTY".into(),date:v[0].into(),open:v[1].parse().unwrap(),high:v[2].parse().unwrap(),low:v[3].parse().unwrap(),close:v[4].parse().unwrap(),volume:v[5].parse().ok()}
        }).collect();
        let from=rows.iter().position(|r|r.date.as_str()>="2019-01-01").unwrap();
        let to=rows.iter().rposition(|r|r.date.as_str()<="2019-01-31").unwrap();
        let x=Ctx::new(&rows,&rows,&[]);
        let (events,marks)=gap_portfolio_observations(CONFIRM_POS,&x,from,to,&Params::new(),17).unwrap();
        assert!(!marks.is_empty());assert!(marks.iter().all(|m|m.symbol=="BANKNIFTY" && m.price>0.));
        for e in &events {assert!(e.score.is_finite());assert!(e.exit.is_none_or(|(t,_)|t>e.entry_time));}
        let ledger=crate::yt_intraday_portfolio::run_gap_research(CONFIRM_POS,&[(&x,from,to)],&Params::new(),1000.,1,0.).unwrap();
        assert!(ledger.accepted.len()<=events.len());
        let (unranked,_)=gap_portfolio_observations(CONFIRM_NONE,&x,from,to,&Params::new(),0).unwrap();
        assert!(unranked.iter().all(|e|e.score==0.));
    }
    #[test]
    fn minute_gap_source_exits_on_entry_bar_when_next_minute_differs() {
        let bar = MinuteGapInputs { date: 20220103, time: 919, open: 98., close: 98.,
            previous_daily_close: Some(100.), previous_daily_low: Some(99.), today_open: Some(98.), expanded_dma: Some(90.),
            preceding_close: Some(100.), preceding_dma: Some(90.), turnover: true };
        let bars = [bar, MinuteGapInputs { time: 920, open: 97., close: 98., ..bar },
            MinuteGapInputs { time: 921, ..bar }];
        let signals = minute_gap_signals(&bars, 1., 0.);
        assert_eq!(signals[1], (Some(97.), Some(98.)));
        assert_eq!(signals[2], (None, None));
        let no_gap = [bar, MinuteGapInputs { time: 920, today_open: Some(99.), ..bar }, bars[2]];
        assert_eq!(minute_gap_signals(&no_gap, 1., 0.)[1], (None, None));
        assert_eq!(confirmation_signals(&bars,1.,false,false,10000.,CONFIRM_POS)[1],(Some(97.),Some(98.)));
        let no_double=[bar,MinuteGapInputs {time:920,open:99.5,..bar},bars[2]];
        assert_eq!(confirmation_signals(&no_double,1.,false,true,10000.,CONFIRM_NONE)[1],(None,None));
    }
    use crate::books::ytwh::testbars::*;
    use crate::Row;

    fn flat(n: usize) -> Vec<(f64, f64, f64, f64)> {
        (0..n).map(|_| (100.0, 101.0, 99.0, 100.5)).collect()
    }

    #[test]
    fn gap_down_buy_open_plus_buffer_sell_close() {
        let mut v = flat(5);
        v.push((97.0, 99.0, 96.5, 98.0)); // open 97 < 99 x 0.99 = 98.01
        v.push((98.5, 99.0, 98.0, 98.8)); // 98.5 > 96.5 x 0.99: no trade
        let t = go(run, BUY, &bars(&v), &[]);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].0, t[0].2), (5, 5));
        assert!((t[0].1 - 97.0 * 1.0025).abs() < 1e-9);
        assert!((t[0].3 - 98.0).abs() < 1e-9);
        // open 98.2 is not 1% below the previous low (98.01) -> nothing
        let mut v = flat(5);
        v.push((98.2, 99.0, 97.0, 98.0));
        assert!(go(run, BUY, &bars(&v), &[]).is_empty());
        // by the previous close (100.5 x 0.99 = 99.495) it is a gap
        assert_eq!(go(run, BUY, &bars(&v), &[("gap_calc", 1.0)]).len(), 1);
    }

    #[test]
    fn lower_circuit_and_hard_stop() {
        let mut v = flat(5);
        v.push((97.0, 98.0, 97.0, 97.0)); // open = low = close: lower circuit
        let r = bars(&v);
        assert!(go(run, BUY, &r, &[]).is_empty());
        assert_eq!(go(run, BUY, &r, &[("lc_flt", 0.0)]).len(), 1);
        let mut v = flat(5);
        v.push((97.0, 99.0, 95.0, 98.0));
        let t = go(run, BUY, &bars(&v), &[("hard_sl", 2.0)]);
        let epx = 97.0 * 1.0025;
        assert!(crate::books::ab::near_ab(t[0].3, epx * 0.98)); // low 95 reaches the stop, taken before the close
        // a buy price above the high is kept at the high
        let mut v = flat(5);
        v.push((97.0, 97.1, 96.0, 96.5));
        assert!((go(run, BUY, &bars(&v), &[])[0].1 - 97.1).abs() < 1e-9);
    }

    #[test]
    fn sma_filter_and_turnover() {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..10).map(|k| { let c = 100.0 + k as f64; (c, c + 1.0, c - 1.0, c) }).collect();
        v.push((95.0, 99.0, 94.0, 98.0));
        let r = bars(&v);
        // MA(200) Null with 10 bars: no trade; MA(10): previous close 109 > its average 104.5
        assert!(go(run, BUY_SMA, &r, &[]).is_empty());
        assert_eq!(go(run, BUY_SMA, &r, &[("ma_period", 10.0)]).len(), 1);
        assert_eq!(go(run, BUY, &r, &[]).len(), 1);
        // turnover filter: no volume fails it; enough volume passes (2024 threshold 13,000,000)
        assert!(go(run, BUY, &r, &[("turnover_flt", 1.0)]).is_empty());
        let rv: Vec<Row> = r.into_iter().map(|mut b| { b.volume = Some(200_000.0); b }).collect();
        assert_eq!(go(run, BUY, &rv, &[("turnover_flt", 1.0)]).len(), 1);
    }

    #[test]
    fn shock_short_after_bullish_bar() {
        let mut v = flat(5);
        v.push((100.0, 104.0, 99.8, 103.9)); // bullish: close in the top 20%, open in the bottom 20%
        v.push((100.5, 101.0, 99.0, 99.5)); // open 100.5 < 99.8 x 1.01 = 100.798
        let r = bars(&v);
        // with the default monthly ST filter there is no down trend in this short history: no short
        assert!(go(run, SHOCK, &r, &[]).is_empty());
        let t = go(run, SHOCK, &r, &[("st_flt", 0.0)]);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].0, t[0].2), (6, 6));
        assert!(crate::books::ab::near_ab(t[0].1, 100.5 * 0.9975));
        assert!((t[0].3 - 99.5).abs() < 1e-9);
        // trailing SL 1%: level 99.8 x 1.01 = 100.798; the high 101 reaches it -> covered at that level
        let t = go(run, SHOCK, &r, &[("st_flt", 0.0), ("trail_pct", 1.0)]);
        assert!((t[0].3 - 99.8 * 1.01).abs() < 1e-9);
        // the open too high (101 > 100.798): no short; a non-bullish previous bar: no short
        let mut v2 = v.clone();
        v2[6].0 = 101.0;
        assert!(go(run, SHOCK, &bars(&v2), &[("st_flt", 0.0)]).is_empty());
        let mut v3 = v.clone();
        v3[5].3 = 102.0;
        assert!(go(run, SHOCK, &bars(&v3), &[("st_flt", 0.0)]).is_empty());
    }

    #[test]
    fn shock_filter_needs_st_down() {
        // a long decline makes the daily ST(20, 1) point down; the bullish bar and gap then short
        let mut v: Vec<(f64, f64, f64, f64)> = (0..60).map(|k| { let c = 200.0 - 3.0 * k as f64; (c + 0.5, c + 1.0, c - 1.0, c - 0.9) }).collect();
        v.push((19.2, 20.5, 19.0, 20.4)); // bullish bar
        v.push((19.1, 19.5, 17.0, 18.0)); // open 19.1 < 19.0 x 1.01
        let r = bars(&v);
        let t = go(run, SHOCK, &r, &[("flt_tf", 3.0)]);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].0, 61);
        // in an up trend (rising bars) the same pattern is filtered out
        let mut u: Vec<(f64, f64, f64, f64)> = (0..60).map(|k| { let c = 20.0 + 3.0 * k as f64; (c - 0.5, c + 1.0, c - 1.0, c + 0.9) }).collect();
        u.push((199.2, 200.5, 199.0, 200.4));
        u.push((199.1, 199.5, 197.0, 198.0));
        assert!(go(run, SHOCK, &bars(&u), &[("flt_tf", 3.0)]).is_empty());
        assert_eq!(go(run, SHOCK, &bars(&u), &[("st_flt", 0.0)]).len(), 1);
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
        assert_eq!(catalogue().len(), IDS.len() + INTRADAY_IDS.len());
        for id in INTRADAY_IDS {assert!(params(id).is_some() && family_of(id).is_some());}
        for id in IDS {
            assert!(params(id).is_some() && family_of(id).is_some());
        }
        assert!(run("ytgd_none", &Ctx::new(&bars(&flat(3)), &bars(&flat(3)), &[]), 0, 2, &Params::new()).is_none());
    }
}
