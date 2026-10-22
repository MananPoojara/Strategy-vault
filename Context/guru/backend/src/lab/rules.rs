//! Rule spec v1 (indicators lab spec §4): parse, validate against the catalogue, expand optimize grids.
//! Reserved fields return `Unsupported`; malformed input returns `Invalid` naming the field.
use super::catalogue::{self, Params};
use super::json::{self, Json};
use super::resample::{Expand, Tf, TF_HELP};

pub const LABEL: &str = "User-defined indicator strategy · simulated · not a book method";
pub const MAX_CONDITIONS: usize = 12;
pub const MAX_SPEC_BYTES: usize = 8192;
pub const MAX_OFFSET: usize = 1000;
pub const PRICE_SOURCES: &[&str] = &["open", "high", "low", "close", "hl2", "hlc3", "ohlc4", "volume"];
pub const OPERATORS: &[&str] = &["crosses_above", "crosses_below", ">", "<", ">=", "<=", "=", "!=", "crosses", "between", "outside", "rising", "falling"];
/// Group kinds a rule list may hold besides comparisons (spec §4.1).
pub const NODE_KINDS: &[&str] = &["all", "any", "not", "at_least", "then"];
/// Whole-spec bounds (spec §4.3): comparisons, and comparisons + groups + expressions, over every rule list.
pub const MAX_LEAVES: usize = 40;
pub const MAX_NODES: usize = 120;
pub const MAX_STEPS: usize = 4;

#[derive(Clone, Debug, PartialEq)]
pub enum LabError { Invalid(String), Unsupported(String), NeedsData(String) }
impl LabError {
    pub fn status(&self) -> &'static str {
        match self { Self::Invalid(_) => "invalid", Self::Unsupported(_) => "unsupported", Self::NeedsData(_) => "needs_data" }
    }
    pub fn message(&self) -> &str { match self { Self::Invalid(m) | Self::Unsupported(m) | Self::NeedsData(m) => m } }
    pub fn json(&self) -> String {
        format!("{{\"status\":\"{}\",\"error\":\"{}\",\"label\":\"{LABEL}\"}}", self.status(), json::esc(self.message()))
    }
}
/// Engine helpers report data gaps as "needs_data: …"; any other engine error is an invalid request.
impl From<String> for LabError {
    fn from(e: String) -> Self {
        match e.strip_prefix("needs_data:") { Some(m) => Self::NeedsData(m.trim().into()), None => Self::Invalid(e) }
    }
}
fn inv(m: impl Into<String>) -> LabError { LabError::Invalid(m.into()) }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op { CrossesAbove, CrossesBelow, Gt, Lt, Ge, Le, Eq, Ne, Crosses }
#[derive(Clone, Debug, PartialEq)]
/// `formula` is set on a custom-indicator operand (id "formula"); its params are the formula's Params.
pub struct SeriesRef { pub id: String, pub params: Params, pub tf: Tf, pub out: usize, pub offset: usize, pub foreign: Foreign, pub formula: Option<std::sync::Arc<super::formula::Formula>> }
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Foreign { pub symbol: Option<String>, pub vs: Option<String>, pub fixup: bool }
pub const MAX_FOREIGN: usize = 4;
pub const MAX_SYMBOL_LEN: usize = 80;
pub const MAX_UNIVERSE: usize = 120;
pub const MAX_UNIVERSE_LIST: usize = 50;
pub const MAX_OPEN: usize = 50;
pub const MAX_PORTFOLIO_COMBOS: usize = 200;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode { Signals, Rotation }
#[derive(Clone, Debug, PartialEq)]
pub enum Universe { Basket(String), Tickers(Vec<String>) }
pub type Rebalance = Option<usize>;
#[derive(Clone, Debug, PartialEq)]
pub struct Portfolio { pub universe: Universe, pub mode: Mode, pub score: Option<Operand>, pub lowest: bool, pub max_open: usize, pub worst_rank_held: Option<usize>, pub rebalance: Rebalance }
impl Portfolio { pub fn worst_rank_held(&self) -> usize { self.worst_rank_held.unwrap_or(self.max_open) } }
#[derive(Clone, Debug, PartialEq)]
pub enum Operand { Series(SeriesRef), Value(f64), BarsHeld, Expr(Box<Expr>) }
/// Expression operands (spec §4.1), evaluated on the daily bars.
pub const EXPRESSIONS: &[&str] = &["add", "sub", "mul", "div", "min", "max", "pct", "abs", "ref", "sum", "avg", "highest", "lowest", "change",
    "bars_since", "value_when", "highest_since", "lowest_since", "count", "compress"];
/// TimeFrameCompress modes (spec §5.1).
pub const AGGREGATES: &[&str] = &["last", "first", "high", "low", "sum"];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Agg { Last, First, High, Low, Sum }
impl Agg { pub fn key(self) -> &'static str { AGGREGATES[self as usize] } }
pub const MAX_NTH: usize = 20;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bin { Add, Sub, Mul, Div, Min, Max, Pct }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Win { Sum, Avg, Highest, Lowest, Change }
impl Bin { pub fn key(self) -> &'static str { EXPRESSIONS[self as usize] } }
impl Win { pub fn key(self) -> &'static str { EXPRESSIONS[9 + self as usize] } }
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Bin(Bin, Operand, Operand),
    Abs(Operand),
    Ref(Operand, usize),
    Win(Win, Operand, usize),
    BarsSince(Node),
    ValueWhen { when: Node, of: Operand, nth: usize },
    Since { when: Node, of: Operand, highest: bool },
    Count(Node, usize),
    /// A series operand expanded with `expand: first|point` (`last` stays a plain series operand, the v1 path).
    Expand(SeriesRef, Expand),
    Compress { of: Operand, tf: Tf, agg: Agg, offset: usize, expand: Expand },
}
#[derive(Clone, Debug, PartialEq)]
pub struct Cond { pub lhs: Operand, pub op: Op, pub rhs: Operand }
/// One entry of a rule list (spec §4.1). `Cmp` is the v1 condition; the other kinds are Phase 7.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Cmp(Cond),
    Range { x: Operand, lo: Operand, hi: Operand, outside: bool },
    Trend { x: Operand, bars: usize, rising: bool },
    All(Vec<Node>),
    Any(Vec<Node>),
    Not(Box<Node>),
    AtLeast { k: usize, of: usize, when: Box<Node> },
    Then { steps: Vec<Node>, within: usize },
}
impl Node {
    /// The v1 comparison, when this node is one.
    pub fn cmp(&self) -> Option<&Cond> { if let Node::Cmp(c) = self { Some(c) } else { None } }
}
/// Where an optimize path points: a price-order slot, or a number inside a rule list addressed by its spec path.
#[derive(Clone, Debug, PartialEq)]
pub struct Target { pub path: String, pub slot: Option<Slot> }
/// Market orders fill at the next bar's open (default) or at the signal bar's own close (the book convention).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill { NextOpen, Close }
/// AmiBroker trade price field (Analysis settings › Trades) of a market order's fill bar; Average = (O+H+L+C)/4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Px { Open, High, Low, Close, Average }
pub const MAX_TRADE_DELAY: usize = 20;
/// A price distance: percent of a reference price, or a multiple of ATR(length) read at a bar already closed.
#[derive(Clone, Copy, Debug, PartialEq)]
/// `ProfitPct` (trailing only): the stop sits x % of the open profit below the peak (AmiBroker trailing in % of profit).
pub enum Dist { Pct(f64), Atr { mult: f64, length: usize }, Points(f64), Ticks(f64), ProfitPct(f64) }
/// The price-order settings an optimize path can address (`entry.limit.pct`, `exit.stop_loss.atr`, ...).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot { Limit, StopLoss, Target, Trailing, Stop, Activate, BreakevenAfter, TakeProfit(usize) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Roll { DaysBefore(usize), Volume, OpenInterest, None }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Adjust { Forward, Ratio, None }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Futures { pub roll: Roll, pub adjust: Adjust }
pub const MAX_TICKS: f64 = 100_000.;
pub const MAX_TAKE_PROFITS: usize = 3;
pub const MAX_LEGS: usize = 5;
/// Which price a scale-in leg needs relative to the previous leg: any, better (averaging down a long), or worse (adding to strength).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegPrice { Any, Better, Worse }
/// Extra legs while in a trade (AmiBroker sigScaleIn, Pine pyramiding): `legs[0]` is the first entry; fractions of one position.
#[derive(Clone, Debug, PartialEq)]
pub struct ScaleIn { pub legs: Vec<f64>, pub min_bars: usize, pub price: LegPrice }
/// Position size (spec §7.4). Unit mode accepts Full (one unit), PctEquity and RiskPct; Units and Value need execution.capital.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sizing { Full, PctEquity(f64), RiskPct(f64), Units(f64), Value(f64) }
/// Which exits start a cooldown (AmiBroker ApplyStop ReEntryDelay counts after stops; here after any, losing or winning trades).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum After { Any, Loss, Win }
/// Equity-level entry rules (spec §6.5), evaluated on closed trades: pause after n consecutive losses, stop after a drawdown.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Risk { pub losses: Option<(usize, usize)>, pub max_dd_pct: Option<f64> }
pub const MAX_PAUSE: usize = 100_000;
pub const MAX_CAPITAL: f64 = 1e12;
pub const MAX_TAXES: usize = 6;
/// Which side of a trade a declared tax applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaxSide { Buy, Sell, Both }
/// Money-mode costs (spec §7.4), all user-declared: commission per order / per unit / % of value, slippage in bps or ticks
/// (ticks: futures only), named taxes as % of value on a side, and the yearly borrow % for shorts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MoneyCosts { pub per_order: f64, pub per_unit: f64, pub pct: f64, pub slip_bps: f64, pub slip_ticks: f64, pub taxes: Vec<(String, f64, TaxSide)>, pub borrow_pct_year: Option<f64> }
/// AmiBroker Analysis settings › General / Portfolio position limits (money mode only).
/// `shrink` (AmiBroker "Allow position size shrinking"): an entry that a max_value / max_volume_pct cap or the cash on hand cuts
/// takes the smaller size; off (the default, as in AmiBroker), it is skipped. Without `execution.limits` the ledger uses
/// [`Limits::none`], today's behaviour.
#[derive(Clone, Copy, Debug, PartialEq)]
/// `margin_pct` (AmiBroker "Account margin", 100 = no margin): a cash account's buying power is equity × 100 / margin_pct.
/// `margin_rate_pct` (AmiBroker "Margin rate"): annual interest charged on the part of a long cash position the account borrows.
pub struct Limits { pub min_shares: f64, pub round_lot: f64, pub min_value: f64, pub max_value: Option<f64>, pub max_volume_pct: Option<f64>, pub shrink: bool, pub prev_bar_equity: bool, pub interest_pct: f64, pub margin_pct: f64, pub margin_rate_pct: f64 }
impl Default for Limits {
    fn default() -> Self { Limits { min_shares: 0., round_lot: 1., min_value: 0., max_value: None, max_volume_pct: None, shrink: false, prev_bar_equity: false, interest_pct: 0., margin_pct: 100., margin_rate_pct: 0. } }
}
impl Limits {
    /// No `execution.limits`: no caps, and a cash shortfall shrinks the order to the affordable lots.
    pub fn none() -> Self { Limits { shrink: true, ..Limits::default() } }
}
pub const MAX_VALID_BARS: usize = 20;
pub const MAX_DELAY: usize = 10;
pub const MAX_POINTS: f64 = 1e9;
/// What the other side's entry does while a position is open (`direction: both`, spec §6.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opposite { Reverse, Exit, Ignore }
#[derive(Clone, Debug, PartialEq)]
pub struct Exec {
    pub short: bool, pub fill: Fill, pub exit_fill: Fill, pub limit: Option<Dist>, pub stop_loss: Option<Dist>, pub target: Option<Dist>, pub trailing: Option<Dist>, pub opposite: Opposite,
    /// Phase 7 orders (spec §6.1): a stop entry, absolute order levels from operands (`at`), how many bars an order rests,
    /// the next-open delay in bars, and edge-only entries.
    pub stop: Option<Dist>, pub limit_at: Option<Operand>, pub stop_at: Option<Operand>, pub valid_bars: usize, pub delay: usize, pub edge: bool,
    /// Phase 7 exits (spec §6.1): trailing activation, breakeven (after, plus), partial take-profits (level, fraction of the
    /// initial position), the bars before rule exits count, and the TradingView intrabar path instead of stop-first.
    pub trail_activate: Option<Dist>, pub breakeven: Option<(Dist, Option<Dist>)>, pub take_profits: Vec<(Dist, f64)>, pub min_bars: usize, pub ohlc_path: bool,
    pub scale_in: Option<ScaleIn>, pub sizing: Sizing,
    /// re-entry rules: cooldown (bars, after which exits), at most `count` entries per calendar period (anchor 1 W … 4 Y), risk
    pub cooldown: Option<(usize, After)>, pub max_trades: Option<(usize, usize)>, pub risk: Risk,
    /// money mode (spec §7.4): starting capital and declared costs
    pub capital: Option<f64>, pub costs: MoneyCosts, pub futures: Option<Futures>, pub limits: Option<Limits>,
    /// AmiBroker trade prices and delays of this side's market orders, [entry, exit] (buy/sell long, short/cover short);
    /// None: fill, exit_fill and delay. Re-entry delay in bars after a stop_loss, target, trailing or bars_held exit (same side).
    pub market: Option<[(Px, usize); 2]>, pub reentry: Option<[usize; 4]>,
    /// AmiBroker Stops (shared by both sides): ExitAtStop 0/1/2 of stop_loss, target, trailing and the N-bar stop (1: today's),
    /// "Activate stops immediately" (true: today's), N-bar "Has priority", General › "Allow same bar exit / entry signal" (true: today's).
    pub exit_at_stop: [u8; 4], pub stops_immediately: bool, pub nbar_first: bool, pub same_bar: bool,
    /// AmiBroker General › "Tick size" of a cash instrument (0: no rounding; ledger::snap_ticks) and "Pad and align all data to
    /// reference symbol" (the market benchmark's calendar). Run-level: read from the main side only.
    pub tick_size: f64, pub pad_align: bool,
    /// `execution.bars_held_from` "signal": bars_held counts from the entry signal bar, not the fill bar (AmiBroker AFL
    /// BarsSince(Buy) with a next-open fill). Same as the fill bar for close fills.
    pub held_from_signal: bool,
}
impl Default for Exec {
    fn default() -> Self {
        Exec { short: false, fill: Fill::NextOpen, exit_fill: Fill::NextOpen, limit: None, stop_loss: None, target: None, trailing: None, opposite: Opposite::Reverse,
               stop: None, limit_at: None, stop_at: None, valid_bars: 1, delay: 1, edge: false,
               trail_activate: None, breakeven: None, take_profits: vec![], min_bars: 0, ohlc_path: false, scale_in: None, sizing: Sizing::Full,
               cooldown: None, max_trades: None, risk: Risk::default(), capital: None, costs: MoneyCosts::default(), futures: None, limits: None, market: None, reentry: None,
               exit_at_stop: [1; 4], stops_immediately: true, nbar_first: false, same_bar: true,
               tick_size: 0., pad_align: false, held_from_signal: false }
    }
}
impl Exec {
    pub fn slot(&self, s: Slot) -> Option<Dist> {
        match s {
            Slot::Limit => self.limit, Slot::StopLoss => self.stop_loss, Slot::Target => self.target, Slot::Trailing => self.trailing, Slot::Stop => self.stop,
            Slot::Activate => self.trail_activate, Slot::BreakevenAfter => self.breakeven.map(|b| b.0), Slot::TakeProfit(k) => self.take_profits.get(k).map(|t| t.0),
        }
    }
    fn slot_mut(&mut self, s: Slot) -> Option<&mut Dist> {
        match s {
            Slot::Limit => self.limit.as_mut(), Slot::StopLoss => self.stop_loss.as_mut(), Slot::Target => self.target.as_mut(), Slot::Trailing => self.trailing.as_mut(),
            Slot::Stop => self.stop.as_mut(), Slot::Activate => self.trail_activate.as_mut(), Slot::BreakevenAfter => self.breakeven.as_mut().map(|b| &mut b.0),
            Slot::TakeProfit(k) => self.take_profits.get_mut(k).map(|t| &mut t.0),
        }
    }
    fn put(&mut self, s: Slot, d: Dist) {
        match s { Slot::Limit => self.limit = Some(d), Slot::StopLoss => self.stop_loss = Some(d), Slot::Target => self.target = Some(d), Slot::Trailing => self.trailing = Some(d), Slot::Stop => self.stop = Some(d), _ => {} }
    }
    /// A limit, stop or stop-limit entry order (the entry rests instead of filling at market).
    pub fn has_order(&self) -> bool { self.limit.is_some() || self.stop.is_some() || self.limit_at.is_some() || self.stop_at.is_some() }
}
/// Lineage of a customized book template (`origin` in the spec). Checked against the template catalogue by the API.
#[derive(Clone, Debug, PartialEq)]
pub struct Origin { pub book: String, pub method: String, pub version: u32 }
/// Slot, spec path, largest pct. A limit may sit at the signal close (0%); stop-loss, target and trailing need pct > 0.
const SLOTS: [(Slot, &str, f64); 5] = [(Slot::Limit, "entry.limit", 50.), (Slot::StopLoss, "exit.stop_loss", 99.), (Slot::Target, "exit.target", 1000.), (Slot::Trailing, "exit.trailing", 99.), (Slot::Stop, "entry.stop", 50.)];
pub const MAX_ATR_MULT: f64 = 50.;
pub const MAX_ATR_LENGTH: usize = 500;
#[derive(Clone, Debug, PartialEq)]
pub struct Dim { pub path: String, pub target: Target, pub values: Vec<f64> }
#[derive(Clone, Debug, PartialEq)]
pub struct Spec { pub entry: Vec<Node>, pub exit: Vec<Node>, pub cost_bps: Option<f64>, pub optimize: Vec<Dim>, pub exec: Exec, pub origin: Option<Origin>, pub short_side: Option<Box<Side>>, pub portfolio: Option<Portfolio>,
    /// AmiBroker Settings › Report risk-free rates (Sharpe, UPI) in percent from `execution.report` (lab/report.rs rates); None: defaults.
    pub report: Option<(f64, f64)> }
/// The short side of a `direction: both` spec: its own rule lists and price orders (exec.short = true).
#[derive(Clone, Debug, PartialEq)]
pub struct Side { pub entry: Vec<Node>, pub exit: Vec<Node>, pub exec: Exec }

pub fn parse_spec(text: &str) -> Result<Spec, LabError> {
    if text.len() > MAX_SPEC_BYTES { return Err(inv(format!("spec is {} bytes; the limit is {MAX_SPEC_BYTES}", text.len()))); }
    let root = json::parse(text).map_err(|e| inv(format!("spec is not valid JSON: {e}")))?;
    fields(&root, "spec", &["v", "direction", "origin", "entry", "exit", "short_side", "execution", "optimize", "portfolio"], &["short", "scale_in"])?;
    if root.get("v").and_then(Json::as_f64) != Some(1.) { return Err(inv("v must be 1")); }
    let mut exec = Exec::default();
    let both = match root.get("direction") {
        None => false,
        Some(Json::Str(d)) if d == "long" => false,
        Some(Json::Str(d)) if d == "short" => { exec.short = true; false }
        Some(Json::Str(d)) if d == "both" => true,
        _ => return Err(inv("direction must be \"long\", \"short\" or \"both\"")),
    };
    let origin = root.get("origin").map(origin).transpose()?;
    let mut budget = Budget::default();
    let portfolio = root.get("portfolio").map(|v|portfolio(v,&mut budget)).transpose()?;
    let rotation = portfolio.as_ref().is_some_and(|p|p.mode==Mode::Rotation);
    let (entry, exit) = side(root.get("entry"), root.get("exit"), "", both, rotation, &mut exec, &mut budget)?;
    let mut short_side = match (both, root.get("short_side")) {
        (true, Some(s)) => {
            fields(s, "short_side", &["entry", "exit"], &[])?;
            let mut x = Exec { short: true, ..Exec::default() };
            let (entry, exit) = side(s.get("entry"), s.get("exit"), "short_side.", true, false, &mut x, &mut budget)?;
            Some(Box::new(Side { entry, exit, exec: x }))
        }
        (true, None) => return Err(inv("direction \"both\" needs short_side with the short entry and exit rules")),
        (false, Some(_)) => return Err(inv("short_side applies to direction \"both\" only")),
        (false, None) => None,
    };
    let mut cost_bps = None;
    if let Some(ex) = root.get("execution") {
        fields(ex, "execution", &["fill", "exit_fill", "cost_bps", "opposite", "delay_bars", "entry_on", "intrabar", "sizing", "cooldown", "max_trades", "risk", "capital", "costs", "instrument", "prices", "delays", "reentry", "limits", "exit_at_stop", "activate_stops_immediately", "nbar_has_priority", "allow_same_bar_exit", "report", "tick_size", "pad_align", "bars_held_from"], &["limit", "stop"])?;
        if let Some(v) = ex.get("instrument") { exec.futures = instrument(v)?; }
        if let Some(c) = ex.get("capital") {
            exec.capital = Some(c.as_f64().filter(|c| c.is_finite() && *c > 0. && *c <= MAX_CAPITAL).ok_or_else(|| inv(format!("execution.capital must be a number above 0 and at most {MAX_CAPITAL}")))?);
            if ex.get("cost_bps").is_some() { return Err(inv("money mode takes costs from execution.costs; remove execution.cost_bps")); }
        }
        if let Some(v) = ex.get("costs") {
            if exec.capital.is_none() { return Err(inv("execution.costs applies to money mode (execution.capital)")); }
            exec.costs = money_costs(v)?;
        }
        if let Some(v) = ex.get("limits") {
            if exec.capital.is_none() { return Err(inv("execution.limits needs money mode (execution.capital)")); }
            exec.limits = Some(limits(v)?);
        }
        if let Some(v) = ex.get("tick_size") {
            exec.tick_size = v.as_f64().filter(|t| t.is_finite() && (0. ..=1e6).contains(t)).ok_or_else(|| inv("execution.tick_size must be a number from 0 (no rounding) to 1e6"))?;
            if exec.tick_size > 0. && exec.futures.is_some() { return Err(inv("futures round to the contract's tick; remove execution.tick_size")); }
        }
        if let Some(v) = ex.get("pad_align") { exec.pad_align = match v { Json::Bool(b) => *b, _ => return Err(inv("execution.pad_align must be true or false")) }; }
        if let Some(v) = ex.get("cooldown") {
            fields(v, "execution.cooldown", &["bars", "after"], &[])?;
            let after = match v.get("after").map(Json::as_str) {
                None | Some(Some("any")) => After::Any, Some(Some("loss")) => After::Loss, Some(Some("win")) => After::Win,
                _ => return Err(inv("execution.cooldown.after must be \"any\", \"loss\" or \"win\"")),
            };
            exec.cooldown = Some((whole(v.get("bars"), "execution.cooldown.bars", 1)?, after));
        }
        if let Some(v) = ex.get("max_trades") {
            fields(v, "execution.max_trades", &["count", "per"], &[])?;
            let per = match v.get("per").and_then(Json::as_str) { Some("W") => 1, Some("M") => 2, Some("Q") => 3, Some("Y") => 4, _ => return Err(inv("execution.max_trades.per must be W, M, Q or Y")) };
            exec.max_trades = Some((whole(v.get("count"), "execution.max_trades.count", 1)?, per));
        }
        if let Some(v) = ex.get("risk") {
            fields(v, "execution.risk", &["max_consecutive_losses", "pause_bars", "max_drawdown_pct"], &[])?;
            if let Some(n) = v.get("max_consecutive_losses") {
                let pause = v.get("pause_bars").and_then(Json::as_f64).filter(|p| p.fract() == 0. && *p >= 1. && *p <= MAX_PAUSE as f64)
                    .ok_or_else(|| inv(format!("execution.risk.pause_bars must be a whole number from 1 to {MAX_PAUSE} (with max_consecutive_losses)")))? as usize;
                exec.risk.losses = Some((whole(Some(n), "execution.risk.max_consecutive_losses", 1)?, pause));
            } else if v.get("pause_bars").is_some() { return Err(inv("execution.risk.pause_bars goes with max_consecutive_losses")); }
            if let Some(d) = v.get("max_drawdown_pct") {
                exec.risk.max_dd_pct = Some(d.as_f64().filter(|d| *d > 0. && *d < 100.).ok_or_else(|| inv("execution.risk.max_drawdown_pct must be a number above 0 and below 100"))?);
            }
        }
        if let Some(z) = ex.get("sizing") {
            fields(z, "execution.sizing", &["pct_equity", "risk_pct", "units", "value"], &[])?;
            let pct = |v: &Json, k: &str| v.as_f64().filter(|p| *p > 0. && *p <= 100.).ok_or_else(|| inv(format!("execution.sizing.{k} must be a number above 0 and at most 100")));
            exec.sizing = match (z.get("pct_equity"), z.get("risk_pct"), z.get("units"), z.get("value")) {
                (Some(p), None, None, None) => Sizing::PctEquity(pct(p, "pct_equity")?),
                (None, Some(r), None, None) => {
                    if exec.stop_loss.is_none() || short_side.as_ref().is_some_and(|s| s.exec.stop_loss.is_none()) {
                        return Err(inv("execution.sizing.risk_pct needs an exit.stop_loss on every side (the risk is the stop distance)"));
                    }
                    Sizing::RiskPct(pct(r, "risk_pct")?)
                }
                (None, None, Some(n), None) if ex.get("capital").is_some() => Sizing::Units(n.as_f64().filter(|n| n.fract() == 0. && *n >= 1. && *n <= 1e9).ok_or_else(|| inv("execution.sizing.units must be a whole number from 1 (shares, or lots for futures)"))?),
                (None, None, None, Some(v)) if ex.get("capital").is_some() => Sizing::Value(v.as_f64().filter(|v| *v > 0. && *v <= MAX_CAPITAL).ok_or_else(|| inv("execution.sizing.value must be a number above 0"))?),
                (None, None, Some(_), None) | (None, None, None, Some(_)) => return Err(inv("execution.sizing units and value need execution.capital (money mode)")),
                _ => return Err(inv("execution.sizing needs exactly one of pct_equity, risk_pct, units or value")),
            };
        }
        exec.ohlc_path = match ex.get("intrabar").map(Json::as_str) {
            None | Some(Some("stop_first")) => false,
            Some(Some("ohlc_path")) => true,
            _ => return Err(inv("execution.intrabar must be \"stop_first\" or \"ohlc_path\"")),
        };
        if let Some(f) = ex.get("fill") { exec.fill = fill(f, "execution.fill")?; exec.exit_fill = exec.fill; }
        if let Some(f) = ex.get("exit_fill") { exec.exit_fill = fill(f, "execution.exit_fill")?; }
        let ordered = exec.has_order() || short_side.as_ref().is_some_and(|s| s.exec.has_order());
        if ordered && ex.get("fill").is_some() {
            return Err(inv("execution.fill applies to market entries; entry.limit and entry.stop orders fill on the bars after the signal, so remove execution.fill"));
        }
        if let Some(n) = ex.get("delay_bars") {
            if exec.fill != Fill::NextOpen || exec.exit_fill != Fill::NextOpen { return Err(inv("execution.delay_bars applies to next_open fills only")); }
            exec.delay = n.as_f64().filter(|n| n.fract() == 0. && (1. ..=MAX_DELAY as f64).contains(n))
                .ok_or_else(|| inv(format!("execution.delay_bars must be a whole number from 1 to {MAX_DELAY}")))? as usize;
        }
        if ex.get("prices").is_some() || ex.get("delays").is_some() {
            exec.market = Some(market(ex, &exec, short_side.as_mut())?);
        }
        if let Some(v) = ex.get("reentry") {
            fields(v, "execution.reentry", &REENTRY, &[])?;
            let mut n = [0; 4];
            for (k, key) in REENTRY.iter().enumerate() { if let Some(b) = v.get(key) { n[k] = whole(Some(b), &format!("execution.reentry.{key}"), 0)?; } }
            exec.reentry = Some(n);
        }
        if let Some(v) = ex.get("exit_at_stop") {
            fields(v, "execution.exit_at_stop", &REENTRY, &[])?;
            for (k, key) in REENTRY.iter().enumerate() { if let Some(m) = v.get(key) { exec.exit_at_stop[k] = m.as_f64().filter(|m| [0., 1., 2.].contains(m)).ok_or_else(|| inv(format!("execution.exit_at_stop.{key} must be 0, 1 or 2")))? as u8; } }
        }
        let flag = |k: &str, default: bool| match ex.get(k) { None => Ok(default), Some(Json::Bool(b)) => Ok(*b), _ => Err(inv(format!("execution.{k} must be true or false"))) };
        (exec.stops_immediately, exec.nbar_first, exec.same_bar) = (flag("activate_stops_immediately", true)?, flag("nbar_has_priority", false)?, flag("allow_same_bar_exit", true)?);
        exec.held_from_signal = match ex.get("bars_held_from").map(Json::as_str) {
            None | Some(Some("fill")) => false,
            Some(Some("signal")) => true,
            _ => return Err(inv("execution.bars_held_from must be \"fill\" or \"signal\"")),
        };
        exec.edge = match ex.get("entry_on").map(Json::as_str) {
            None | Some(Some("level")) => false,
            Some(Some("edge")) => true,
            _ => return Err(inv("execution.entry_on must be \"level\" or \"edge\"")),
        };
        if let Some(c) = ex.get("cost_bps") {
            cost_bps = Some(c.as_f64().filter(|c| (0. ..=5000.).contains(c)).ok_or_else(|| inv("execution.cost_bps must be a number from 0 to 5000"))?);
        }
        if let Some(o) = ex.get("opposite") {
            if !both { return Err(inv("execution.opposite applies to direction \"both\" only")); }
            exec.opposite = match o.as_str() {
                Some("reverse") => Opposite::Reverse, Some("exit") => Opposite::Exit, Some("ignore") => Opposite::Ignore,
                _ => return Err(inv("execution.opposite must be \"reverse\", \"exit\" or \"ignore\"")),
            };
        }
    }
    if let Some(s) = short_side.as_mut() {
        (s.exec.fill, s.exec.exit_fill, s.exec.opposite, s.exec.delay, s.exec.edge, s.exec.ohlc_path) = (exec.fill, exec.exit_fill, exec.opposite, exec.delay, exec.edge, exec.ohlc_path);
        (s.exec.exit_at_stop, s.exec.stops_immediately, s.exec.nbar_first, s.exec.same_bar) = (exec.exit_at_stop, exec.stops_immediately, exec.nbar_first, exec.same_bar);
    }
    // An exitless side relies on the opposite entry to reverse its position.
    let exitless = |x: &Exec, e: &[Node]| e.is_empty() && x.stop_loss.is_none() && x.target.is_none() && x.trailing.is_none() && x.breakeven.is_none();
    if both && exec.opposite != Opposite::Reverse && (exitless(&exec,&exit) || short_side.as_ref().is_some_and(|s|exitless(&s.exec,&s.exit))) {
        return Err(inv("a side without exit rules (always in) needs execution.opposite \"reverse\""));
    }
    let execs: Vec<&Exec> = std::iter::once(&exec).chain(short_side.as_ref().map(|s| &s.exec)).collect();
    if exec.futures.is_none() && execs.iter().any(|x| distances(x).iter().any(|d| matches!(d, Dist::Ticks(_)))) {
        return Err(inv("ticks need a futures contract's tick size (execution.instrument kind futures)"));
    }
    if exec.futures.is_some() && execs.iter().any(|x| x.scale_in.is_some() || !x.take_profits.is_empty()) {
        return Err(inv("futures runs do not combine with scale_in or take_profits yet"));
    }
    let report = root.get("execution").and_then(|x| x.get("report")).map(super::report::rates).transpose()?;
    if report.is_some() && portfolio.is_some() { return Err(LabError::Unsupported("execution.report applies to single-instrument runs; a portfolio run has no AmiBroker report".into())); }
    let mut spec = Spec { entry, exit, cost_bps, optimize: vec![], exec, origin, short_side, portfolio, report };
    validate_applied(&spec)?;
    if let Some(o) = root.get("optimize") { spec.optimize = dims(&spec, o)?; }
    Ok(spec)
}

const REENTRY: [&str; 4] = ["stop_loss", "target", "trailing", "bars"];
/// `execution.prices` / `execution.delays` (AmiBroker Analysis settings › Trades): each order fills on bar signal + delay at
/// that bar's field. A missing key keeps today's default (open, 1 bar); delay 0 is the signal bar, known only at its close.
/// Sets the short side's [short, cover] and returns this side's [buy, sell] (or [short, cover] for direction short).
fn market(ex: &Json, exec: &Exec, short_side: Option<&mut Box<Side>>) -> Result<[(Px, usize); 2], LabError> {
    const KEYS: [&str; 4] = ["buy", "sell", "short", "cover"];
    const NAMES: [(&str, Px); 5] = [("open", Px::Open), ("high", Px::High), ("low", Px::Low), ("close", Px::Close), ("average", Px::Average)];
    if ["fill", "exit_fill", "delay_bars"].iter().any(|k| ex.get(k).is_some()) {
        return Err(inv("execution.prices and execution.delays replace execution.fill, exit_fill and delay_bars; give one form"));
    }
    let (prices, delays) = (ex.get("prices"), ex.get("delays"));
    // only the sides this spec trades: buy/sell for long, short/cover for short, all four for both
    let used: &[&str] = if short_side.is_some() { &KEYS } else if exec.short { &KEYS[2..] } else { &KEYS[..2] };
    for (v, at) in [(prices, "execution.prices"), (delays, "execution.delays")] {
        let Some(v) = v else { continue };
        fields(v, at, &KEYS, &[])?;
        if let Some(key) = KEYS.iter().find(|k| !used.contains(k) && v.get(k).is_some()) {
            return Err(inv(format!("{at}.{key} applies to direction {} or both", if KEYS[..2].contains(key) { "long" } else { "short" })));
        }
    }
    let mut m = [(Px::Open, 1); 4];
    for (k, key) in KEYS.iter().enumerate().filter(|(_, k)| used.contains(k)) {
        if let Some(p) = prices.and_then(|v| v.get(key)) {
            m[k].0 = NAMES.iter().find(|n| p.as_str() == Some(n.0)).map(|n| n.1)
                .ok_or_else(|| inv(format!("execution.prices.{key} must be \"open\", \"high\", \"low\", \"close\" or \"average\"")))?;
        }
        if let Some(n) = delays.and_then(|v| v.get(key)) {
            m[k].1 = n.as_f64().filter(|n| n.fract() == 0. && (0. ..=MAX_TRADE_DELAY as f64).contains(n))
                .ok_or_else(|| inv(format!("execution.delays.{key} must be a whole number from 0 to {MAX_TRADE_DELAY}")))? as usize;
        }
        if m[k].1 == 0 && m[k].0 != Px::Close {
            let name = NAMES.iter().find(|n| n.1 == m[k].0).unwrap().0;
            return Err(inv(format!("execution.prices.{key} \"{name}\" with delay 0 would use prices before the signal (a signal is known at the bar's close); use \"close\" or a delay of 1 or more")));
        }
    }
    // entry orders (limit / stop) fill by their own rules: their side's entry price and delay do not apply
    let given = |key: &str| prices.and_then(|v| v.get(key)).or(delays.and_then(|v| v.get(key))).is_some();
    let entry_key = if exec.short { "short" } else { "buy" };
    for (ordered, key) in [(exec.has_order(), entry_key), (short_side.as_ref().is_some_and(|s| s.exec.has_order()), "short")] {
        if ordered && given(key) { return Err(inv(format!("execution.prices.{key} / execution.delays.{key} apply to market entries; entry.limit and entry.stop orders fill on the bars after the signal"))); }
    }
    if let Some(s) = short_side { s.exec.market = Some([m[2], m[3]]); }
    Ok(if exec.short { [m[2], m[3]] } else { [m[0], m[1]] })
}

/// One side's rule lists and price orders; `pre` is "" (the long side, or the only side) or "short_side.".
fn side(entry: Option<&Json>, exit: Option<&Json>, pre: &str, both: bool, rotation: bool, exec: &mut Exec, b: &mut Budget) -> Result<(Vec<Node>, Vec<Node>), LabError> {
    let entry_obj = entry.ok_or_else(|| inv(format!("{pre}entry is required")))?;
    fields(entry_obj, &format!("{pre}entry"), &["all", "limit", "stop", "valid_bars", "scale_in"], &[])?;
    if let Some(v) = entry_obj.get("scale_in") { exec.scale_in = Some(scale_in(v, &format!("{pre}entry.scale_in"))?); }
    let exit_obj = exit.ok_or_else(|| inv(format!("{pre}exit is required")))?;
    fields(exit_obj, &format!("{pre}exit"), &["any", "stop_loss", "target", "trailing", "breakeven", "take_profits", "min_bars"], &[])?;
    for (slot, at, hi) in SLOTS {
        let order = matches!(slot, Slot::Limit | Slot::Stop);
        let Some(v) = (if order { entry_obj } else { exit_obj }).get(at.split_once('.').unwrap().1) else { continue };
        if slot == Slot::Trailing {
            if let Some(a) = v.get("activate") { exec.trail_activate = Some(dist(a, &format!("{pre}exit.trailing.activate"), 1000.)?); }
            let d = without(v, "activate");
            if let Some(p) = d.get("profit_pct") {
                fields(&d, &format!("{pre}{at}"), &["profit_pct"], &[])?;
                exec.trailing = Some(Dist::ProfitPct(p.as_f64().filter(|p| *p > 0. && *p < 100.).ok_or_else(|| inv(format!("{pre}{at}.profit_pct must be a number above 0 and below 100")))?));
                continue;
            }
            exec.put(slot, dist(&d, &format!("{pre}{at}"), hi)?);
            continue;
        }
        if v.get("at").is_none() { exec.put(slot, dist(v, &format!("{pre}{at}"), hi)?); continue; }
        // an absolute level: the operand's value at the signal bar (entry orders only)
        if !order { return Err(inv(format!("{pre}{at}.at is for entry orders (entry.limit, entry.stop)"))); }
        fields(v, &format!("{pre}{at}"), &["at"], &[])?;
        let o = operand(v.get("at"), &format!("{pre}{at}.at"), false, b)?;
        if slot == Slot::Limit { exec.limit_at = Some(o); } else { exec.stop_at = Some(o); }
    }
    if let Some(n) = entry_obj.get("valid_bars") {
        if !exec.has_order() { return Err(inv(format!("{pre}entry.valid_bars applies to entry.limit or entry.stop orders"))); }
        exec.valid_bars = n.as_f64().filter(|n| n.fract() == 0. && (1. ..=MAX_VALID_BARS as f64).contains(n))
            .ok_or_else(|| inv(format!("{pre}entry.valid_bars must be a whole number from 1 to {MAX_VALID_BARS}")))? as usize;
    }
    if let Some(v) = exit_obj.get("breakeven") {
        let at = format!("{pre}exit.breakeven");
        fields(v, &at, &["after", "plus"], &[])?;
        let after = dist(v.get("after").ok_or_else(|| inv(format!("{at}.after is required")))?, &format!("{at}.after"), 1000.)?;
        if v.get("plus").is_some_and(|p| p.get("at").is_some()) { return Err(inv(format!("{at}.plus takes pct|atr|points; `at` levels are not supported"))); }
        let plus = v.get("plus").map(|p| dist(p, &format!("{at}.plus"), 1000.)).transpose()?;
        breakeven_order(after, plus, &at).map_err(inv)?;
        exec.breakeven = Some((after, plus));
    }
    if let Some(v) = exit_obj.get("take_profits") {
        let at = format!("{pre}exit.take_profits");
        let Json::Arr(items) = v else { return Err(inv(format!("{at} must be a list"))) };
        if items.is_empty() || items.len() > MAX_TAKE_PROFITS { return Err(inv(format!("{at} needs 1 to {MAX_TAKE_PROFITS} levels"))); }
        for (k, it) in items.iter().enumerate() {
            let a = format!("{at}[{k}]");
            let size = it.get("size").and_then(Json::as_f64).filter(|s| *s > 0. && *s < 1.)
                .ok_or_else(|| inv(format!("{a}.size must be a fraction above 0 and below 1 of the initial position")))?;
            exec.take_profits.push((dist(&without(it, "size"), &a, 1000.)?, size));
        }
        if exec.take_profits.iter().map(|t| t.1).sum::<f64>() >= 1. { return Err(inv(format!("{at} sizes must add up to less than 1; the rest exits by the other rules"))); }
    }
    if let Some(n) = exit_obj.get("min_bars") { exec.min_bars = whole(Some(n), &format!("{pre}exit.min_bars"), 0)?; }
    if exec.scale_in.is_some() && !exec.take_profits.is_empty() {
        return Err(inv(format!("{pre}entry.scale_in cannot combine with {pre}exit.take_profits (a trade's scale-out fractions are of one position)")));
    }
    let price_exit = exec.stop_loss.is_some() || exec.target.is_some() || exec.trailing.is_some() || exec.breakeven.is_some();
    let entry = conds(entry_obj.get("all"), &format!("{pre}entry.all"), false, rotation, b)?;
    let exit = conds(exit_obj.get("any"), &format!("{pre}exit.any"), true, price_exit || both || rotation, b)?;
    if entry.len() + exit.len() > MAX_CONDITIONS {
        return Err(inv(format!("{}{} conditions; the limit is {MAX_CONDITIONS}", if pre.is_empty() { "" } else { "short_side: " }, entry.len() + exit.len())));
    }
    Ok((entry, exit))
}

/// The same validation is used by JSON leaves and chart query fields.
pub fn foreign_ref(at: &str, symbol: Option<&str>, vs: Option<&str>, fixup: Option<&str>) -> Result<Foreign,LabError> {
    let name=|key:&str,value:Option<&str>|value.map(|s| {
        if s.trim().is_empty() || s.len()>MAX_SYMBOL_LEN || s.chars().any(char::is_control) {Err(inv(format!("{at}.{key} must name an instrument (1 to {MAX_SYMBOL_LEN} characters)")))} else {Ok(s.to_string())}
    }).transpose();
    let (symbol,vs)=(name("symbol",symbol)?,name("vs",vs)?);
    let previous=match fixup {None|Some("none")=>false,Some("previous")=>true,_=>return Err(inv(format!("{at}.fixup must be none or previous")))};
    if fixup.is_some()&&symbol.is_none(){return Err(inv(format!("{at}.fixup applies to a symbol operand")));}
    Ok(Foreign{symbol,vs,fixup:previous})
}
fn portfolio(v:&Json,budget:&mut Budget)->Result<Portfolio,LabError> {
    fields(v,"portfolio",&["universe","mode","score","rank","max_open","worst_rank_held","rebalance"],&[])?;
    let u=v.get("universe").ok_or_else(||inv("portfolio.universe is required"))?;
    fields(u,"portfolio.universe",&["basket","tickers"],&[])?;
    let universe=match (u.get("basket"),u.get("tickers")) {
        (Some(b),None)=>Universe::Basket(b.as_str().filter(|s|!s.is_empty()&&s.len()<=40&&s.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'_'||c==b'-')).ok_or_else(||inv("portfolio.universe.basket must be a basket id (letters, digits, _ or -)"))?.into()),
        (None,Some(Json::Arr(items)))=>{
            if !(2..=MAX_UNIVERSE_LIST).contains(&items.len()){return Err(inv(format!("portfolio.universe.tickers needs 2 to {MAX_UNIVERSE_LIST} instruments (at most {MAX_UNIVERSE_LIST})")));}
            let mut names=Vec::new();
            for (i,t) in items.iter().enumerate(){
                let value=t.as_str().ok_or_else(||inv(format!("portfolio.universe.tickers[{i}] must name an instrument")))?;
                let name=foreign_ref(&format!("portfolio.universe.tickers[{i}]"),Some(value),None,None)?.symbol.unwrap();
                if names.contains(&name){return Err(inv(format!("portfolio.universe.tickers[{i}] {name} is listed twice")));}names.push(name);
            }
            Universe::Tickers(names)
        }
        _=>return Err(inv("portfolio.universe needs exactly one of basket or tickers")),
    };
    let mode=match v.get("mode").map(Json::as_str){None|Some(Some("signals"))=>Mode::Signals,Some(Some("rotation"))=>Mode::Rotation,_=>return Err(inv("portfolio.mode must be signals or rotation"))};
    let lowest=match v.get("rank").map(Json::as_str){None|Some(Some("highest"))=>false,Some(Some("lowest"))=>true,_=>return Err(inv("portfolio.rank must be highest or lowest"))};
    let number=|key:&str,hi:usize|v.get(key).map(|n|n.as_f64().filter(|n|n.is_finite()&&n.fract()==0.&&*n>=1.&&*n<=hi as f64).map(|n|n as usize).ok_or_else(||inv(format!("portfolio.{key} must be a whole number from 1 to {hi}")))).transpose();
    let max_open=number("max_open",MAX_OPEN)?.unwrap_or(1);let worst_rank_held=number("worst_rank_held",MAX_UNIVERSE)?;
    if worst_rank_held.is_some()&&mode!=Mode::Rotation {return Err(inv("portfolio.worst_rank_held applies to rotation mode"));}
    let rebalance=match v.get("rebalance").map(Json::as_str){None|Some(Some("bar"))=>None,Some(Some("W"))=>Some(1),Some(Some("M"))=>Some(2),Some(Some("Q"))=>Some(3),Some(Some("Y"))=>Some(4),_=>return Err(inv("portfolio.rebalance must be bar, W, M, Q or Y"))};
    if rebalance.is_some()&&mode!=Mode::Rotation{return Err(inv("portfolio.rebalance applies to rotation mode"));}
    let score=match v.get("score"){
        None=>None,
        Some(v)=>{
            budget.take(false)?;
            let mut score=operand(Some(v),"portfolio.score",false,budget)?;
            if matches!(score,Operand::Value(_)|Operand::BarsHeld){return Err(inv("portfolio.score needs a series or expression, not a constant or bars_held"));}
            let (mut symbol,mut has_series)=(None,false);
            walk_operand(&mut score,"portfolio.score",&mut |path,k|if let Knob::Series(r)=k {has_series=true;if r.foreign.symbol.is_some(){symbol=Some(path);}});
            if !has_series{return Err(inv("portfolio.score needs at least one series, including inside state conditions"));}
            if let Some(path)=symbol{return Err(inv(format!("{path}: portfolio.score cannot read another symbol; vs is allowed")));}
            Some(score)
        }
    };
    if mode==Mode::Rotation&&score.is_none(){return Err(inv("rotation needs portfolio.score"));}
    Ok(Portfolio{universe,mode,score,lowest,max_open,worst_rank_held,rebalance})
}
fn portfolio_rules(s:&Spec)->Result<(),LabError>{
    let Some(p)=&s.portfolio else{return Ok(())};
    let unsupported=|field:&str|Err(LabError::Unsupported(format!("{field} in a portfolio is not supported by the declared execution model")));
    if s.origin.is_some(){return unsupported("origin");}
    if s.exec.short||s.short_side.is_some(){return unsupported("portfolio shorts or direction both");}
    let x=&s.exec;
    if x.limit.is_some()||x.limit_at.is_some(){return unsupported("entry.limit");}
    if x.stop.is_some()||x.stop_at.is_some(){return unsupported("entry.stop");}
    if x.scale_in.is_some(){return unsupported("entry.scale_in");}
    if !x.take_profits.is_empty(){return unsupported("exit.take_profits");}
    if x.cooldown.is_some(){return unsupported("execution.cooldown");}
    if x.max_trades.is_some(){return unsupported("execution.max_trades");}
    if x.risk!=Risk::default(){return unsupported("execution.risk");}
    if x.futures.is_some(){return unsupported("execution.instrument futures");}
    if x.delay!=1{return unsupported("execution.delay_bars");}
    if x.market.is_some(){return unsupported("execution.prices / execution.delays");}
    if x.reentry.is_some(){return unsupported("execution.reentry");}
    if x.exit_at_stop!=[1;4]||!x.stops_immediately||x.nbar_first||!x.same_bar{return unsupported("execution.exit_at_stop, activate_stops_immediately, nbar_has_priority or allow_same_bar_exit");}
    if x.limits.is_some(){return unsupported("execution.limits");}
    if x.tick_size>0.{return unsupported("execution.tick_size");}
    if x.pad_align{return unsupported("execution.pad_align");}
    if x.held_from_signal{return unsupported("execution.bars_held_from");}
    if x.capital.is_none(){return Err(inv("a portfolio needs execution.capital (shared money mode)"));}
    if p.max_open==0||p.max_open>MAX_OPEN{return Err(inv(format!("portfolio.max_open must be a whole number from 1 to {MAX_OPEN}")));}
    if p.worst_rank_held()==0||p.worst_rank_held()>MAX_UNIVERSE||p.worst_rank_held()<p.max_open{return Err(inv(format!("portfolio.worst_rank_held must be at least max_open and at most {MAX_UNIVERSE}")));}
    if p.mode==Mode::Rotation&&(!s.exit.is_empty()||x.stop_loss.is_some()||x.target.is_some()||x.trailing.is_some()||x.trail_activate.is_some()||x.breakeven.is_some()) {return Err(inv("rotation ranks by score; exit.any must be empty and price exits are not used"));}
    Ok(())
}
impl Spec {
    pub fn foreign_locations(&self)->Vec<(String,Foreign)>{
        let mut out=Vec::new();walk(&mut self.clone(),&mut |path,k|if let Knob::Series(r)=k {if r.foreign!=Foreign::default(){out.push((path,r.foreign.clone()));}});out
    }
    pub fn foreign_symbols(&self)->Vec<String>{
        let mut names:Vec<_>=self.foreign_locations().into_iter().flat_map(|(_,f)|f.symbol.into_iter().chain(f.vs)).collect();names.sort();names.dedup();names
    }
    pub fn fixup_symbols(&self)->std::collections::BTreeSet<String>{self.foreign_locations().into_iter().filter(|(_,f)|f.fixup).filter_map(|(_,f)|f.symbol).collect()}
}
/// Recheck applied portfolio candidates before expensive member work; an invalid combination is never ranked.
pub fn validate_applied(s:&Spec)->Result<(),LabError>{
    let foreign=s.foreign_symbols();if foreign.len()>MAX_FOREIGN{return Err(inv(format!("the spec reads {} other instruments ({}); the limit is {MAX_FOREIGN}",foreign.len(),foreign.join(", "))));}
    portfolio_rules(s)?;
    let short=s.short_side.as_ref().map(|x|x.entry.iter().chain(&x.exit));
    s.entry.iter().chain(&s.exit).chain(short.into_iter().flatten()).try_for_each(consistent).map_err(inv)?;
    let execs=std::iter::once(&s.exec).chain(s.short_side.as_ref().map(|x|&x.exec));
    execs.flat_map(|x|[&x.limit_at,&x.stop_at]).flatten().try_for_each(consistent_operand).map_err(inv)?;
    if let Some(score)=s.portfolio.as_ref().and_then(|p|p.score.as_ref()){consistent_operand(score).map_err(inv)?;}
    Ok(())
}

fn instrument(v: &Json) -> Result<Option<Futures>, LabError> {
    fields(v, "execution.instrument", &["kind", "roll", "adjust"], &[])?;
    if v.get("kind").is_some_and(|v|v.as_str().is_none()) {return Err(inv("execution.instrument.kind must be cash or futures"));}
    if v.get("adjust").is_some_and(|v|v.as_str().is_none()) {return Err(inv("execution.instrument.adjust must be forward, ratio or none"));}
    match v.get("kind").and_then(Json::as_str) {
        None | Some("cash") if v.get("roll").is_none() && v.get("adjust").is_none() => Ok(None),
        None | Some("cash") => Err(inv("execution.instrument roll and adjust apply to kind futures")),
        Some("futures") => {
            let roll = match v.get("roll") {
                None => Roll::DaysBefore(1),
                Some(Json::Str(s)) if s == "none" => Roll::None,
                Some(r) => {
                    fields(r, "execution.instrument.roll", &["days_before_expiry", "on"], &[])?;
                    match (r.get("days_before_expiry"), r.get("on").and_then(Json::as_str)) {
                        (Some(n), None) => Roll::DaysBefore(whole(Some(n), "execution.instrument.roll.days_before_expiry", 0)?),
                        (None, Some("volume")) => Roll::Volume,
                        (None, Some("open_interest")) => Roll::OpenInterest,
                        _ => return Err(inv("execution.instrument.roll must be none, days_before_expiry or on volume/open_interest")),
                    }
                }
            };
            let adjust = match v.get("adjust").and_then(Json::as_str) {
                None | Some("forward") => Adjust::Forward, Some("ratio") => Adjust::Ratio, Some("none") => Adjust::None,
                Some("back") => return Err(LabError::Unsupported("back adjustment rewrites earlier prices (lookahead); use forward, ratio or none".into())),
                _ => return Err(inv("execution.instrument.adjust must be forward, ratio or none")),
            };
            Ok(Some(Futures { roll, adjust }))
        }
        _ => Err(inv("execution.instrument.kind must be cash or futures")),
    }
}
fn distances(x: &Exec) -> Vec<Dist> {
    [x.limit,x.stop_loss,x.target,x.trailing,x.stop,x.trail_activate].into_iter().flatten()
        .chain(x.breakeven.iter().flat_map(|b|[Some(b.0),b.1]).flatten())
        .chain(x.take_profits.iter().map(|t|t.0)).collect()
}

fn limits(v: &Json) -> Result<Limits, LabError> {
    fields(v, "execution.limits", &["min_shares", "round_lot", "min_value", "max_value", "max_volume_pct", "shrink", "prev_bar_equity", "interest_pct", "margin_pct", "margin_rate_pct"], &[])?;
    let num = |k: &str, ok: &dyn Fn(f64) -> bool, want: &str| -> Result<Option<f64>, LabError> {
        v.get(k).map(|x| x.as_f64().filter(|x| x.is_finite() && ok(*x)).ok_or_else(|| inv(format!("execution.limits.{k} must be {want}")))).transpose()
    };
    let flag = |k: &str| match v.get(k) { None => Ok(false), Some(Json::Bool(b)) => Ok(*b), _ => Err(inv(format!("execution.limits.{k} must be true or false"))) };
    let d = Limits::default();
    let l = Limits {
        min_shares: num("min_shares", &|x| (0. ..=1e9).contains(&x), "a number from 0 to 1e9")?.unwrap_or(d.min_shares),
        round_lot: num("round_lot", &|x| x.fract() == 0. && (1. ..=1e9).contains(&x), "a whole number from 1 to 1e9")?.unwrap_or(d.round_lot),
        min_value: num("min_value", &|x| (0. ..=MAX_CAPITAL).contains(&x), "a number from 0 to 1e12")?.unwrap_or(d.min_value),
        // AmiBroker: 0 means no maximum / no volume limit
        max_value: num("max_value", &|x| (0. ..=MAX_CAPITAL).contains(&x), "a number from 0 (no maximum) to 1e12")?.filter(|x| *x > 0.),
        max_volume_pct: num("max_volume_pct", &|x| (0. ..=100.).contains(&x), "a number from 0 (no limit) to 100")?.filter(|x| *x > 0.),
        shrink: flag("shrink")?, prev_bar_equity: flag("prev_bar_equity")?,
        interest_pct: num("interest_pct", &|x| (0. ..=100.).contains(&x), "a number from 0 to 100")?.unwrap_or(d.interest_pct),
        margin_pct: num("margin_pct", &|x| (1. ..=100.).contains(&x), "a number from 1 to 100 (100 = no margin)")?.unwrap_or(d.margin_pct),
        margin_rate_pct: num("margin_rate_pct", &|x| (0. ..=100.).contains(&x), "a number from 0 to 100")?.unwrap_or(d.margin_rate_pct),
    };
    if l.max_value.is_some_and(|m| l.min_value > m) { return Err(inv("execution.limits.min_value must not exceed max_value")); }
    Ok(l)
}

fn money_costs(v: &Json) -> Result<MoneyCosts, LabError> {
    fields(v, "execution.costs", &["commission", "slippage", "taxes", "borrow_pct_year"], &[])?;
    let num = |v: Option<&Json>, at: &str, hi: f64| -> Result<f64, LabError> {
        v.map_or(Ok(0.), |x| x.as_f64().filter(|x| (0. ..=hi).contains(x)).ok_or_else(|| inv(format!("execution.costs.{at} must be a number from 0 to {hi}"))))
    };
    let mut c = MoneyCosts::default();
    if let Some(m) = v.get("commission") {
        fields(m, "execution.costs.commission", &["per_order", "per_unit", "pct"], &[])?;
        (c.per_order, c.per_unit, c.pct) = (num(m.get("per_order"), "commission.per_order", 1e9)?, num(m.get("per_unit"), "commission.per_unit", 1e6)?, num(m.get("pct"), "commission.pct", 10.)?);
    }
    if let Some(s) = v.get("slippage") {
        fields(s, "execution.costs.slippage", &["bps", "ticks"], &[])?;
        if s.get("bps").is_some() == s.get("ticks").is_some() { return Err(inv("execution.costs.slippage needs exactly one of bps or ticks")); }
        (c.slip_bps, c.slip_ticks) = (num(s.get("bps"), "slippage.bps", 1000.)?, num(s.get("ticks"), "slippage.ticks", 100.)?);
    }
    if let Some(t) = v.get("taxes") {
        let Json::Arr(items) = t else { return Err(inv("execution.costs.taxes must be a list")) };
        if items.len() > MAX_TAXES { return Err(inv(format!("execution.costs.taxes allows at most {MAX_TAXES} entries"))); }
        for (k, it) in items.iter().enumerate() {
            let at = format!("execution.costs.taxes[{k}]");
            fields(it, &at, &["name", "pct", "side"], &[])?;
            let name = it.get("name").and_then(Json::as_str).filter(|n| !n.is_empty() && n.len() <= 40 && n.bytes().all(|b| b.is_ascii_alphanumeric() || b" _-".contains(&b)))
                .ok_or_else(|| inv(format!("{at}.name must be 1 to 40 letters, digits, spaces, _ or -")))?;
            let side = match it.get("side").and_then(Json::as_str) { Some("buy") => TaxSide::Buy, Some("sell") => TaxSide::Sell, Some("both") => TaxSide::Both, _ => return Err(inv(format!("{at}.side must be buy, sell or both"))) };
            c.taxes.push((name.into(), num(it.get("pct"), &format!("taxes[{k}].pct"), 10.)?, side));
        }
    }
    if let Some(b) = v.get("borrow_pct_year") { c.borrow_pct_year = Some(num(Some(b), "borrow_pct_year", 100.)?); }
    Ok(c)
}

fn scale_in(v: &Json, at: &str) -> Result<ScaleIn, LabError> {
    fields(v, at, &["legs", "min_bars", "price"], &[])?;
    let bad = || inv(format!("{at}.legs must be a list of 2 to {MAX_LEGS} fractions above 0 that add up to at most 1"));
    let Some(Json::Arr(ls)) = v.get("legs") else { return Err(bad()) };
    let legs = ls.iter().map(|l| l.as_f64().filter(|w| *w > 0. && *w <= 1.)).collect::<Option<Vec<f64>>>().ok_or_else(bad)?;
    if !(2..=MAX_LEGS).contains(&legs.len()) || legs.iter().sum::<f64>() > 1. + 1e-9 { return Err(bad()); }
    let min_bars = match v.get("min_bars") { None => 1, Some(n) => whole(Some(n), &format!("{at}.min_bars"), 1)? };
    let price = match v.get("price").map(Json::as_str) {
        None | Some(Some("any")) => LegPrice::Any, Some(Some("better")) => LegPrice::Better, Some(Some("worse")) => LegPrice::Worse,
        _ => return Err(inv(format!("{at}.price must be \"any\", \"better\" or \"worse\""))),
    };
    Ok(ScaleIn { legs, min_bars, price })
}

/// In the same units the moved stop (`plus`) must stay short of the level that arms it (`after`); `at` is the breakeven path.
fn breakeven_order(after: Dist, plus: Option<Dist>, at: &str) -> Result<(), String> {
    let pair = match (after, plus) {
        (Dist::Pct(a), Some(Dist::Pct(p))) | (Dist::Points(a), Some(Dist::Points(p))) | (Dist::Ticks(a), Some(Dist::Ticks(p))) => Some((a, p)),
        (Dist::Atr { mult: a, length: n }, Some(Dist::Atr { mult: p, length: m })) if n == m => Some((a, p)),
        _ => None,
    };
    match pair { Some((a, p)) if p >= a => Err(format!("{at}.plus ({p}) must be below {at}.after ({a})")), _ => Ok(()) }
}

/// `v` without one key (`trailing.activate`, `take_profits[k].size` sit next to a distance).
fn without(v: &Json, key: &str) -> Json { match v { Json::Obj(kv) => Json::Obj(kv.iter().filter(|(k, _)| k != key).cloned().collect()), _ => v.clone() } }

fn fields(v: &Json, at: &str, allowed: &[&str], reserved: &[&str]) -> Result<(), LabError> {
    let Json::Obj(kv) = v else { return Err(inv(format!("{at} must be an object"))) };
    for (k, _) in kv {
        if reserved.contains(&k.as_str()) { return Err(LabError::Unsupported(format!("{at}.{k} is reserved for a later version; v1 does not simulate it"))); }
        if !allowed.contains(&k.as_str()) { return Err(inv(format!("unknown field '{k}' in {at}"))); }
    }
    Ok(())
}

fn fill(v: &Json, at: &str) -> Result<Fill, LabError> {
    match v.as_str() {
        Some("next_open") => Ok(Fill::NextOpen),
        Some("close") => Ok(Fill::Close),
        Some(f @ ("limit" | "stop")) => Err(LabError::Unsupported(format!("{at} '{f}' is reserved; use entry.limit for limit entries (stop entries are a later version)"))),
        _ => Err(inv(format!("{at} must be \"next_open\" or \"close\""))),
    }
}
/// `{"pct": p}` or `{"atr": m, "atr_length": n}` (n defaults to 14).
fn dist(v: &Json, at: &str, hi: f64) -> Result<Dist, LabError> {
    fields(v, at, &["pct", "atr", "atr_length", "points", "ticks"], &[])?;
    // an entry order may sit at the signal close (0); exits need a positive distance
    let order = at.ends_with("entry.limit") || at.ends_with("entry.stop");
    let min = if order { "from 0" } else { "above 0" };
    if let Some(t) = v.get("ticks") {
        if ["pct","atr","atr_length","points"].iter().any(|k|v.get(k).is_some()) { return Err(inv(format!("{at} needs one distance unit"))); }
        return t.as_f64().filter(|n|n.is_finite() && n.fract()==0. && *n<=MAX_TICKS && (*n>0. || (*n==0. && order))).map(Dist::Ticks)
            .ok_or_else(||inv(format!("{at}.ticks must be a whole number {min} and at most {MAX_TICKS}")));
    }
    match (v.get("pct"), v.get("atr"), v.get("points")) {
        (Some(p), None, None) if v.get("atr_length").is_none() => {
            p.as_f64().filter(|p| *p <= hi && (*p > 0. || (*p == 0. && order))).map(Dist::Pct)
                .ok_or_else(|| inv(format!("{at}.pct must be a number {min} and at most {hi}")))
        }
        (None, None, Some(p)) if v.get("atr_length").is_none() => {
            p.as_f64().filter(|p| *p <= MAX_POINTS && (*p > 0. || (*p == 0. && order))).map(Dist::Points)
                .ok_or_else(|| inv(format!("{at}.points must be a number {min} and at most {MAX_POINTS}")))
        }
        (None, Some(m), None) => {
            let mult = m.as_f64().filter(|m| *m > 0. && *m <= MAX_ATR_MULT).ok_or_else(|| inv(format!("{at}.atr must be a number above 0 and at most {MAX_ATR_MULT}")))?;
            let length = match v.get("atr_length") {
                None => 14,
                Some(n) => n.as_f64().filter(|n| n.fract() == 0. && *n >= 1. && *n <= MAX_ATR_LENGTH as f64)
                    .ok_or_else(|| inv(format!("{at}.atr_length must be a whole number from 1 to {MAX_ATR_LENGTH}")))? as usize,
            };
            Ok(Dist::Atr { mult, length })
        }
        _ => Err(inv(format!("{at} needs exactly one of pct or atr, or points (atr_length goes with atr)"))),
    }
}
fn origin(v: &Json) -> Result<Origin, LabError> {
    fields(v, "origin", &["book", "method", "template_version"], &[])?;
    let id = |k: &str| v.get(k).and_then(Json::as_str).filter(|s| !s.is_empty() && s.len() <= 80 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
        .map(String::from).ok_or_else(|| inv(format!("origin.{k} must be a catalogue id (letters, digits, _)")));
    let (book, method) = (id("book")?, id("method")?);
    let version = v.get("template_version").and_then(Json::as_f64).filter(|n| n.fract() == 0. && *n >= 1. && *n <= 1000.)
        .ok_or_else(|| inv("origin.template_version must be a whole number from 1 to 1000"))? as u32;
    Ok(Origin { book, method, version })
}

/// Counts comparisons and nodes while a spec is parsed (spec §4.3).
#[derive(Default)]
struct Budget { leaves: usize, nodes: usize }
impl Budget {
    fn take(&mut self, leaf: bool) -> Result<(), LabError> {
        self.nodes += 1;
        if leaf { self.leaves += 1; }
        if self.leaves > MAX_LEAVES { return Err(inv(format!("more than {MAX_LEAVES} comparisons in the spec; the limit is {MAX_LEAVES}"))); }
        if self.nodes > MAX_NODES { return Err(inv(format!("more than {MAX_NODES} comparisons, groups and expressions in the spec; the limit is {MAX_NODES}"))); }
        Ok(())
    }
}

fn conds(v: Option<&Json>, at: &str, exit: bool, may_be_empty: bool, b: &mut Budget) -> Result<Vec<Node>, LabError> {
    let Some(Json::Arr(items)) = v else { return Err(inv(format!("{at} must be a list of conditions"))) };
    if items.is_empty() && !may_be_empty { return Err(inv(format!("{at} needs at least one condition{}", if exit { " (or a stop_loss, target or trailing exit)" } else { "" }))); }
    if items.len() > MAX_CONDITIONS { return Err(inv(format!("{at} has {} conditions; the limit is {MAX_CONDITIONS}", items.len()))); }
    items.iter().enumerate().map(|(i, c)| node(c, &format!("{at}[{i}]"), exit, b)).collect()
}

/// One rule-list entry. `held` allows bars_held: top-level exit comparisons only, because every other node is a precomputed array.
fn node(v: &Json, at: &str, held: bool, b: &mut Budget) -> Result<Node, LabError> {
    if v.get("lhs").is_some() { return leaf(v, at, held, b); }
    b.take(false)?;
    let list = |key: &str, b: &mut Budget| -> Result<Vec<Node>, LabError> {
        let Some(Json::Arr(items)) = v.get(key) else { return Err(inv(format!("{at}.{key} must be a list"))) };
        if items.is_empty() { return Err(inv(format!("{at}.{key} needs at least one entry"))); }
        items.iter().enumerate().map(|(i, c)| node(c, &format!("{at}.{key}[{i}]"), false, b)).collect()
    };
    if v.get("all").is_some() { fields(v, at, &["all"], &[])?; return Ok(Node::All(list("all", b)?)); }
    if v.get("any").is_some() { fields(v, at, &["any"], &[])?; return Ok(Node::Any(list("any", b)?)); }
    if let Some(n) = v.get("not") { fields(v, at, &["not"], &[])?; return Ok(Node::Not(Box::new(node(n, &format!("{at}.not"), false, b)?))); }
    if v.get("at_least").is_some() {
        fields(v, at, &["at_least", "of", "when"], &[])?;
        let (k, of) = (whole(v.get("at_least"), &format!("{at}.at_least"), 1)?, whole(v.get("of"), &format!("{at}.of"), 1)?);
        if k > of { return Err(inv(format!("{at}: at_least ({k}) cannot exceed of ({of})"))); }
        let when = v.get("when").ok_or_else(|| inv(format!("{at}.when is required")))?;
        return Ok(Node::AtLeast { k, of, when: Box::new(node(when, &format!("{at}.when"), false, b)?) });
    }
    if v.get("then").is_some() {
        fields(v, at, &["then", "within"], &[])?;
        let steps = list("then", b)?;
        if !(2..=MAX_STEPS).contains(&steps.len()) { return Err(inv(format!("{at}.then needs 2 to {MAX_STEPS} steps"))); }
        let within = whole(v.get("within"), &format!("{at}.within"), 1)?;
        if within + 1 < steps.len() { return Err(inv(format!("{at}: within ({within}) is shorter than the {} steps need ({}); the sequence could never complete", steps.len(), steps.len() - 1))); }
        return Ok(Node::Then { steps, within });
    }
    Err(inv(format!("{at} must be a comparison (lhs, op, rhs) or a group ({})", NODE_KINDS.join(", "))))
}

/// A whole number from `min` to MAX_OFFSET (window lengths, counts, sequence spans).
fn whole(v: Option<&Json>, at: &str, min: usize) -> Result<usize, LabError> {
    v.and_then(Json::as_f64).filter(|n| n.fract() == 0. && *n >= min as f64 && *n <= MAX_OFFSET as f64).map(|n| n as usize)
        .ok_or_else(|| inv(format!("{at} must be a whole number from {min} to {MAX_OFFSET}")))
}

fn leaf(v: &Json, at: &str, held: bool, b: &mut Budget) -> Result<Node, LabError> {
    b.take(true)?;
    let side = |k: &str, b: &mut Budget| operand(v.get(k), &format!("{at}.{k}"), false, b);
    match v.get("op").and_then(Json::as_str) {
        Some(op @ ("between" | "outside")) => {
            fields(v, at, &["lhs", "op", "lo", "hi"], &[])?;
            let (x, lo, hi) = (side("lhs", b)?, side("lo", b)?, side("hi", b)?);
            if let (Operand::Value(l), Operand::Value(h)) = (&lo, &hi) { if l > h { return Err(inv(format!("{at}: lo ({l}) is above hi ({h}); {op} would be {}", if op == "between" { "never true" } else { "always true" }))); } }
            Ok(Node::Range { x, lo, hi, outside: op == "outside" })
        }
        Some(op @ ("rising" | "falling")) => {
            fields(v, at, &["lhs", "op", "bars"], &[])?;
            Ok(Node::Trend { x: side("lhs", b)?, bars: whole(v.get("bars"), &format!("{at}.bars"), 1)?, rising: op == "rising" })
        }
        _ => cond(v, at, held, b).map(Node::Cmp),
    }
}

fn cond(v: &Json, at: &str, held: bool, b: &mut Budget) -> Result<Cond, LabError> {
    fields(v, at, &["lhs", "op", "rhs"], &[])?;
    let op = match v.get("op").and_then(Json::as_str) {
        Some("crosses_above") => Op::CrossesAbove,
        Some("crosses_below") => Op::CrossesBelow,
        Some("crosses") => Op::Crosses,
        Some(">") => Op::Gt,
        Some("<") => Op::Lt,
        Some(">=") => Op::Ge,
        Some("<=") => Op::Le,
        Some("=") => Op::Eq,
        Some("!=") => Op::Ne,
        _ => return Err(inv(format!("{at}.op must be one of {}", OPERATORS.join(", ")))),
    };
    let lhs = operand(v.get("lhs"), &format!("{at}.lhs"), held, b)?;
    let rhs = operand(v.get("rhs"), &format!("{at}.rhs"), held, b)?;
    if matches!((&lhs, &rhs), (Operand::Value(_), Operand::Value(_))) { return Err(inv(format!("{at} compares two numbers; use at least one series"))); }
    Ok(Cond { lhs, op, rhs })
}

fn operand(v: Option<&Json>, at: &str, held: bool, b: &mut Budget) -> Result<Operand, LabError> {
    let v = v.ok_or_else(|| inv(format!("{at} is required")))?;
    if v.get("value").is_some() {
        fields(v, at, &["value"], &[])?;
        return v.get("value").and_then(Json::as_f64).map(Operand::Value).ok_or_else(|| inv(format!("{at}.value must be a number")));
    }
    if v.get("formula").is_some() { return formula_operand(v, at); }
    if let Some(e) = expr(v, at, b)? { return Ok(Operand::Expr(Box::new(e))); }
    fields(v, at, &["s", "p", "tf", "out", "offset", "expand", "symbol", "vs", "fixup"], &[])?;
    let id = v.get("s").and_then(Json::as_str).ok_or_else(|| inv(format!("{at}.s must name a series (or give a value)")))?;
    if id == "bars_held" {
        if !held { return Err(inv(format!("{at}: bars_held is only available as a plain comparison at the top level of the exit list (not inside a group, range, trend or expression)"))); }
        if ["p", "tf", "out", "offset", "expand", "symbol", "vs", "fixup"].iter().any(|k| v.get(k).is_some()) { return Err(inv(format!("{at}: bars_held takes no p, tf, out, offset, expand, symbol, vs or fixup"))); }
        return Ok(Operand::BarsHeld);
    }
    let (tf, offset, given, out) = timing(v, at)?;
    let expand = expand_of(v, at, tf, offset, id == "open")?;
    let text=|key:&str|v.get(key).map(|v|v.as_str().ok_or_else(||inv(format!("{at}.{key} must be a string")))).transpose();
    let foreign=foreign_ref(at,text("symbol")?,text("vs")?,text("fixup")?)?;
    let r = series_ref(at, id, &given, tf, out, offset, foreign)?;
    Ok(if expand == Expand::Last { Operand::Series(r) } else { Operand::Expr(Box::new(Expr::Expand(r, expand))) })
}
/// tf, offset, p and out of a series or formula operand.
fn timing<'a>(v: &'a Json, at: &str) -> Result<(Tf, usize, Params, Option<&'a str>), LabError> {
    let tf = match v.get("tf") { None => Tf::D, Some(t) => t.as_str().and_then(Tf::parse).ok_or_else(|| inv(format!("{at}.tf must be {TF_HELP}")))? };
    let offset = match v.get("offset") {
        None => 0,
        Some(o) => o.as_f64().filter(|n| n.fract() == 0. && *n >= 0. && *n <= MAX_OFFSET as f64)
            .ok_or_else(|| inv(format!("{at}.offset must be a whole number from 0 to {MAX_OFFSET} (past bars only)")))? as usize,
    };
    let mut given = Params::new();
    if let Some(p) = v.get("p") {
        let Json::Obj(kv) = p else { return Err(inv(format!("{at}.p must be an object"))) };
        for (k, x) in kv { given.insert(k.clone(), x.as_f64().ok_or_else(|| inv(format!("{at}.p.{k} must be a number")))?); }
    }
    let out = match v.get("out") { None => None, Some(o) => Some(o.as_str().ok_or_else(|| inv(format!("{at}.out must be a string")))?) };
    Ok((tf, offset, given, out))
}
/// A custom-indicator operand `{"formula","name","p","out","tf","offset","expand"}` (design 2026-10-19 §2). The formula
/// text travels inside the spec, so it counts toward MAX_SPEC_BYTES and a saved spec keeps the formula it was saved with.
fn formula_operand(v: &Json, at: &str) -> Result<Operand, LabError> {
    fields(v, at, &["formula", "name", "p", "out", "tf", "offset", "expand"], &[])?;
    let text = v.get("formula").and_then(Json::as_str).ok_or_else(|| inv(format!("{at}.formula must be a string")))?;
    let name = v.get("name").map(|n| n.as_str().ok_or_else(|| inv(format!("{at}.name must be a string")))).transpose()?;
    let (tf, offset, given, out) = timing(v, at)?;
    let (f, params, out) = super::formula::operand(name, text, &given, out).map_err(|e| inv(format!("{at}.formula: {e}")))?;
    let expand = expand_of(v, at, tf, offset, false)?;
    let r = SeriesRef { id: "formula".into(), params, tf, out, offset, foreign: Foreign::default(), formula: Some(std::sync::Arc::new(f)) };
    Ok(if expand == Expand::Last { Operand::Series(r) } else { Operand::Expr(Box::new(Expr::Expand(r, expand))) })
}
/// `expand` (default last). `first` is accepted only where the value is known by the close of the period's first daily bar:
/// a completed earlier period (offset >= 1) or the period's own first value (`known_at_start`). AmiBroker's docs: expandFirst
/// looks into the future for anything else.
fn expand_of(v: &Json, at: &str, tf: Tf, offset: usize, known_at_start: bool) -> Result<Expand, LabError> {
    let Some(e) = v.get("expand") else { return Ok(Expand::Last) };
    let e = e.as_str().and_then(Expand::parse).ok_or_else(|| inv(format!("{at}.expand must be last, first or point")))?;
    if tf == Tf::D && e != Expand::Last { return Err(inv(format!("{at}.expand applies to W, M, Q, Y and nD operands"))); }
    if e == Expand::First && offset == 0 && !known_at_start {
        return Err(LabError::Unsupported(format!("{at}: expandFirst would show the period's final value from its first day (lookahead); use offset >= 1, the period open, or expand last")));
    }
    Ok(e)
}

/// An expression operand (spec §4.1), or None when `v` is a series operand.
fn expr(v: &Json, at: &str, b: &mut Budget) -> Result<Option<Expr>, LabError> {
    let Json::Obj(kv) = v else { return Ok(None) };
    let Some(key) = kv.iter().map(|(k, _)| k.as_str()).find(|k| EXPRESSIONS.contains(k)) else { return Ok(None) };
    b.take(false)?;
    // a unary, window or state expression over a constant is that constant (or 0): refuse it like two constants
    let sub = |k: &str, b: &mut Budget| match operand(v.get(k), &format!("{at}.{k}"), false, b)? {
        Operand::Value(_) => Err(inv(format!("{at}.{k} is a number; {key} needs a series"))),
        o => Ok(o),
    };
    Ok(Some(match key {
        "add" | "sub" | "mul" | "div" | "min" | "max" | "pct" => {
            fields(v, at, &[key], &[])?;
            let pair = || inv(format!("{at}.{key} must be a list of two operands"));
            let Some(Json::Arr(xs)) = v.get(key) else { return Err(pair()) };
            if xs.len() != 2 { return Err(pair()); }
            let (l, r) = (operand(Some(&xs[0]), &format!("{at}.{key}[0]"), false, b)?, operand(Some(&xs[1]), &format!("{at}.{key}[1]"), false, b)?);
            if matches!((&l, &r), (Operand::Value(_), Operand::Value(_))) { return Err(inv(format!("{at}.{key} combines two numbers; use at least one series"))); }
            let op = [Bin::Add, Bin::Sub, Bin::Mul, Bin::Div, Bin::Min, Bin::Max, Bin::Pct].into_iter().find(|o| o.key() == key).unwrap();
            Expr::Bin(op, l, r)
        }
        "abs" => { fields(v, at, &["abs"], &[])?; Expr::Abs(sub("abs", b)?) }
        "ref" => { fields(v, at, &["ref", "bars"], &[])?; Expr::Ref(sub("ref", b)?, whole(v.get("bars"), &format!("{at}.bars"), 0)?) }
        "bars_since" => { fields(v, at, &[key], &[])?; Expr::BarsSince(cond_of(v, key, at, b)?) }
        "count" => { fields(v, at, &[key, "bars"], &[])?; Expr::Count(cond_of(v, key, at, b)?, whole(v.get("bars"), &format!("{at}.bars"), 1)?) }
        "value_when" => {
            fields(v, at, &[key, "of", "nth"], &[])?;
            let nth = match v.get("nth") {
                None => 1,
                Some(n) => n.as_f64().filter(|n| n.fract() == 0. && (1. ..=MAX_NTH as f64).contains(n)).ok_or_else(|| inv(format!("{at}.nth must be a whole number from 1 to {MAX_NTH}")))? as usize,
            };
            Expr::ValueWhen { when: cond_of(v, key, at, b)?, of: sub("of", b)?, nth }
        }
        "compress" => {
            fields(v, at, &[key, "tf", "mode", "offset", "expand"], &[])?;
            let tf = v.get("tf").and_then(Json::as_str).and_then(Tf::parse).filter(|t| *t != Tf::D)
                .ok_or_else(|| inv(format!("{at}.tf must be one of W, M, Q, Y or nD (2D to 63D)")))?;
            let agg = match v.get("mode") {
                None => Agg::Last,
                Some(m) => [Agg::Last, Agg::First, Agg::High, Agg::Low, Agg::Sum].into_iter().find(|a| Some(a.key()) == m.as_str())
                    .ok_or_else(|| inv(format!("{at}.mode must be one of {}", AGGREGATES.join(", "))))?,
            };
            let offset = if v.get("offset").is_some() { whole(v.get("offset"), &format!("{at}.offset"), 0)? } else { 0 };
            let expand = expand_of(v, at, tf, offset, agg == Agg::First)?;
            Expr::Compress { of: sub(key, b)?, tf, agg, offset, expand }
        }
        "highest_since" | "lowest_since" => { fields(v, at, &[key, "of"], &[])?; Expr::Since { when: cond_of(v, key, at, b)?, of: sub("of", b)?, highest: key == "highest_since" } }
        _ => {
            fields(v, at, &[key, "bars"], &[])?;
            let w = [Win::Sum, Win::Avg, Win::Highest, Win::Lowest, Win::Change].into_iter().find(|w| w.key() == key).unwrap();
            Expr::Win(w, sub(key, b)?, whole(v.get("bars"), &format!("{at}.bars"), 1)?)
        }
    }))
}

/// The condition a state operand watches (`{"bars_since": node}` …).
fn cond_of(v: &Json, key: &str, at: &str, b: &mut Budget) -> Result<Node, LabError> {
    node(v.get(key).ok_or_else(|| inv(format!("{at}.{key} is required")))?, &format!("{at}.{key}"), false, b)
}

/// Validates one series operand against the catalogue (shared by rule specs and /api/lab/series).
pub fn series_ref(at: &str, id: &str, given: &Params, tf: Tf, out: Option<&str>, offset: usize, foreign: Foreign) -> Result<SeriesRef, LabError> {
    if PRICE_SOURCES.contains(&id) {
        if !given.is_empty() || out.is_some() || foreign.vs.is_some() { return Err(inv(format!("{at}: price source '{id}' takes no p or out or vs"))); }
        return Ok(SeriesRef { id: id.into(), params: Params::new(), tf, out: 0, offset, foreign, formula: None });
    }
    let def = catalogue::get(id).ok_or_else(|| inv(format!("{at}.s: unknown series '{id}'")))?;
    if def.needs.intraday { return Err(LabError::NeedsData(format!("{at}: {}: {}", def.name, def.note))); }
    if def.compute.is_none() {
        let msg = format!("{at}: {}: {}", def.name, def.note);
        return Err(if def.needs.any_data() { LabError::NeedsData(msg) } else { LabError::Unsupported(msg) });
    }
    if def.needs.second_symbol != foreign.vs.is_some() {
        return Err(inv(if foreign.vs.is_some(){format!("{at}.vs applies to second-instrument indicators only")}else{format!("{at}: {} compares two instruments; name the second one in vs",def.name)}));
    }
    if foreign.vs.is_some() && tf!=Tf::D {return Err(inv(format!("{at}: second-instrument indicators run on daily bars (tf D)")));}
    let params = catalogue::resolve_params(&def, given).map_err(|e| inv(format!("{at}.p: {e}")))?;
    let out = match out {
        None => 0,
        Some(o) => def.outputs.iter().position(|x| *x == o).ok_or_else(|| inv(format!("{at}.out: {} has outputs {}", def.id, def.outputs.join(", "))))?,
    };
    Ok(SeriesRef { id: id.into(), params, tf, out, offset, foreign, formula: None })
}

fn dims(spec: &Spec, v: &Json) -> Result<Vec<Dim>, LabError> {
    let Json::Arr(items) = v else { return Err(inv("optimize must be a list")) };
    let mut out: Vec<Dim> = Vec::new();
    let mut combos = 1usize;
    for (i, it) in items.iter().enumerate() {
        let at = format!("optimize[{i}]");
        fields(it, &at, &["path", "from", "to", "step"], &[])?;
        let path = it.get("path").and_then(Json::as_str).ok_or_else(|| inv(format!("{at}.path must be a string")))?;
        let target = target(spec, path).map_err(|e| inv(format!("{at}.path '{path}': {e}")))?;
        if out.iter().any(|d| d.target == target) { return Err(inv(format!("{at}.path '{path}' is listed twice"))); }
        let num = |k: &str| it.get(k).and_then(Json::as_f64).ok_or_else(|| inv(format!("{at}.{k} must be a number")));
        // Same grid rule as the book optimizer (main.rs grid(): ≤ 1000 values, 1e-9 rounding).
        let (from, to, step) = (num("from")?, num("to")?, num("step")?);
        // crate::grid overflows on huge or tiny-step ranges, so bound them before calling it.
        if ![from, to, step].iter().all(|x| x.is_finite()) || step <= 0. || to < from || (to - from) / step > 1000. {
            return Err(inv(format!("{at}.path '{path}': bad range {from}:{to}:{step}; need finite from <= to, step > 0 and at most 1000 values")));
        }
        let values = crate::grid(&format!("{from}:{to}:{step}")).map_err(|e| inv(format!("{at}: {e}")))?;
        if values.is_empty() || values.iter().any(|x| !x.is_finite()) { return Err(inv(format!("{at}.path '{path}': bad range, no values"))); }
        for &x in &values { check_value(spec, &target, x).map_err(|e| inv(format!("{at}: {e}")))?; }
        combos = combos.saturating_mul(values.len());
        let cap=if spec.portfolio.is_some(){MAX_PORTFOLIO_COMBOS}else{crate::MAX_COMBOS};
        if combos > cap { return Err(inv(format!("optimize gives more than {cap} combinations; narrow a range or raise a step"))); }
        out.push(Dim { path: path.into(), target, values });
    }
    // at_least and of may both be optimized: the worst pairing must still be a valid node.
    for a in &out {
        let Some(base) = a.path.strip_suffix(".at_least") else { continue };
        let of_path = format!("{base}.of");
        let Some(o) = out.iter().find(|d| d.path == of_path) else { continue };
        let (hi, lo) = (a.values.iter().copied().fold(f64::MIN, f64::max), o.values.iter().copied().fold(f64::MAX, f64::min));
        if hi > lo { return Err(inv(format!("optimize '{}' reaches {hi} but '{of_path}' reaches down to {lo}; at_least cannot exceed of", a.path))); }
    }
    if let Some(p)=spec.portfolio.as_ref().filter(|p|p.worst_rank_held.is_some()||out.iter().any(|d|d.path=="portfolio.worst_rank_held")) {
        let high=out.iter().find(|d|d.path=="portfolio.max_open").map_or(p.max_open as f64,|d|d.values.iter().copied().fold(f64::MIN,f64::max));
        let low=out.iter().find(|d|d.path=="portfolio.worst_rank_held").map_or(p.worst_rank_held() as f64,|d|d.values.iter().copied().fold(f64::MAX,f64::min));
        if high>low{return Err(inv("optimize portfolio.worst_rank_held must cover every portfolio.max_open value"));}
    }
    Ok(out)
}

/// The execution settings a path addresses: the short side's for `short_side.…`, otherwise the spec's.
fn exec_of<'a>(s: &'a Spec, path: &str) -> Option<&'a Exec> { if path.starts_with("short_side.") { s.short_side.as_ref().map(|x| &x.exec) } else { Some(&s.exec) } }
fn exec_mut<'a>(s: &'a mut Spec, path: &str) -> Option<&'a mut Exec> { if path.starts_with("short_side.") { s.short_side.as_mut().map(|x| &mut x.exec) } else { Some(&mut s.exec) } }

/// The price-order slot a path (without `short_side.`) addresses: (slot, slot path, field, largest pct).
fn slot_at(rest: &str) -> Option<(Slot, String, String, f64)> {
    if let Some(r) = rest.strip_prefix("exit.take_profits[") {
        let (k, f) = r.split_once("].")?;
        let k: usize = k.parse().ok()?;
        return Some((Slot::TakeProfit(k), format!("exit.take_profits[{k}]"), f.into(), 1000.));
    }
    [(Slot::Activate, "exit.trailing.activate", 1000.), (Slot::BreakevenAfter, "exit.breakeven.after", 1000.)].into_iter().chain(SLOTS)
        .find_map(|(s, at, hi)| rest.strip_prefix(&format!("{at}.")).filter(|f| !f.starts_with("at.")).map(|f| (s, at.to_string(), f.to_string(), hi)))
}

fn target(spec: &Spec, path: &str) -> Result<Target, String> {
    if path.starts_with("portfolio.") && spec.portfolio.is_none(){return Err("the spec has no portfolio".into());}
    if matches!(path,"portfolio.max_open"|"portfolio.worst_rank_held") {
        if path.ends_with("worst_rank_held")&&spec.portfolio.as_ref().is_some_and(|p|p.mode!=Mode::Rotation){return Err("portfolio.worst_rank_held applies to rotation mode".into());}
        return Ok(Target{path:path.into(),slot:None});
    }
    let rest = path.strip_prefix("short_side.").unwrap_or(path);
    if rest == "entry.stop.at" || rest == "entry.limit.at" {
        return Err(format!("{rest} is a level operand; optimize a number inside it ({rest}.value, {rest}.p.<key> or a whole-number field)"));
    }
    if let Some((slot, at, field, _)) = slot_at(rest) {
        let exec = exec_of(spec, path).ok_or("the spec has no short_side")?;
        return match (exec.slot(slot), field.as_str()) {
            (Some(Dist::Pct(_)), "pct") | (Some(Dist::Atr { .. }), "atr") | (Some(Dist::Points(_)), "points") | (Some(Dist::Ticks(_)), "ticks") | (Some(Dist::ProfitPct(_)), "profit_pct") => Ok(Target { path: path.into(), slot: Some(slot) }),
            (None, _) => Err(format!("the spec has no {at}")),
            (Some(d), _) => Err(format!("{at} is set by {}; optimize that field", match d { Dist::Pct(_) => "pct", Dist::Atr { .. } => "atr", Dist::Points(_) => "points", Dist::Ticks(_) => "ticks", Dist::ProfitPct(_) => "profit_pct" })),
        };
    }
    if !["entry.all[", "exit.any[", "entry.limit.at.", "entry.stop.at.", "entry.scale_in.", "execution.sizing.", "portfolio.score."].iter().any(|p| rest.starts_with(p)) {
        return Err("must start with entry.all[i], exit.any[i], entry.limit, entry.limit.at, entry.stop, entry.stop.at, exit.stop_loss, exit.target, exit.trailing, exit.trailing.activate, exit.breakeven.after, exit.take_profits[k], entry.scale_in.min_bars (each also under short_side.), execution.sizing.pct_equity or execution.sizing.risk_pct".into());
    }
    let (mut found, mut near) = (false, None::<String>);
    walk(&mut spec.clone(), &mut |p, k| match k {
        Knob::Series(r) => if let Some(key) = path.strip_prefix(&format!("{p}.p.")) {
            near = Some(if let Some(f) = &r.formula { format!("{} has no parameter '{key}'", f.name) } else { catalogue::get(&r.id).map_or("price sources have no parameters".into(), |d| format!("{} has no parameter '{key}'", d.id)) });
        },
        _ => found |= p == path,
    });
    if found { return Ok(Target { path: path.into(), slot: None }); }
    if let Some(m) = near { return Err(m); }
    let (entry, exit) = match (path.starts_with("short_side."), &spec.short_side) {
        (false, _) => (spec.entry.len(), spec.exit.len()),
        (true, Some(s)) => (s.entry.len(), s.exit.len()),
        (true, None) => return Err("the spec has no short_side".into()),
    };
    let beyond = |list: &str, n: usize| rest.strip_prefix(list).and_then(|r| r.split_once(']')).and_then(|(i, _)| i.parse::<usize>().ok()).is_some_and(|i| i >= n);
    if beyond("entry.all[", entry) || beyond("exit.any[", exit) { return Err("no condition at that index".into()); }
    Err("names no number in the spec (paths end in .value, .p.<key> or a whole-number field: bars, within, of, at_least)".into())
}

/// A number an optimize path can reach inside a rule list; `Series` is reported for error messages only.
enum Knob<'a> { Value(&'a mut f64), Param(&'a mut SeriesRef, String), Whole(&'a mut usize), Shift(&'a mut usize), Series(&'a SeriesRef), Pct(&'a mut f64) }
/// Visits every optimizable number in the rule lists with its spec path (`entry.all[0].lhs.p.length`, `exit.any[1].rhs.value`, `….bars`).
fn walk(s: &mut Spec, f: &mut dyn FnMut(String, Knob)) {
    for (k, n) in s.entry.iter_mut().enumerate() { walk_node(n, &format!("entry.all[{k}]"), f); }
    for (k, n) in s.exit.iter_mut().enumerate() { walk_node(n, &format!("exit.any[{k}]"), f); }
    walk_levels(&mut s.exec, "", f);
    match &mut s.exec.sizing {
        Sizing::PctEquity(p) => f("execution.sizing.pct_equity".into(), Knob::Pct(p)),
        Sizing::RiskPct(p) => f("execution.sizing.risk_pct".into(), Knob::Pct(p)),
        _ => {}
    }
    if let Some(x) = s.short_side.as_mut() {
        for (k, n) in x.entry.iter_mut().enumerate() { walk_node(n, &format!("short_side.entry.all[{k}]"), f); }
        for (k, n) in x.exit.iter_mut().enumerate() { walk_node(n, &format!("short_side.exit.any[{k}]"), f); }
        walk_levels(&mut x.exec, "short_side.", f);
    }
    if let Some(score)=s.portfolio.as_mut().and_then(|p|p.score.as_mut()){walk_operand(score,"portfolio.score",f);}

}
fn walk_levels(x: &mut Exec, pre: &str, f: &mut dyn FnMut(String, Knob)) {
    if let Some(si) = x.scale_in.as_mut() { f(format!("{pre}entry.scale_in.min_bars"), Knob::Whole(&mut si.min_bars)); }
    if let Some(o) = x.limit_at.as_mut() { walk_operand(o, &format!("{pre}entry.limit.at"), f); }
    if let Some(o) = x.stop_at.as_mut() { walk_operand(o, &format!("{pre}entry.stop.at"), f); }
}
fn walk_node(n: &mut Node, at: &str, f: &mut dyn FnMut(String, Knob)) {
    match n {
        Node::Cmp(c) => { walk_operand(&mut c.lhs, &format!("{at}.lhs"), f); walk_operand(&mut c.rhs, &format!("{at}.rhs"), f); }
        Node::Range { x, lo, hi, .. } => { walk_operand(x, &format!("{at}.lhs"), f); walk_operand(lo, &format!("{at}.lo"), f); walk_operand(hi, &format!("{at}.hi"), f); }
        Node::Trend { x, bars, .. } => { walk_operand(x, &format!("{at}.lhs"), f); f(format!("{at}.bars"), Knob::Whole(bars)); }
        Node::All(v) => for (k, c) in v.iter_mut().enumerate() { walk_node(c, &format!("{at}.all[{k}]"), f) },
        Node::Any(v) => for (k, c) in v.iter_mut().enumerate() { walk_node(c, &format!("{at}.any[{k}]"), f) },
        Node::Not(c) => walk_node(c, &format!("{at}.not"), f),
        Node::AtLeast { k, of, when } => { f(format!("{at}.at_least"), Knob::Whole(k)); f(format!("{at}.of"), Knob::Whole(of)); walk_node(when, &format!("{at}.when"), f); }
        Node::Then { steps, within } => {
            for (k, c) in steps.iter_mut().enumerate() { walk_node(c, &format!("{at}.then[{k}]"), f); }
            f(format!("{at}.within"), Knob::Whole(within));
        }
    }
}
fn walk_operand(o: &mut Operand, at: &str, f: &mut dyn FnMut(String, Knob)) {
    match o {
        Operand::Value(x) => f(format!("{at}.value"), Knob::Value(x)),
        Operand::Series(r) => walk_series(r, at, f),
        Operand::BarsHeld => {}
        Operand::Expr(e) => match &mut **e {
            Expr::Bin(op, l, r) => { walk_operand(l, &format!("{at}.{}[0]", op.key()), f); walk_operand(r, &format!("{at}.{}[1]", op.key()), f); }
            Expr::Abs(x) => walk_operand(x, &format!("{at}.abs"), f),
            Expr::Ref(x, n) => { walk_operand(x, &format!("{at}.ref"), f); f(format!("{at}.bars"), Knob::Shift(n)); }
            Expr::Win(w, x, n) => { walk_operand(x, &format!("{at}.{}", w.key()), f); f(format!("{at}.bars"), Knob::Whole(n)); }
            Expr::BarsSince(c) => walk_node(c, &format!("{at}.bars_since"), f),
            Expr::Count(c, n) => { walk_node(c, &format!("{at}.count"), f); f(format!("{at}.bars"), Knob::Whole(n)); }
            Expr::ValueWhen { when, of, nth } => { walk_node(when, &format!("{at}.value_when"), f); walk_operand(of, &format!("{at}.of"), f); f(format!("{at}.nth"), Knob::Whole(nth)); }
            Expr::Since { when, of, highest } => { walk_node(when, &format!("{at}.{}", if *highest { "highest_since" } else { "lowest_since" }), f); walk_operand(of, &format!("{at}.of"), f); }
            Expr::Expand(r, _) => walk_series(r, at, f),
            Expr::Compress { of, .. } => walk_operand(of, &format!("{at}.compress"), f),
        },
    }
}
fn walk_series(r: &mut SeriesRef, at: &str, f: &mut dyn FnMut(String, Knob)) {
    f(at.to_string(), Knob::Series(r));
    let keys: Vec<String> = r.params.keys().cloned().collect();
    for k in keys { f(format!("{at}.p.{k}"), Knob::Param(&mut *r, k)); }
}
/// Node invariants the parser checks, re-checked after an optimize value is applied (at_least <= of, then spans, constant range bounds).
fn consistent(n: &Node) -> Result<(), String> {
    match n {
        Node::AtLeast { k, of, .. } if k > of => Err(format!("at_least ({k}) cannot exceed of ({of})")),
        Node::Then { steps, within } if *within + 1 < steps.len() => Err(format!("within ({within}) is shorter than the {} steps need ({})", steps.len(), steps.len() - 1)),
        Node::Range { lo: Operand::Value(l), hi: Operand::Value(h), .. } if l > h => Err(format!("lo ({l}) is above hi ({h})")),
        Node::AtLeast { when, .. } | Node::Not(when) => consistent(when),
        Node::All(v) | Node::Any(v) | Node::Then { steps: v, .. } => v.iter().try_for_each(consistent),
        Node::Cmp(c) => [&c.lhs, &c.rhs].into_iter().try_for_each(consistent_operand),
        Node::Range { x, lo, hi, .. } => [x, lo, hi].into_iter().try_for_each(consistent_operand),
        Node::Trend { x, .. } => consistent_operand(x),
    }
}
/// Nodes inside state operands obey the same rules; value_when's nth stays within MAX_NTH.
fn consistent_operand(o: &Operand) -> Result<(), String> {
    let Operand::Expr(e) = o else { return Ok(()) };
    match &**e {
        Expr::Bin(_, l, r) => { consistent_operand(l)?; consistent_operand(r) }
        Expr::Abs(x) | Expr::Ref(x, _) | Expr::Win(_, x, _) => consistent_operand(x),
        Expr::BarsSince(c) | Expr::Count(c, _) => consistent(c),
        Expr::ValueWhen { nth, .. } if *nth > MAX_NTH => Err(format!("nth ({nth}) must be a whole number from 1 to {MAX_NTH}")),
        Expr::ValueWhen { when, of, .. } | Expr::Since { when, of, .. } => { consistent(when)?; consistent_operand(of) }
        Expr::Compress { of, .. } => consistent_operand(of),
        Expr::Expand(..) => Ok(()),
    }
}

fn set(s: &mut Spec, t: &Target, x: f64) {
    if matches!(t.path.as_str(),"portfolio.max_open"|"portfolio.worst_rank_held") {
        if let Some(p)=s.portfolio.as_mut(){
            let hi=if t.path.ends_with("max_open"){MAX_OPEN}else{MAX_UNIVERSE};
            let n=if x.is_finite()&&x.fract()==0.&&x>=1.&&x<=hi as f64{x as usize}else{usize::MAX};
            if t.path.ends_with("max_open"){p.max_open=n;}else{p.worst_rank_held=Some(n);}
        }
        return;
    }
    if let Some(slot) = t.slot {
        match exec_mut(s, &t.path).and_then(|e| e.slot_mut(slot)) { Some(Dist::Pct(p) | Dist::Points(p) | Dist::Ticks(p) | Dist::ProfitPct(p)) => *p = x, Some(Dist::Atr { mult, .. }) => *mult = x, None => {} }
        return;
    }
    walk(s, &mut |p, k| if p == t.path {
        match k {
            Knob::Value(v) | Knob::Pct(v) => *v = x,
            Knob::Param(r, key) => { r.params.insert(key, x); }
            Knob::Whole(n) | Knob::Shift(n) => *n = x as usize,
            Knob::Series(_) => {}
        }
    });
}
fn check_value(spec: &Spec, t: &Target, x: f64) -> Result<(), String> {
    if matches!(t.path.as_str(),"portfolio.max_open"|"portfolio.worst_rank_held") {
        let hi=if t.path.ends_with("max_open"){MAX_OPEN}else{MAX_UNIVERSE};
        return if x.is_finite()&&x.fract()==0.&&x>=1.&&x<=hi as f64{Ok(())}else{Err(format!("{} must be a whole number from 1 to {hi}",t.path))};
    }
    if let Some(slot) = t.slot {
        let (_, at, _, hi) = slot_at(t.path.strip_prefix("short_side.").unwrap_or(&t.path)).unwrap();
        let order = matches!(slot, Slot::Limit | Slot::Stop);
        let ok = match exec_of(spec, &t.path).and_then(|e| e.slot(slot)) {
            Some(Dist::Pct(_)) => x <= hi && (x > 0. || (x == 0. && order)),
            Some(Dist::Ticks(_)) => x.fract() == 0. && x <= MAX_TICKS && (x > 0. || (x == 0. && order)),
            Some(Dist::Points(_)) => x <= MAX_POINTS && (x > 0. || (x == 0. && order)),
            Some(Dist::ProfitPct(_)) => x > 0. && x < 100.,
            _ => x > 0. && x <= MAX_ATR_MULT,
        };
        if !ok { return Err(format!("{at} value {x} is out of range")); }
        if slot == Slot::BreakevenAfter {
            let mut s = spec.clone();
            set(&mut s, t, x);
            if let Some((after, plus)) = exec_of(&s, &t.path).and_then(|e| e.breakeven) {
                return breakeven_order(after, plus, at.strip_suffix(".after").unwrap_or(&at)).map_err(|e| format!("{} value {x}: {e}", t.path));
            }
        }
        return Ok(());
    }
    let mut s = spec.clone();
    set(&mut s, t, x);
    let mut res = Ok(());
    walk(&mut s, &mut |p, k| if p == t.path {
        res = match k {
            Knob::Param(r, _) if r.formula.is_some() => r.formula.as_ref().map_or(Ok(()), |f| super::formula::resolve(&f.prog, &r.params).and_then(|p| super::formula::check(&f.prog, &p))),
            Knob::Param(r, _) => catalogue::get(&r.id).ok_or_else(|| "unknown series".to_string()).and_then(|d| catalogue::resolve_params(&d, &r.params).map(|_| ())),
            Knob::Shift(_) if x.fract() != 0. || x < 0. || x > MAX_OFFSET as f64 => Err(format!("{} must be a whole number from 0 to {MAX_OFFSET}, not {x}", t.path)),
            Knob::Whole(_) if x.fract() != 0. || x < 1. || x > MAX_OFFSET as f64 => Err(format!("{} must be a whole number from 1 to {MAX_OFFSET}, not {x}", t.path)),
            Knob::Pct(_) if !(x > 0. && x <= 100.) => Err(format!("{} must be above 0 and at most 100, not {x}", t.path)),
            _ => Ok(()),
        };
    });
    res?;
    let short = s.short_side.as_ref().map(|x| x.entry.iter().chain(&x.exit));
    s.entry.iter().chain(&s.exit).chain(short.into_iter().flatten()).try_for_each(consistent)?;
    let execs = std::iter::once(&s.exec).chain(s.short_side.as_ref().map(|x| &x.exec));
    let levels = execs.flat_map(|e| [&e.limit_at, &e.stop_at]).flatten().try_for_each(consistent_operand);
    levels?;
    if let Some(score)=s.portfolio.as_ref().and_then(|p|p.score.as_ref()){consistent_operand(score)?;}
    Ok(())
}
/// Every catalogue or price series id the rule lists read, nested ones included.
pub fn series_ids(spec: &Spec) -> Vec<String> {
    let mut ids = Vec::new();
    walk(&mut spec.clone(), &mut |_, k| if let Knob::Series(r) = k { ids.push(r.id.clone()); });
    ids
}
/// The spec's own value at an optimize target (the "base" setting the ranking prefers on ties).
pub fn current(spec: &Spec, t: &Target) -> f64 {
    if matches!(t.path.as_str(),"portfolio.max_open"|"portfolio.worst_rank_held") {
        return spec.portfolio.as_ref().map_or(f64::NAN,|p|if t.path.ends_with("max_open"){p.max_open as f64}else{p.worst_rank_held() as f64});
    }
    if let Some(slot) = t.slot { return match exec_of(spec, &t.path).and_then(|e| e.slot(slot)) { Some(Dist::Pct(p) | Dist::Points(p) | Dist::Ticks(p) | Dist::ProfitPct(p)) => p, Some(Dist::Atr { mult, .. }) => mult, None => f64::NAN }; }
    let mut v = f64::NAN;
    walk(&mut spec.clone(), &mut |p, k| if p == t.path {
        match k {
            Knob::Value(x) | Knob::Pct(x) => v = *x,
            Knob::Param(r, key) => v = r.params.get(&key).copied().unwrap_or(f64::NAN),
            Knob::Whole(n) | Knob::Shift(n) => v = *n as f64,
            Knob::Series(_) => {}
        }
    });
    v
}
/// The spec with one grid combination applied (values in `dims` order; validated when the spec was parsed).
pub fn apply(spec: &Spec, dims: &[Dim], values: &[f64]) -> Spec {
    let mut s = spec.clone();
    for (d, &x) in dims.iter().zip(values) { set(&mut s, &d.target, x); }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    const EXAMPLE: &str = r#"{ "v": 1, "direction": "long",
      "entry": { "all": [
        { "lhs": {"s":"rsi","p":{"length":2},"tf":"D"}, "op":"crosses_below", "rhs": {"value":10} },
        { "lhs": {"s":"close","tf":"D"}, "op":">", "rhs": {"s":"sma","p":{"length":40},"tf":"W"} } ] },
      "exit": { "any": [
        { "lhs": {"s":"rsi","p":{"length":2},"tf":"D"}, "op":"crosses_above", "rhs": {"value":70} },
        { "lhs": {"s":"bars_held"}, "op":">=", "rhs": {"value":10} } ] },
      "execution": { "fill": "next_open", "cost_bps": 12 },
      "optimize": [ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ] }"#;
    fn with(old: &str, new: &str) -> String { assert!(EXAMPLE.contains(old), "{old}"); EXAMPLE.replacen(old, new, 1) }
    fn err(text: &str) -> LabError { parse_spec(text).unwrap_err() }

    #[test] fn spec_example_round_trips() {
        let s = parse_spec(EXAMPLE).unwrap();
        assert_eq!((s.entry.len(), s.exit.len(), s.cost_bps), (2, 2, Some(12.)));
        let Operand::Series(r) = &s.entry[1].cmp().unwrap().rhs else { panic!() };
        assert_eq!((r.id.as_str(), r.tf, r.params["length"]), ("sma", Tf::W, 40.));
        assert_eq!(s.exit[1].cmp().unwrap().lhs, Operand::BarsHeld);
        assert_eq!(s.optimize[0].values, vec![2., 3., 4., 5., 6.]);
        assert_eq!(current(&s, &s.optimize[0].target), 2.);
        let t = apply(&s, &s.optimize, &[5.]);
        let Operand::Series(r) = &t.entry[0].cmp().unwrap().lhs else { panic!() };
        assert_eq!(r.params["length"], 5.);
    }
    #[test] fn defaults_fill_params_tf_and_output() {
        let s = parse_spec(&with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"macd","out":"signal"}"#)).unwrap();
        let Operand::Series(r) = &s.entry[1].cmp().unwrap().lhs else { panic!() };
        assert_eq!((r.tf, r.out, r.params["fast"], r.params["slow"]), (Tf::D, 1, 12., 26.));
    }
    #[test] fn malformed_input_is_invalid_and_names_the_field() {
        for (text, needle) in [
            (with(r#""v": 1"#, r#""v": 2"#), "v must be 1"),
            (with(r#""op":"crosses_below""#, r#""op":"below""#), "entry.all[0].op"),
            (with(r#""tf":"W""#, r#""tf":"H""#), "entry.all[1].rhs.tf"),
            (with(r#"{"length":40}"#, r#"{"length":0}"#), "entry.all[1].rhs.p: sma: length"),
            (with(r#"{"length":40}"#, r#"{"len":40}"#), "unknown parameter 'len'"),
            (with(r#""s":"sma""#, r#""s":"smaa""#), "unknown series 'smaa'"),
            (with(r#""rhs": {"value":10}"#, r#""rhs": {"value":10,"offset":1}"#), "unknown field 'offset' in entry.all[0].rhs"),
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"close","offset":-1}"#), "entry.all[1].lhs.offset"),
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"close","offset":1.5}"#), "entry.all[1].lhs.offset"),
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"bars_held"}"#), "only available as a plain comparison"),
            (with(r#"{"s":"rsi","p":{"length":2},"tf":"D"}"#, r#"{"value":3}"#), "compares two numbers"),
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"macd","out":"sig"}"#), "macd has outputs macd, signal, hist"),
            (with(r#""entry""#, r#""entri""#), "unknown field 'entri' in spec"),
            (with(r#""all": ["#, r#""all": [], "x": ["#), "unknown field 'x' in entry"),
            ("{\"v\":1}".to_string(), "entry is required"),
            ("not json".to_string(), "not valid JSON"),
            (format!("{{\"v\":1,\"pad\":\"{}\"}}", "x".repeat(MAX_SPEC_BYTES)), "the limit is 8192"),
            (with("\"from\":2,\"to\":6", "\"from\":0,\"to\":6"), "optimize[0]: rsi: length must be between"),
            (with("entry.all[0].lhs.p.length", "entry.all[0].lhs.p.bogus"), "rsi has no parameter 'bogus'"),
            (with("entry.all[0].lhs.p.length", "entry.all[1].lhs.p.length"), "price sources have no parameters"),
            (with("entry.all[0].lhs.p.length", "entry.all[9].lhs.p.length"), "no condition at that index"),
            (with("\"step\":1", "\"step\":0"), "bad range"),
        ] {
            let e = err(&text);
            assert_eq!(e.status(), "invalid", "{text}: {e:?}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
    }
    #[test] fn reserved_fields_are_unsupported() {
        for (text, needle) in [

            (with(r#""fill": "next_open""#, r#""fill": "limit""#), "execution.fill 'limit'"),
            (with(r#""fill": "next_open""#, r#""fill": "stop""#), "execution.fill 'stop'"),
            (with(r#""v": 1,"#, r#""v": 1, "scale_in": [],"#), "spec.scale_in"),
        ] {
            let e = err(&text);
            assert_eq!(e.status(), "unsupported", "{text}: {e:?}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
    }
    #[test] fn customize_fields_parse() {
        let s = parse_spec(&with(r#""direction": "long""#, r#""direction": "short", "origin": {"book":"stc9","method":"rsi2_4","template_version":1}"#)).unwrap();
        assert!(s.exec.short);
        assert_eq!(s.origin, Some(Origin { book: "stc9".into(), method: "rsi2_4".into(), version: 1 }));
        assert_eq!((s.exec.fill, s.exec.exit_fill, s.exec.limit, s.exec.stop_loss), (Fill::NextOpen, Fill::NextOpen, None, None));
        let s = parse_spec(&with(r#""fill": "next_open""#, r#""fill": "close""#)).unwrap();
        assert_eq!((s.exec.fill, s.exec.exit_fill), (Fill::Close, Fill::Close));
        let s = parse_spec(&with(r#""fill": "next_open""#, r#""fill": "close", "exit_fill": "next_open""#)).unwrap();
        assert_eq!((s.exec.fill, s.exec.exit_fill), (Fill::Close, Fill::NextOpen));
        let s = parse_spec(&with(r#""all": ["#, r#""limit": {"pct": 3}, "all": ["#).replace(r#""fill": "next_open", "#, "")).unwrap();
        assert_eq!(s.exec.limit, Some(Dist::Pct(3.)));
        let s = parse_spec(&with(r#""any": ["#, r#""stop_loss": {"atr": 2}, "target": {"pct": 10}, "trailing": {"atr": 3, "atr_length": 20}, "any": ["#)).unwrap();
        assert_eq!((s.exec.stop_loss, s.exec.target, s.exec.trailing), (Some(Dist::Atr { mult: 2., length: 14 }), Some(Dist::Pct(10.)), Some(Dist::Atr { mult: 3., length: 20 })));
        // A price exit lets exit.any be empty; without one it still needs a condition.
        let only_target = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}]},"exit":{"any":[],"target":{"pct":10}}}"#;
        assert!(parse_spec(only_target).unwrap().exit.is_empty());
        assert!(err(&only_target.replace(r#","target":{"pct":10}"#, "")).message().contains("or a stop_loss, target or trailing exit"));
    }
    #[test] fn customize_fields_are_bounded_and_named() {
        for (text, needle) in [
            (with(r#""direction": "long""#, r#""direction": "sideways""#), "direction must be"),
            (with(r#""fill": "next_open""#, r#""fill": "open""#), "execution.fill must be"),
            (with(r#""fill": "next_open""#, r#""exit_fill": "later""#), "execution.exit_fill must be"),
            (with(r#""all": ["#, r#""limit": {"pct": 3}, "all": ["#), "remove execution.fill"),
            (with(r#""all": ["#, r#""limit": {"pct": 51}, "all": ["#).replace(r#""fill": "next_open", "#, ""), "entry.limit.pct must be a number from 0 and at most 50"),
            (with(r#""any": ["#, r#""stop_loss": {"pct": 0}, "any": ["#), "exit.stop_loss.pct must be a number above 0"),
            (with(r#""any": ["#, r#""trailing": {"pct": 100}, "any": ["#), "exit.trailing.pct"),
            (with(r#""any": ["#, r#""target": {"pct": 5, "atr": 1}, "any": ["#), "exactly one of pct or atr"),
            (with(r#""any": ["#, r#""target": {"pct": 5, "atr_length": 3}, "any": ["#), "exactly one of pct or atr"),
            (with(r#""any": ["#, r#""stop_loss": {"atr": 0}, "any": ["#), "exit.stop_loss.atr must be"),
            (with(r#""any": ["#, r#""stop_loss": {"atr": 1, "atr_length": 501}, "any": ["#), "atr_length must be a whole number"),
            (with(r#""any": ["#, r#""stop_loss": {"atr": 1, "len": 5}, "any": ["#), "unknown field 'len' in exit.stop_loss"),
            (with(r#""v": 1,"#, r#""v": 1, "origin": {"book":"stc9","method":"rsi 2"},"#), "origin.method"),
            (with(r#""v": 1,"#, r#""v": 1, "origin": {"book":"stc9","method":"rsi2_4","template_version":0},"#), "origin.template_version"),
            (with(r#""v": 1,"#, r#""v": 1, "origin": {"book":"stc9","method":"rsi2_4","template_version":1,"x":1},"#), "unknown field 'x' in origin"),
        ] {
            let e = err(&text);
            assert_eq!(e.status(), "invalid", "{text}: {e:?}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
    }
    #[test] fn optimize_reaches_price_orders() {
        let text = with(r#""any": ["#, r#""stop_loss": {"pct": 5}, "trailing": {"atr": 3}, "any": ["#)
            .replace(r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ]"#, r#"[ {"path":"exit.stop_loss.pct","from":2,"to":8,"step":2}, {"path":"exit.trailing.atr","from":1,"to":3,"step":1} ]"#);
        let s = parse_spec(&text).unwrap();
        assert_eq!((s.optimize[0].values.len(), current(&s, &s.optimize[0].target), current(&s, &s.optimize[1].target)), (4, 5., 3.));
        let t = apply(&s, &s.optimize, &[4., 2.]);
        assert_eq!((t.exec.stop_loss, t.exec.trailing), (Some(Dist::Pct(4.)), Some(Dist::Atr { mult: 2., length: 14 })));
        assert!(err(&text.replace("exit.stop_loss.pct", "exit.stop_loss.atr")).message().contains("exit.stop_loss is set by pct"));
        assert!(err(&text.replace("exit.stop_loss.pct", "exit.target.pct")).message().contains("the spec has no exit.target"));
        assert!(err(&text.replace(r#""from":2,"to":8"#, r#""from":50,"to":120"#)).message().contains("out of range"));
    }
    #[test] fn intraday_and_condition_limits() {
        let e = err(&with(r#""s":"sma""#, r#""s":"vwap""#));
        assert_eq!(e.status(), "needs_data");
        let cond = r#"{ "lhs": {"s":"close"}, "op":">", "rhs": {"value":1} }"#;
        let many = format!(r#"{{"v":1,"entry":{{"all":[{}]}},"exit":{{"any":[{}]}}}}"#, vec![cond; 7].join(","), vec![cond; 6].join(","));
        assert!(err(&many).message().contains("13 conditions; the limit is 12"));
    }
    #[test] fn optimize_grid_is_capped_like_the_book_optimizer() {
        let big = with(r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ]"#,
            r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":301,"step":1}, {"path":"entry.all[1].rhs.p.length","from":1,"to":200,"step":1} ]"#);
        assert!(err(&big).message().contains("more than 50000 combinations"));
        let twice = with(r#"{"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1}"#, r#"{"path":"entry.all[0].lhs.p.length","from":2,"to":3,"step":1},{"path":"entry.all[0].lhs.p.length","from":2,"to":3,"step":1}"#);
        assert!(err(&twice).message().contains("listed twice"));
        let value = parse_spec(&with("entry.all[0].lhs.p.length\",\"from\":2,\"to\":6", "entry.all[0].rhs.value\",\"from\":5,\"to\":15")).unwrap();
        assert_eq!((value.optimize[0].values.len(), current(&value, &value.optimize[0].target)), (11, 10.));
    }
    #[test] fn unbounded_optimize_ranges_are_rejected() {
        for (f, t, st) in ["0,1e30,1", "-1e308,1e308,1", "0,1,1e-300"].iter().map(|r| { let v: Vec<&str> = r.split(',').collect(); (v[0], v[1], v[2]) }) {
            let e = err(&with("\"from\":2,\"to\":6,\"step\":1", &format!("\"from\":{f},\"to\":{t},\"step\":{st}")));
            assert_eq!(e.status(), "invalid");
            assert!(e.message().contains("bad range"), "{:?}", e.message());
        }
        assert_eq!(parse_spec(EXAMPLE).unwrap().optimize[0].values.len(), 5);
    }
    #[test] fn engine_errors_map_to_types() {
        assert_eq!(LabError::from("needs_data: no VIX".to_string()), LabError::NeedsData("no VIX".into()));
        assert_eq!(LabError::from("empty date range".to_string()).status(), "invalid");
        let j = LabError::Invalid("bad \"x\"\n".into()).json();
        assert!(j.starts_with("{\"status\":\"invalid\",\"error\":\"bad \\\"x\\\"\\u000a\",\"label\":\"User-defined indicator strategy"), "{j}");
    }

    // ---- Phase 7: nodes (spec §4.1–§4.3)
    const ONE_EXIT: &str = r#"{"v":1,"entry":{"all":[NODE]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]}}"#;
    #[test] fn nodes_parse_and_name_their_fields() {
        let s = parse_spec(r#"{"v":1,"entry":{"all":[
            {"any":[{"lhs":{"s":"close"},"op":"=","rhs":{"value":1}},{"not":{"lhs":{"s":"close"},"op":"crosses","rhs":{"s":"open"}}}]},
            {"lhs":{"s":"rsi"},"op":"between","lo":{"value":20},"hi":{"value":40}},
            {"lhs":{"s":"close"},"op":"rising","bars":3},
            {"at_least":3,"of":5,"when":{"lhs":{"s":"close"},"op":">","rhs":{"s":"open"}}},
            {"then":[{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}},{"lhs":{"s":"rsi"},"op":">","rhs":{"value":50}}],"within":10}]},
            "exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]}}"#).unwrap();
        assert!(matches!(&s.entry[0], Node::Any(v) if matches!(&v[1], Node::Not(_))));
        assert!(matches!(&s.entry[1], Node::Range { outside: false, .. }));
        assert!(matches!(&s.entry[2], Node::Trend { bars: 3, rising: true, .. }));
        assert!(matches!(&s.entry[3], Node::AtLeast { k: 3, of: 5, .. }));
        assert!(matches!(&s.entry[4], Node::Then { within: 10, .. }));
        for (node, needle) in [
            (r#"{"all":[]}"#, "entry.all[0].all needs at least one entry"),
            (r#"{"at_least":4,"of":3,"when":{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}}"#, "at_least (4) cannot exceed of (3)"),
            (r#"{"then":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}],"within":3}"#, "then needs 2 to 4 steps"),
            (r#"{"lhs":{"s":"close"},"op":"rising","bars":0}"#, "entry.all[0].bars must be a whole number from 1"),
            (r#"{"lhs":{"s":"close"},"op":"between","lo":{"value":1}}"#, "entry.all[0].hi is required"),
            (r#"{"lhs":{"s":"close"},"op":"between","lo":{"value":1},"hi":{"value":2},"rhs":{"value":3}}"#, "unknown field 'rhs' in entry.all[0]"),
            (r#"{"not":{"lhs":{"s":"bars_held"},"op":">","rhs":{"value":1}}}"#, "only available as a plain comparison"),
            (r#"{"maybe":[]}"#, "must be a comparison (lhs, op, rhs) or a group (all, any, not, at_least, then)"),
        ] {
            let e = err(&ONE_EXIT.replace("NODE", node));
            assert_eq!(e.status(), "invalid", "{node}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
        // bars_held stays a top-level exit operand: inside an exit group it is refused
        let grouped = ONE_EXIT.replace("NODE", r#"{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}"#)
            .replace(r#""any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]"#, r#""any":[{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]}]"#);
        assert!(err(&grouped).message().contains("only available as a plain comparison"));
    }
    #[test] fn whole_spec_bounds_count_nested_comparisons_and_nodes() {
        let c = r#"{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}"#;
        let group = |n: usize| format!(r#"{{"all":[{}]}}"#, vec![c; n].join(","));
        let spec = |items: Vec<String>| format!(r#"{{"v":1,"entry":{{"all":[{}]}},"exit":{{"any":[{c}]}}}}"#, items.join(","));
        assert!(parse_spec(&spec(vec![group(13), group(13), group(13)])).is_ok()); // 39 nested + 1 exit = 40 comparisons
        assert!(err(&spec(vec![group(13), group(13), group(14)])).message().contains("more than 40 comparisons"));
        let nots = format!("{}{c}{}", r#"{"not":"#.repeat(10), "}".repeat(10)); // 11 nodes, 1 comparison
        assert!(err(&spec(vec![nots; 11])).message().contains("more than 120 comparisons, groups and expressions"));
        // exactly 120 nodes (119 in entry + 1 in exit) pass, 121 are refused
        let chain = |n: usize| format!("{}{c}{}", r#"{"not":"#.repeat(n - 1), "}".repeat(n - 1));
        let mut ok: Vec<String> = vec![chain(11); 10];
        ok.push(chain(9));
        assert!(parse_spec(&spec(ok.clone())).is_ok());
        ok.pop();
        ok.push(chain(10));
        assert!(err(&spec(ok)).message().contains("more than 120 comparisons, groups and expressions"));
    }
    #[test] fn optimize_reaches_numbers_inside_nodes() {
        let text = r#"{"v":1,"entry":{"all":[{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":10}}]},{"at_least":2,"of":3,"when":{"lhs":{"s":"close"},"op":"rising","bars":2}}]},
            "exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},
            "optimize":[{"path":"entry.all[0].any[0].rhs.value","from":5,"to":15,"step":5},{"path":"entry.all[0].any[0].lhs.p.length","from":2,"to":3,"step":1},{"path":"entry.all[1].when.bars","from":1,"to":3,"step":1},{"path":"entry.all[1].at_least","from":1,"to":3,"step":1}]}"#;
        let s = parse_spec(text).unwrap();
        assert_eq!(s.optimize.iter().map(|d| current(&s, &d.target)).collect::<Vec<_>>(), vec![10., 2., 2., 2.]);
        let t = apply(&s, &s.optimize, &[15., 3., 3., 1.]);
        let Node::Any(v) = &t.entry[0] else { panic!() };
        let c = v[0].cmp().unwrap();
        let Operand::Series(r) = &c.lhs else { panic!() };
        assert_eq!((&c.rhs, r.params["length"]), (&Operand::Value(15.), 3.));
        assert!(matches!(&t.entry[1], Node::AtLeast { k: 1, of: 3, when } if matches!(**when, Node::Trend { bars: 3, .. })));
        let one = |path: &str, from: f64, to: f64| text.split(r#""optimize":"#).next().unwrap().to_string()
            + &format!(r#""optimize":[{{"path":"{path}","from":{from},"to":{to},"step":1}}]}}"#);
        for (path, from, to, needle) in [
            ("entry.all[1].at_least", 1., 4., "at_least (4) cannot exceed of (3)"),
            ("entry.all[1].when.bars", 0., 2., "entry.all[1].when.bars must be a whole number from 1"),
            ("entry.all[0].any[3].rhs.value", 1., 2., "names no number in the spec"),
            ("entry.all[0].any[0].lhs.p.len", 1., 2., "rsi has no parameter 'len'"),
            ("entry.all[2].lhs.p.length", 1., 2., "no condition at that index"),
        ] {
            let e = err(&one(path, from, to));
            assert!(e.message().contains(needle), "{path}: {needle:?} not in {:?}", e.message());
        }
    }

    #[test] fn optimizer_keeps_node_dimensions_consistent_and_useless_nodes_are_refused() {
        let base = r#"{"v":1,"entry":{"all":[{"at_least":2,"of":3,"when":{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}},{"then":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}},{"lhs":{"s":"close"},"op":"<","rhs":{"value":9}},{"lhs":{"s":"close"},"op":">","rhs":{"value":2}}],"within":4}]},
            "exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"optimize":[OPT]}"#;
        let d = |p: &str, f: f64, t: f64| format!(r#"{{"path":"{p}","from":{f},"to":{t},"step":1}}"#);
        let run = |opts: Vec<String>| parse_spec(&base.replace("OPT", &opts.join(",")));
        // each dimension is valid alone, but the worst pairing is not
        let e = run(vec![d("entry.all[0].at_least", 1., 3.), d("entry.all[0].of", 2., 4.)]).unwrap_err();
        assert!(e.message().contains("entry.all[0].at_least") && e.message().contains("entry.all[0].of"), "{}", e.message());
        assert!(run(vec![d("entry.all[0].at_least", 1., 2.), d("entry.all[0].of", 2., 4.)]).is_ok());
        // within below steps-1 is refused value by value
        assert!(run(vec![d("entry.all[1].within", 2., 5.)]).is_ok());
        assert!(run(vec![d("entry.all[1].within", 1., 5.)]).unwrap_err().message().contains("within"));
        // a ref shift may be optimized down to 0
        let r = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"ref":{"s":"close"},"bars":2}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"optimize":[{"path":"entry.all[0].rhs.bars","from":0,"to":3,"step":1}]}"#;
        assert!(parse_spec(r).is_ok());
        // an optimized constant bound may not cross the other constant bound
        let range = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":"between","lo":{"value":2},"hi":{"value":5}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"optimize":[{"path":"entry.all[0].lo.value","from":1,"to":TO,"step":1}]}"#;
        assert!(parse_spec(&range.replace("TO", "5")).is_ok());
        assert!(err(&range.replace("TO", "6")).message().contains("lo (6) is above hi (5)"));
        for (node, needle, status) in [
            (r#"{"lhs":{"s":"close"},"op":"between","lo":{"value":5},"hi":{"value":2}}"#, "lo (5) is above hi (2)", "invalid"),
            (r#"{"lhs":{"s":"close"},"op":"outside","lo":{"value":5},"hi":{"value":2}}"#, "lo (5) is above hi (2)", "invalid"),
            (r#"{"then":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}},{"lhs":{"s":"close"},"op":">","rhs":{"value":1}},{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}],"within":1}"#, "within (1) is shorter", "invalid"),
            (r#"{"lhs":{"add":[{"value":1},{"value":2}]},"op":">","rhs":{"value":1}}"#, "combines two numbers", "invalid"),
            (r#"{"lhs":{"abs":{"value":-1}},"op":">","rhs":{"s":"close"}}"#, "entry.all[0].lhs.abs is a number", "invalid"),
            (r#"{"lhs":{"sum":{"value":1},"bars":3},"op":">","rhs":{"s":"close"}}"#, "entry.all[0].lhs.sum is a number", "invalid"),
            (r#"{"lhs":{"ref":{"value":1},"bars":3},"op":">","rhs":{"s":"close"}}"#, "entry.all[0].lhs.ref is a number", "invalid"),
            (r#"{"lhs":{"abs":{"s":"close"},"symbol":"X"},"op":">","rhs":{"value":1}}"#, "unknown field 'symbol'", "invalid"),
        ] {
            let e = err(&ONE_EXIT.replace("NODE", node));
            assert_eq!(e.status(), status, "{node}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
    }

    // ---- Phase 7: expressions (spec §4.1)
    #[test] fn expressions_parse_validate_and_reach_optimize() {
        let s = parse_spec(&ONE_EXIT.replace("NODE", r#"{"lhs":{"sum":{"s":"rsi","p":{"length":2}},"bars":3},"op":"<","rhs":{"mul":[{"s":"atr"},{"value":2}]}}"#)).unwrap();
        let c = s.entry[0].cmp().unwrap();
        assert!(matches!(&c.lhs, Operand::Expr(e) if matches!(**e, Expr::Win(Win::Sum, Operand::Series(_), 3))));
        assert!(matches!(&c.rhs, Operand::Expr(e) if matches!(**e, Expr::Bin(Bin::Mul, Operand::Series(_), Operand::Value(_)))));
        for (lhs, needle, status) in [
            (r#"{"add":[{"value":1}]}"#, "entry.all[0].lhs.add must be a list of two operands", "invalid"),
            (r#"{"sum":{"s":"close"}}"#, "entry.all[0].lhs.bars must be a whole number from 1", "invalid"),
            (r#"{"sum":{"s":"close"},"bars":3,"x":1}"#, "unknown field 'x' in entry.all[0].lhs", "invalid"),
            (r#"{"ref":{"s":"close"},"bars":-1}"#, "entry.all[0].lhs.bars must be a whole number from 0", "invalid"),
            (r#"{"abs":{"s":"bars_held"}}"#, "only available as a plain comparison", "invalid"),
            (r#"{"s":"close","symbol":7}"#, "symbol must be a string", "invalid"),
        ] {
            let e = err(&ONE_EXIT.replace("NODE", &format!(r#"{{"lhs":{lhs},"op":">","rhs":{{"value":1}}}}"#)));
            assert_eq!(e.status(), status, "{lhs}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
        let base = ONE_EXIT.replace("NODE", r#"{"lhs":{"sum":{"s":"rsi","p":{"length":2}},"bars":3},"op":"<","rhs":{"add":[{"s":"close"},{"value":5}]}}"#);
        let text = format!(r#"{},"optimize":[{{"path":"entry.all[0].lhs.bars","from":2,"to":4,"step":1}},{{"path":"entry.all[0].lhs.sum.p.length","from":2,"to":3,"step":1}},{{"path":"entry.all[0].rhs.add[1].value","from":1,"to":9,"step":4}}]}}"#, &base[..base.len() - 1]);
        let s = parse_spec(&text).unwrap();
        assert_eq!(s.optimize.iter().map(|d| current(&s, &d.target)).collect::<Vec<_>>(), vec![3., 2., 5.]);
        let t = apply(&s, &s.optimize, &[4., 3., 9.]);
        assert_eq!(crate::lab::series::describe(t.entry[0].cmp().unwrap()), "sum(rsi(length=3), 4) < add(close, 9)");
    }

    #[test] fn state_operands_parse_and_reach_optimize() {
        let base = ONE_EXIT.replace("NODE", r#"{"lhs":{"value_when":{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}},"of":{"s":"low"},"nth":2},"op":"<","rhs":{"s":"close"}}"#);
        let text = format!(r#"{},"optimize":[{{"path":"entry.all[0].lhs.value_when.rhs.value","from":20,"to":30,"step":10}},{{"path":"entry.all[0].lhs.nth","from":1,"to":3,"step":1}}]}}"#, &base[..base.len() - 1]);
        let s = parse_spec(&text).unwrap();
        assert_eq!(s.optimize.iter().map(|d| current(&s, &d.target)).collect::<Vec<_>>(), vec![30., 2.]);
        for (lhs, needle) in [
            (r#"{"value_when":{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}},"of":{"s":"low"},"nth":21}"#, "entry.all[0].lhs.nth must be a whole number from 1 to 20"),
            (r#"{"count":{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}}}"#, "entry.all[0].lhs.bars must be a whole number from 1"),
            (r#"{"bars_since":{"value":1}}"#, "entry.all[0].lhs.bars_since must be a comparison"),
            (r#"{"highest_since":{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}}"#, "entry.all[0].lhs.of is required"),
        ] {
            let e = err(&ONE_EXIT.replace("NODE", &format!(r#"{{"lhs":{lhs},"op":">","rhs":{{"value":1}}}}"#)));
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
        // a node inside a state operand stays consistent under optimize
        let nested = ONE_EXIT.replace("NODE", r#"{"lhs":{"bars_since":{"at_least":2,"of":3,"when":{"lhs":{"s":"close"},"op":">","rhs":{"s":"open"}}}},"op":">","rhs":{"value":1}}"#);
        let opt = |to: u32| format!(r#"{},"optimize":[{{"path":"entry.all[0].lhs.bars_since.at_least","from":1,"to":{to},"step":1}}]}}"#, &nested[..nested.len() - 1]);
        assert!(parse_spec(&opt(3)).is_ok());
        assert!(err(&opt(4)).message().contains("at_least (4) cannot exceed of (3)"));
        let vw = ONE_EXIT.replace("NODE", r#"{"lhs":{"value_when":{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}},"of":{"s":"low"},"nth":2},"op":"<","rhs":{"s":"close"}}"#);
        let nth = |to: u32| format!(r#"{},"optimize":[{{"path":"entry.all[0].lhs.nth","from":1,"to":{to},"step":1}}]}}"#, &vw[..vw.len() - 1]);
        assert!(parse_spec(&nth(20)).is_ok());
        assert!(err(&nth(21)).message().contains("nth (21) must be a whole number from 1 to 20"));
    }

    // ---- Phase 7: two-sided specs (spec §6.1)
    const BOTH: &str = r#"{"v":1,"direction":"both",
      "entry":{"all":[{"lhs":{"s":"sma","p":{"length":5}},"op":"crosses_above","rhs":{"s":"sma","p":{"length":20}}}]},"exit":{"any":[],"stop_loss":{"pct":5}},
      "short_side":{"entry":{"all":[{"lhs":{"s":"sma","p":{"length":5}},"op":"crosses_below","rhs":{"s":"sma","p":{"length":20}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":10}}],"target":{"pct":8}}},
      "execution":{"opposite":"exit"}}"#;
    #[test] fn two_sided_specs_parse_and_validate() {
        let s = parse_spec(BOTH).unwrap();
        let x = s.short_side.as_ref().unwrap();
        assert!(!s.exec.short && x.exec.short);
        assert_eq!((s.exec.stop_loss, x.exec.target, x.exec.stop_loss), (Some(Dist::Pct(5.)), Some(Dist::Pct(8.)), None));
        assert_eq!((s.exec.opposite, x.exec.opposite, x.exec.fill), (Opposite::Exit, Opposite::Exit, Fill::NextOpen));
        assert_eq!(x.exit[0].cmp().unwrap().lhs, Operand::BarsHeld);
        assert_eq!(parse_spec(&BOTH.replace(r#"{"opposite":"exit"}"#, "{}")).unwrap().exec.opposite, Opposite::Reverse);
        let no_short = r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":1}}]}}"#;
        for (text, needle) in [
            (BOTH.replace(r#""direction":"both","#, ""), "short_side applies to direction \"both\" only"),
            (no_short.to_string(), "needs short_side"),
            (no_short.replace(r#""direction":"both","#, "").replace(r#"{"value":1}}]}}"#, r#"{"value":1}}]},"execution":{"opposite":"exit"}}"#), "execution.opposite applies to direction \"both\" only"),
            (BOTH.replace(r#""direction":"both""#, r#""direction":"sideways""#), "\"long\", \"short\" or \"both\""),
            (BOTH.replace(r#""opposite":"exit""#, r#""opposite":"flip""#), "execution.opposite must be"),
            (BOTH.replace(r#""target":{"pct":8}"#, r#""target":{"pct":0}"#), "short_side.exit.target.pct must be a number above 0"),
            (BOTH.replace(r#""crosses_below""#, r#""below""#), "short_side.entry.all[0].op"),
        ] {
            let e = err(&text);
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
    }
    /// Every place an operand can appear reaches `series_ids` (the api's `adaptations` note reads it).
    #[test] fn series_ids_reach_every_operand_location() {
        let sl = r#"{"lhs":{"s":"sessions_left","out":"m"},"op":"<","rhs":{"value":2}}"#;
        let short_entry = BOTH.replacen(r#"{"lhs":{"s":"sma","p":{"length":5}},"op":"crosses_below""#, &format!(r#"{sl},{{"lhs":{{"s":"sma","p":{{"length":5}}}},"op":"crosses_below""#), 1);
        let short_exit = BOTH.replacen(r#""exit":{"any":[{"lhs":{"s":"bars_held"}"#, &format!(r#""exit":{{"any":[{sl},{{"lhs":{{"s":"bars_held"}}"#), 1);
        // entry order levels (`at`), on the only side and on the short side
        let stop_at = with(r#""all": ["#, r#""stop": {"at": {"s":"sessions_left","out":"m"}}, "all": ["#).replace(r#""fill": "next_open", "#, "");
        let short_limit_at = BOTH.replacen(r#""short_side":{"entry":{"#, r#""short_side":{"entry":{"limit":{"at":{"max":[{"s":"high"},{"s":"sessions_left","out":"m"}]}},"#, 1);
        for text in [short_entry, short_exit, stop_at, short_limit_at] {
            assert!(text.contains("sessions_left"));
            assert!(series_ids(&parse_spec(&text).unwrap()).iter().any(|id| id == "sessions_left"), "{text}");
        }
    }
    #[test] fn optimize_reaches_the_short_side() {
        let text = format!(r#"{},"optimize":[{{"path":"short_side.exit.target.pct","from":4,"to":8,"step":2}},{{"path":"short_side.entry.all[0].rhs.p.length","from":10,"to":30,"step":10}},{{"path":"exit.stop_loss.pct","from":1,"to":2,"step":1}}]}}"#, &BOTH[..BOTH.len() - 1]);
        let s = parse_spec(&text).unwrap();
        assert_eq!(s.optimize.iter().map(|d| current(&s, &d.target)).collect::<Vec<_>>(), vec![8., 20., 5.]);
        let t = apply(&s, &s.optimize, &[4., 30., 1.]);
        let x = t.short_side.as_ref().unwrap();
        let Operand::Series(r) = &x.entry[0].cmp().unwrap().rhs else { panic!() };
        assert_eq!((x.exec.target, r.params["length"], t.exec.stop_loss), (Some(Dist::Pct(4.)), 30., Some(Dist::Pct(1.))));
        assert!(err(&text.replace("short_side.exit.target.pct", "short_side.exit.stop_loss.pct")).message().contains("the spec has no exit.stop_loss"));
        assert!(err(&text.replace("short_side.entry.all[0]", "short_side.entry.all[3]")).message().contains("no condition at that index"));
    }

    // ---- Phase 7: entry orders (spec §6.1)
    #[test] fn entry_orders_parse_and_validate() {
        let no_fill = |s: &str| s.replace(r#""fill": "next_open", "#, "");
        let s = parse_spec(&no_fill(&with(r#""all": ["#, r#""stop": {"points": 2.5}, "limit": {"at": {"s":"highest","p":{"length":5}}}, "valid_bars": 3, "all": ["#))).unwrap();
        assert_eq!((s.exec.stop, s.exec.valid_bars, s.exec.limit), (Some(Dist::Points(2.5)), 3, None));
        assert!(matches!(&s.exec.limit_at, Some(Operand::Series(r)) if r.id == "highest"));
        let s = parse_spec(&with(r#""fill": "next_open", "cost_bps": 12"#, r#""fill": "next_open", "delay_bars": 2, "entry_on": "edge""#)).unwrap();
        assert_eq!((s.exec.delay, s.exec.edge, s.exec.valid_bars), (2, true, 1));
        for (text, needle) in [
            (with(r#""all": ["#, r#""stop": {"pct": 1}, "all": ["#), "remove execution.fill"),
            (with(r#""all": ["#, r#""valid_bars": 2, "all": ["#), "entry.valid_bars applies to entry.limit or entry.stop orders"),
            (no_fill(&with(r#""all": ["#, r#""stop": {"pct": 1}, "valid_bars": 21, "all": ["#)), "valid_bars must be a whole number from 1 to 20"),
            (no_fill(&with(r#""all": ["#, r#""stop": {"pct": 1, "points": 2}, "all": ["#)), "exactly one of pct or atr, or points"),
            (with(r#""any": ["#, r#""stop_loss": {"at": {"s":"low"}}, "any": ["#), "exit.stop_loss.at is for entry orders"),
            (with(r#""any": ["#, r#""stop_loss": {"points": 0}, "any": ["#), "exit.stop_loss.points must be a number above 0"),
            (with(r#""cost_bps": 12"#, r#""cost_bps": 12, "delay_bars": 0"#), "delay_bars must be a whole number from 1 to 10"),
            (with(r#""fill": "next_open""#, r#""fill": "close", "delay_bars": 2"#), "delay_bars applies to next_open fills only"),
            (with(r#""cost_bps": 12"#, r#""cost_bps": 12, "entry_on": "rising""#), "entry_on must be"),
        ] {
            let e = err(&text);
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
        let opt = no_fill(&with(r#""all": ["#, r#""stop": {"at": {"s":"highest","p":{"length":5}}}, "limit": {"points": 1}, "all": ["#))
            .replace(r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ]"#, r#"[ {"path":"entry.stop.at.p.length","from":3,"to":9,"step":3}, {"path":"entry.limit.points","from":0,"to":2,"step":1} ]"#);
        let s = parse_spec(&opt).unwrap();
        assert_eq!(s.optimize.iter().map(|d| current(&s, &d.target)).collect::<Vec<_>>(), vec![5., 1.]);
        let t = apply(&s, &s.optimize, &[9., 0.]);
        assert!(matches!(&t.exec.stop_at, Some(Operand::Series(r)) if r.params["length"] == 9.));
        assert_eq!(t.exec.limit, Some(Dist::Points(0.)));
        // a node inside an `at` level stays consistent under optimize, on either side
        let vw = r#"{"value_when":{"lhs":{"s":"close"},"op":">","rhs":{"s":"open"}},"of":{"s":"high"},"nth":2}"#;
        let lvl = no_fill(&with(r#""all": ["#, &format!(r#""stop": {{"at": {vw}}}, "all": ["#)))
            .replace(r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ]"#, r#"[ {"path":"entry.stop.at.nth","from":1,"to":TO,"step":1} ]"#);
        assert!(parse_spec(&lvl.replace("TO", "20")).is_ok());
        assert!(err(&lvl.replace("TO", "21")).message().contains("nth (21) must be a whole number from 1 to 20"));
        let short = BOTH.replacen(r#""short_side":{"entry":{"#, &format!(r#""short_side":{{"entry":{{"stop":{{"at":{vw}}},"#), 1);
        let short = format!(r#"{},"optimize":[{{"path":"short_side.entry.stop.at.nth","from":19,"to":21,"step":1}}]}}"#, &short[..short.len() - 1]);
        assert!(err(&short).message().contains("nth (21) must be a whole number from 1 to 20"));
    }

    // ---- Phase 7: exits (spec §6.1)
    #[test] fn exits_parse_validate_and_reach_optimize() {
        let s = parse_spec(&with(r#""any": ["#, r#""trailing": {"pct": 8, "activate": {"atr": 2}}, "breakeven": {"after": {"pct": 3}, "plus": {"points": 0.5}}, "take_profits": [{"pct": 5, "size": 0.5}, {"atr": 4, "size": 0.25}], "min_bars": 2, "any": ["#)
            .replace(r#""cost_bps": 12"#, r#""cost_bps": 12, "intrabar": "ohlc_path""#)).unwrap();
        assert_eq!((s.exec.trailing, s.exec.trail_activate), (Some(Dist::Pct(8.)), Some(Dist::Atr { mult: 2., length: 14 })));
        assert_eq!(s.exec.breakeven, Some((Dist::Pct(3.), Some(Dist::Points(0.5)))));
        assert_eq!(s.exec.take_profits, vec![(Dist::Pct(5.), 0.5), (Dist::Atr { mult: 4., length: 14 }, 0.25)]);
        assert_eq!((s.exec.min_bars, s.exec.ohlc_path), (2, true));
        for (text, needle) in [
            (with(r#""any": ["#, r#""take_profits": [{"pct": 5, "size": 0.6}, {"pct": 9, "size": 0.4}], "any": ["#), "sizes must add up to less than 1"),
            (with(r#""any": ["#, r#""take_profits": [{"pct": 5}], "any": ["#), "exit.take_profits[0].size must be a fraction"),
            (with(r#""any": ["#, r#""take_profits": [{"pct": 1, "size": 0.1}, {"pct": 2, "size": 0.1}, {"pct": 3, "size": 0.1}, {"pct": 4, "size": 0.1}], "any": ["#), "needs 1 to 3 levels"),
            (with(r#""any": ["#, r#""breakeven": {"plus": {"pct": 1}}, "any": ["#), "exit.breakeven.after is required"),
            (with(r#""any": ["#, r#""trailing": {"pct": 8, "activate": {"pct": 0}}, "any": ["#), "exit.trailing.activate.pct must be a number above 0"),
            (with(r#""any": ["#, r#""min_bars": 1.5, "any": ["#), "exit.min_bars must be a whole number from 0"),
            (with(r#""cost_bps": 12"#, r#""cost_bps": 12, "intrabar": "random""#), "execution.intrabar must be"),
        ] {
            let e = err(&text);
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
        let opt = with(r#""any": ["#, r#""trailing": {"pct": 8, "activate": {"pct": 2}}, "breakeven": {"after": {"pct": 3}}, "take_profits": [{"pct": 5, "size": 0.5}], "any": ["#)
            .replace(r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ]"#, r#"[ {"path":"exit.trailing.activate.pct","from":1,"to":3,"step":1}, {"path":"exit.breakeven.after.pct","from":2,"to":4,"step":1}, {"path":"exit.take_profits[0].pct","from":4,"to":6,"step":1}, {"path":"exit.trailing.pct","from":6,"to":8,"step":2} ]"#);
        let s = parse_spec(&opt).unwrap();
        assert_eq!(s.optimize.iter().map(|d| current(&s, &d.target)).collect::<Vec<_>>(), vec![2., 3., 5., 8.]);
        let t = apply(&s, &s.optimize, &[1., 4., 6., 6.]);
        assert_eq!((t.exec.trail_activate, t.exec.breakeven.unwrap().0, t.exec.take_profits[0].0, t.exec.trailing), (Some(Dist::Pct(1.)), Dist::Pct(4.), Dist::Pct(6.), Some(Dist::Pct(6.))));
        assert!(err(&opt.replace("exit.take_profits[0].pct", "exit.take_profits[1].pct")).message().contains("the spec has no exit.take_profits[1]"));
        // fix round 1: breakeven.plus below after (same units), no `at` for plus, precise optimize paths
        for (text, needle) in [
            (with(r#""any": ["#, r#""breakeven": {"after": {"pct": 2}, "plus": {"pct": 2}}, "any": ["#), "exit.breakeven.plus (2) must be below exit.breakeven.after (2)"),
            (with(r#""any": ["#, r#""breakeven": {"after": {"pct": 2}, "plus": {"at": {"s":"low"}}}, "any": ["#), "exit.breakeven.plus takes pct|atr|points; `at` levels are not supported"),
            (opt.replace("exit.take_profits[0].pct", "exit.nowhere.pct"), "entry.stop, entry.stop.at, exit.stop_loss, exit.target, exit.trailing, exit.trailing.activate, exit.breakeven.after, exit.take_profits[k]"),
        ] {
            let e = err(&text);
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
        assert!(parse_spec(&with(r#""any": ["#, r#""breakeven": {"after": {"pct": 2}, "plus": {"points": 3}}, "any": ["#)).is_ok()); // units differ: not compared
        // fix round 2: the optimizer cannot move after to or below plus
        let be = with(r#""any": ["#, r#""breakeven": {"after": {"pct": 3}, "plus": {"pct": 2}}, "any": ["#)
            .replace(r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ]"#, r#"[ {"path":"exit.breakeven.after.pct","from":FROM,"to":4,"step":1} ]"#);
        assert!(parse_spec(&be.replace("FROM", "3")).is_ok());
        let e = err(&be.replace("FROM", "1"));
        assert!(e.message().contains("exit.breakeven.after.pct") && e.message().contains("plus (2) must be below exit.breakeven.after (1)"), "{}", e.message());
        let at = with(r#""all": ["#, r#""stop": {"at": {"s":"highest","p":{"length":5}}}, "all": ["#).replace(r#""fill": "next_open", "#, "")
            .replace(r#""entry.all[0].lhs.p.length""#, r#""entry.stop.at""#);
        assert!(err(&at).message().contains("entry.stop.at is a level operand; optimize a number inside it"), "{}", err(&at).message());
    }

    // ---- Phase 7: scale-in and unit sizing (spec §6.1, §7.4)
    #[test] fn scale_in_and_sizing_parse_and_validate() {
        let s = parse_spec(&with(r#""all": ["#, r#""scale_in": {"legs": [0.5, 0.3, 0.2], "min_bars": 2, "price": "better"}, "all": ["#)
            .replace(r#""cost_bps": 12"#, r#""cost_bps": 12, "sizing": {"pct_equity": 50}"#)).unwrap();
        assert_eq!(s.exec.scale_in, Some(ScaleIn { legs: vec![0.5, 0.3, 0.2], min_bars: 2, price: LegPrice::Better }));
        assert_eq!(s.exec.sizing, Sizing::PctEquity(50.));
        for (text, needle) in [
            (with(r#""all": ["#, r#""scale_in": {"legs": [0.6, 0.6]}, "all": ["#), "legs must be a list of 2 to 5 fractions"),
            (with(r#""all": ["#, r#""scale_in": {"legs": [1]}, "all": ["#), "legs must be a list of 2 to 5 fractions"),
            (with(r#""all": ["#, r#""scale_in": {"legs": [0.5, 0.5], "price": "cheap"}, "all": ["#), "price must be"),
            (with(r#""all": ["#, r#""scale_in": {"legs": [0.5, 0.5]}, "all": ["#).replace(r#""any": ["#, r#""take_profits": [{"pct": 5, "size": 0.5}], "any": ["#), "cannot combine with exit.take_profits"),
            (with(r#""cost_bps": 12"#, r#""cost_bps": 12, "sizing": {"risk_pct": 1}"#), "risk_pct needs an exit.stop_loss"),
            (with(r#""cost_bps": 12"#, r#""cost_bps": 12, "sizing": {"units": 10}"#), "need execution.capital"),
            (with(r#""cost_bps": 12"#, r#""cost_bps": 12, "sizing": {"pct_equity": 150}"#), "pct_equity must be a number above 0 and at most 100"),
            (with(r#""cost_bps": 12"#, r#""cost_bps": 12, "sizing": {}"#), "exactly one of pct_equity, risk_pct, units or value"),
        ] {
            let e = err(&text);
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
        let opt = with(r#""cost_bps": 12"#, r#""cost_bps": 12, "sizing": {"pct_equity": 50}"#)
            .replace(r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ]"#, r#"[ {"path":"execution.sizing.pct_equity","from":25,"to":100,"step":25} ]"#);
        let s = parse_spec(&opt).unwrap();
        assert_eq!((current(&s, &s.optimize[0].target), apply(&s, &s.optimize, &[75.]).exec.sizing), (50., Sizing::PctEquity(75.)));
        assert!(err(&opt.replace(r#""to":100"#, r#""to":125"#)).message().contains("must be above 0 and at most 100"));
    }

    // ---- Phase 7: re-entry and risk rules (spec §6.1, §6.5)
    #[test] fn reentry_and_risk_rules_parse_and_validate() {
        let s = parse_spec(&with(r#""cost_bps": 12"#, r#""cost_bps": 12, "cooldown": {"bars": 3, "after": "loss"}, "max_trades": {"count": 2, "per": "M"}, "risk": {"max_consecutive_losses": 4, "pause_bars": 20, "max_drawdown_pct": 25}"#)).unwrap();
        assert_eq!((s.exec.cooldown, s.exec.max_trades), (Some((3, After::Loss)), Some((2, 2))));
        assert_eq!(s.exec.risk, Risk { losses: Some((4, 20)), max_dd_pct: Some(25.) });
        for (extra, needle) in [
            (r#""cooldown": {"bars": 0}"#, "execution.cooldown.bars must be a whole number from 1"),
            (r#""cooldown": {"bars": 2, "after": "draw"}"#, "execution.cooldown.after must be"),
            (r#""max_trades": {"count": 2, "per": "D"}"#, "execution.max_trades.per must be W, M, Q or Y"),
            (r#""risk": {"max_consecutive_losses": 3}"#, "pause_bars must be a whole number"),
            (r#""risk": {"pause_bars": 3}"#, "pause_bars goes with max_consecutive_losses"),
            (r#""risk": {"max_drawdown_pct": 100}"#, "max_drawdown_pct must be a number above 0 and below 100"),
        ] {
            let e = err(&with(r#""cost_bps": 12"#, &format!(r#""cost_bps": 12, {extra}"#)));
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
    }
    // ---- Phase 7: money mode (spec §7.4)
    #[test] fn execution_limits_parse_in_money_mode_only() {
        let money = |limits: &str| with(r#""fill": "next_open", "cost_bps": 12"#, &format!(r#""fill": "next_open", "capital": 100000, "limits": {limits}"#));
        let s = parse_spec(&money(r#"{"min_shares":5,"round_lot":10,"min_value":100,"max_value":5000,"max_volume_pct":2.5,"shrink":true,"prev_bar_equity":true,"interest_pct":4}"#)).unwrap();
        assert_eq!(s.exec.limits, Some(Limits { min_shares: 5., round_lot: 10., min_value: 100., max_value: Some(5000.), max_volume_pct: Some(2.5), shrink: true, prev_bar_equity: true, interest_pct: 4., margin_pct: 100., margin_rate_pct: 0. }));
        assert_eq!(parse_spec(&money("{}")).unwrap().exec.limits, Some(Limits::default()));
        for (limits, msg) in [(r#"{"round_lot":0.5}"#, "round_lot must be a whole number"), (r#"{"max_volume_pct":101}"#, "max_volume_pct must be"), (r#"{"shrink":1}"#, "shrink must be true or false"),
            (r#"{"min_value":10,"max_value":5}"#, "min_value must not exceed max_value"), (r#"{"lots":1}"#, "unknown field 'lots'")] {
            assert!(err(&money(limits)).message().contains(msg), "{limits}");
        }
        assert_eq!(parse_spec(&money(r#"{"max_value":0,"max_volume_pct":0}"#)).unwrap().exec.limits, Some(Limits::default()), "AmiBroker: zero means no limit");
        let unit = err(&with(r#""cost_bps": 12"#, r#""cost_bps": 12, "limits": {}"#));
        assert_eq!((unit.status(), unit.message()), ("invalid", "execution.limits needs money mode (execution.capital)"));
        let port = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":10}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"capital":1000000,"limits":{"round_lot":10}},"portfolio":{"universe":{"basket":"nifty50"},"max_open":5}}"#;
        assert_eq!(err(port).status(), "unsupported");
    }
    #[test] fn account_margin_tick_size_and_pad_align_parse() {
        let money = |limits: &str| with(r#""fill": "next_open", "cost_bps": 12"#, &format!(r#""fill": "next_open", "capital": 100000, "limits": {limits}"#));
        assert_eq!(parse_spec(&money(r#"{"margin_pct":50}"#)).unwrap().exec.limits.unwrap().margin_pct, 50.);
        assert_eq!(parse_spec(&money(r#"{"margin_pct":50,"margin_rate_pct":9}"#)).unwrap().exec.limits.unwrap().margin_rate_pct, 9.);
        for m in ["-1", "101", "true"] { assert!(err(&money(&format!(r#"{{"margin_rate_pct":{m}}}"#))).message().contains("margin_rate_pct must be a number from 0 to 100"), "{m}"); }
        for m in ["0", "101", "true"] { assert!(err(&money(&format!(r#"{{"margin_pct":{m}}}"#))).message().contains("margin_pct must be a number from 1 to 100"), "{m}"); }
        let exec = |extra: &str| with(r#""cost_bps": 12"#, &format!(r#""cost_bps": 12, {extra}"#));
        let s = parse_spec(&exec(r#""tick_size": 0.05, "pad_align": true"#)).unwrap();
        assert_eq!((s.exec.tick_size, s.exec.pad_align), (0.05, true));
        assert_eq!(parse_spec(&exec(r#""tick_size": 0"#)).unwrap().exec.tick_size, 0., "AmiBroker: 0 means no rounding");
        for (extra, msg) in [(r#""tick_size": -1"#, "tick_size must be a number"), (r#""pad_align": 1"#, "pad_align must be true or false")] { assert!(err(&exec(extra)).message().contains(msg), "{extra}"); }
        let fut = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":0}}]},"exit":{"any":[],"stop_loss":{"ticks":20}},"execution":{"tick_size":0.05,"instrument":{"kind":"futures","roll":{"days_before_expiry":2},"adjust":"ratio"}}}"#;
        assert!(err(fut).message().contains("contract's tick"), "{}", err(fut).message());
        let port = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":10}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"capital":1000000,"pad_align":true},"portfolio":{"universe":{"basket":"nifty50"},"max_open":5}}"#;
        assert_eq!(err(port).status(), "unsupported");
    }
    #[test] fn money_mode_parses_capital_sizing_and_costs() {
        let base = with(r#""fill": "next_open", "cost_bps": 12"#, r#""fill": "next_open", "capital": 100000, "sizing": {"units": 10},
            "costs": {"commission": {"per_order": 20, "pct": 0.03}, "slippage": {"bps": 5}, "taxes": [{"name": "STT", "pct": 0.1, "side": "both"}], "borrow_pct_year": 0.5}"#);
        let s = parse_spec(&base).unwrap();
        assert_eq!((s.exec.capital, s.exec.sizing), (Some(100000.), Sizing::Units(10.)));
        assert_eq!(s.exec.costs, MoneyCosts { per_order: 20., pct: 0.03, slip_bps: 5., taxes: vec![("STT".into(), 0.1, TaxSide::Both)], borrow_pct_year: Some(0.5), ..MoneyCosts::default() });
        for (from, to, needle) in [
            (r#""capital": 100000"#, r#""capital": 0"#, "execution.capital must be a number above 0"),
            (r#""capital": 100000, "#, "", "execution.costs applies to money mode (execution.capital)"),
            (r#""bps": 5"#, r#""bps": 5, "ticks": 1"#, "exactly one of bps or ticks"),
            (r#""side": "both""#, r#""side": "all""#, "side must be buy, sell or both"),
            (r#""name": "STT""#, r#""name": "STT; DROP""#, "name must be 1 to 40 letters"),
            (r#""pct": 0.03"#, r#""pct": 11"#, "commission.pct must be a number from 0 to 10"),
            (r#""units": 10"#, r#""units": 2.5"#, "units must be a whole number"),
        ] {
            let e = err(&base.replace(from, to));
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
        assert!(err(&with(r#""cost_bps": 12"#, r#""cost_bps": 12, "capital": 5000"#)).message().contains("remove execution.cost_bps"));
        assert!(err(&with(r#""cost_bps": 12"#, r#""cost_bps": 12, "costs": {}"#)).message().contains("applies to money mode"));
    }
    #[test] fn always_in_specs_need_reversal() {
        let always = r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":3}}}]},"exit":{"any":[]},"short_side":{"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_below","rhs":{"s":"sma","p":{"length":3}}}]},"exit":{"any":[]}}}"#;
        assert!(parse_spec(always).unwrap().exit.is_empty());
        for opposite in ["exit","ignore"] {
            let text=always.replacen("\"v\":1,", &format!("\"v\":1,\"execution\":{{\"opposite\":\"{opposite}\"}},"),1);
            assert!(err(&text).message().contains("needs execution.opposite \"reverse\""));
        }
        let protected = always.replace("\"any\":[]", "\"any\":[],\"stop_loss\":{\"pct\":2}");
        assert!(parse_spec(&protected.replacen("\"v\":1,", "\"v\":1,\"execution\":{\"opposite\":\"ignore\"},",1)).is_ok(), "protective exits let both sides stand alone");
        let single=r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}]},"exit":{"any":[]}}"#;
        assert!(err(single).message().contains("needs at least one condition"));
    }

}

#[cfg(test)]
mod phase8_grammar_tests {
    use super::*;
    const PORT: &str=r#"{"v":1,"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":10}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"capital":1000000},"portfolio":{"universe":{"basket":"nifty50"},"score":{"abs":{"s":"rsi","p":{"length":14}}},"rank":"lowest","max_open":5}}"#;
    fn rotation() -> String { PORT.replace(r#""all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":10}}]"#,r#""all":[]"#).replace(r#""any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]"#,r#""any":[]"#).replace(r#""max_open":5"#,r#""max_open":5,"mode":"rotation""#) }
    fn error(s:&str)->LabError {parse_spec(s).err().expect("invalid spec was accepted")}
    fn optimize(s:&str,items:&str)->String {s.replacen(r#""v":1,"#, &format!(r#""v":1,"optimize":[{items}],"#),1)}
    #[test] fn foreign_fields_reach_nested_expand_short_and_price_levels() {
        let text=r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"add":[{"s":"close","symbol":"A","fixup":"previous"},{"abs":{"s":"rsi","symbol":"B"}}]},"op":">","rhs":{"value":0}}],"limit":{"at":{"s":"close","symbol":"C","tf":"W","offset":1,"expand":"first"}}},"exit":{"any":[]},"short_side":{"entry":{"all":[{"not":{"lhs":{"s":"close","symbol":"D"},"op":"<","rhs":{"value":0}}}],"stop":{"at":{"highest_since":{"lhs":{"s":"close","symbol":"D"},"op":">","rhs":{"value":0}},"of":{"s":"close","symbol":"D"}}}},"exit":{"any":[]}}}"#;
        let s=parse_spec(text).unwrap();assert_eq!(s.foreign_symbols(),vec!["A","B","C","D"]);assert_eq!(s.fixup_symbols().into_iter().collect::<Vec<_>>(),vec!["A"]);
        let paths:Vec<_>=s.foreign_locations().into_iter().map(|(p,_)|p).collect();
        assert!(paths.iter().any(|p|p=="entry.limit.at"));assert!(paths.iter().any(|p|p=="short_side.entry.stop.at.of"));
        assert!(error(&text.replace(r#""of":{"s":"close","symbol":"D"}"#,r#""of":{"s":"close","symbol":"E"}"#)).message().contains("limit is 4"));
    }
    #[test] fn foreign_names_and_chart_validation_share_limits() {
        assert_eq!(foreign_ref("series",Some("SPX"),None,Some("previous")).unwrap(),Foreign{symbol:Some("SPX".into()),vs:None,fixup:true});
        for (symbol,vs,fixup) in [(Some(""),None,None),(Some("A\n"),None,None),(None,None,Some("previous")),(Some("SPX"),None,Some("bad")),(None,Some(""),None)] {assert!(foreign_ref("series",symbol,vs,fixup).is_err());}
        assert!(foreign_ref("series",Some(&"A".repeat(81)),None,None).is_err());
        for leaf in [r#"{"s":"close","symbol":7}"#,r#"{"s":"close","symbol":"SPX","fixup":true}"#,r#"{"s":"sma","vs":"SPX"}"#,r#"{"s":"close","vs":"SPX"}"#] {
            let t=PORT.replace(r#"{"s":"rsi","p":{"length":2}}"#,leaf);assert_eq!(error(&t).status(),"invalid");
        }
    }
    #[test] fn portfolio_rotation_defaults_and_expression_score_preserve_context() {
        let s=parse_spec(PORT).unwrap();let p=s.portfolio.as_ref().unwrap();assert_eq!((&p.universe,p.mode,p.lowest,p.max_open,p.worst_rank_held()),(&Universe::Basket("nifty50".into()),Mode::Signals,true,5,5));
        assert!(matches!(p.score,Some(Operand::Expr(_))));assert!(series_ids(&s).contains(&"rsi".into()));
        let s=parse_spec(&rotation().replace(r#"{"abs":{"s":"rsi","p":{"length":14}}}"#,r#"{"s":"close","tf":"Q","offset":1,"expand":"first"}"#)).unwrap();assert!(s.entry.is_empty()&&s.exit.is_empty());assert_eq!(s.portfolio.as_ref().unwrap().mode,Mode::Rotation);
        assert!(matches!(s.portfolio.as_ref().unwrap().score,Some(Operand::Expr(_))));
    }
    #[test] fn portfolio_score_and_lineage_are_never_silently_dropped() {
        for score in [r#"{"value":1}"#,r#"{"s":"bars_held"}"#,r#"{"s":"close","symbol":"NIFTY 50"}"#,r#"{"add":[{"s":"close"},{"s":"close","symbol":"SPX"}]}"#] {
            assert_eq!(error(&PORT.replace(r#"{"abs":{"s":"rsi","p":{"length":14}}}"#,score)).status(),"invalid");
        }
        let lineage=PORT.replacen(r#""v":1,"#,r#""v":1,"origin":{"book":"stc9","method":"m","template_version":1},"#,1);let e=error(&lineage);assert_eq!(e.status(),"unsupported");assert!(e.message().contains("origin"));
        let mut s=parse_spec(PORT).unwrap();let mut r=match s.entry[0].cmp().unwrap().lhs.clone(){Operand::Series(r)=>r,_=>panic!()};r.foreign.vs=Some("SPX".into());s.portfolio.as_mut().unwrap().score=Some(Operand::Series(r));assert_eq!(s.foreign_symbols(),vec!["SPX"]);
    }
    #[test] fn portfolio_state_scores_need_a_series_inside_their_condition() {
        let scores=[
            r#"{"bars_since":{"lhs":{"value":1},"op":"between","lo":{"value":0},"hi":{"value":2}}}"#,
            r#"{"count":{"lhs":{"value":1},"op":"outside","lo":{"value":0},"hi":{"value":2}},"bars":3}"#,
            r#"{"count":{"lhs":{"value":1},"op":"rising","bars":2},"bars":3}"#,
        ];
        for score in scores {
            let text=PORT.replace(r#"{"abs":{"s":"rsi","p":{"length":14}}}"#,score);
            let e=error(&text);assert_eq!(e.status(),"invalid");assert!(e.message().contains("portfolio.score needs at least one series"),"{e:?}");
            for actual in [r#"{"s":"close"}"#,r#"{"s":"close","tf":"Q","offset":1,"expand":"first"}"#] {
                let valid=text.replacen(r#"{"value":1}"#,actual,1);
                assert!(parse_spec(&valid).is_ok(),"state scores with a real series remain supported: {valid}");
            }
        }
    }
    #[test] fn portfolio_refuses_unimplemented_execution_combinations() {
        for execution in [r#"{"capital":1000,"delay_bars":2}"#,r#"{"capital":1000,"cooldown":{"bars":2}}"#,r#"{"capital":1000,"max_trades":{"count":2,"per":"M"}}"#,r#"{"capital":1000,"risk":{"max_drawdown_pct":5}}"#,r#"{"capital":1000,"instrument":{"kind":"futures"}}"#] {assert_eq!(error(&PORT.replace(r#"{"capital":1000000}"#,execution)).status(),"unsupported");}
        for order in [r#""limit":{"at":{"s":"close"}},"#,r#""stop":{"pct":2},"#,r#""scale_in":{"legs":[0.5,0.5]},"#] {assert_eq!(error(&PORT.replacen(r#""entry":{"all"#, &format!(r#""entry":{{{order}"all"#),1)).status(),"unsupported");}
        assert_eq!(error(&PORT.replacen(r#""v":1,"#,r#""v":1,"direction":"short","#,1)).status(),"unsupported");
        assert!(error(&PORT.replace(r#"{"capital":1000000}"#,r#"{}"#)).message().contains("capital"));
        let t=rotation().replace(r#""any":[]"#,r#""any":[],"breakeven":{"after":{"pct":2}}"#);assert!(error(&t).message().contains("rotation"));
    }
    #[test] fn portfolio_universe_field_and_work_caps_are_enforced() {
        for u in [r#"{"basket":"bad basket!"}"#,r#"{"tickers":["A"]}"#,r#"{"tickers":["A","A"]}"#,r#"{"basket":"x","tickers":["A","B"]}"#] {assert_eq!(error(&PORT.replace(r#"{"basket":"nifty50"}"#,u)).status(),"invalid");}
        let list=format!(r#"{{"tickers":[{}]}}"#,(0..51).map(|i|format!("\"S{i}\"")).collect::<Vec<_>>().join(","));assert!(error(&PORT.replace(r#"{"basket":"nifty50"}"#,&list)).message().contains("50"));
        for n in ["0","51","2.5","1e100"] {assert!(error(&PORT.replace(r#""max_open":5"#,&format!(r#""max_open":{n}"#))).message().contains("max_open"));}
        assert!(error(&optimize(PORT,r#"{"path":"portfolio.score.abs.p.length","from":1,"to":201,"step":1}"#)).message().contains("200"));
    }
    #[test] fn portfolio_optimizer_tracks_omitted_worst_and_validates_all_combinations() {
        let r=rotation();let grid=r#"{"path":"portfolio.max_open","from":2,"to":6,"step":2},{"path":"portfolio.score.abs.p.length","from":5,"to":20,"step":5}"#;
        let s=parse_spec(&optimize(&r,grid)).unwrap();let a=apply(&s,&s.optimize,&[6.,10.]);let p=a.portfolio.as_ref().unwrap();assert_eq!((p.max_open,p.worst_rank_held()),(6,6));assert!(validate_applied(&a).is_ok());
        assert_eq!(current(&s,&s.optimize[1].target),14.);
        let explicit=r.replace(r#""max_open":5"#,r#""max_open":5,"worst_rank_held":8"#);
        let valid=r#"{"path":"portfolio.max_open","from":10,"to":12,"step":1},{"path":"portfolio.worst_rank_held","from":20,"to":25,"step":1}"#;assert!(parse_spec(&optimize(&explicit,valid)).is_ok());
        let invalid=r#"{"path":"portfolio.max_open","from":2,"to":10,"step":2},{"path":"portfolio.worst_rank_held","from":6,"to":12,"step":2}"#;assert!(error(&optimize(&explicit,invalid)).message().contains("worst_rank_held"));
        let s=parse_spec(&optimize(&explicit,valid)).unwrap();assert!(validate_applied(&apply(&s,&s.optimize,&[12.,3.])).is_err());
    }
    #[test] fn score_expressions_share_the_existing_rule_part_budget() {
        let mut score=r#"{"s":"close"}"#.to_string();for _ in 0..6{score=format!(r#"{{"add":[{score},{score}]}}"#);}
        let leaf=r#"{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}"#;
        let group=format!(r#"{{"all":[{{"all":[{leaf}]}},{{"all":[{leaf}]}}]}}"#);
        let entry=format!(r#""all":[{}]"#,vec![group;12].join(","));
        let text=PORT.replace(r#"{"abs":{"s":"rsi","p":{"length":14}}}"#,&score).replace(r#""all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":10}}]"#,&entry);
        let e=error(&text);assert!(e.message().contains("limit is 120"),"{e:?}");
    }
}
#[cfg(test)]
mod amibroker_trades_tests {
    use super::*;
    const BASE: &str = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":0}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]},"execution":{}}"#;
    fn ex(execution: &str) -> Result<Spec, LabError> { parse_spec(&BASE.replace(r#""execution":{}"#, &format!(r#""execution":{execution}"#))) }
    fn msg(execution: &str) -> String { ex(execution).unwrap_err().message().to_string() }
    #[test] fn prices_and_delays_parse_per_side_with_todays_defaults() {
        assert_eq!(ex("{}").unwrap().exec.market, None);
        let s = ex(r#"{"prices":{"buy":"high","sell":"average"},"delays":{"buy":2,"sell":0}}"#);
        assert_eq!(s.unwrap_err().message(), r#"execution.prices.sell "average" with delay 0 would use prices before the signal (a signal is known at the bar's close); use "close" or a delay of 1 or more"#);
        let s = ex(r#"{"prices":{"buy":"high","sell":"close"},"delays":{"buy":2,"sell":0}}"#).unwrap();
        assert_eq!(s.exec.market, Some([(Px::High, 2), (Px::Close, 0)]));
        // a missing key keeps today's default: the next bar's open
        assert_eq!(ex(r#"{"prices":{"buy":"low"}}"#).unwrap().exec.market, Some([(Px::Low, 1), (Px::Open, 1)]));
        let short = BASE.replace(r#""v":1,"#, r#""v":1,"direction":"short","#).replace(r#""execution":{}"#, r#""execution":{"prices":{"short":"average","cover":"close"},"delays":{"cover":0}}"#);
        assert_eq!(parse_spec(&short).unwrap().exec.market, Some([(Px::Average, 1), (Px::Close, 0)]));
        // a side the spec does not trade is refused, not ignored (and its delay 0 is not checked)
        assert_eq!(parse_spec(&short.replace(r#""short":"average","#, r#""buy":"high","#)).unwrap_err().message(), "execution.prices.buy applies to direction long or both");
        assert_eq!(msg(r#"{"prices":{"short":"low"}}"#), "execution.prices.short applies to direction short or both");
        assert_eq!(msg(r#"{"delays":{"cover":0}}"#), "execution.delays.cover applies to direction short or both");
        let both = r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":0}}]},"exit":{"any":[]},"short_side":{"entry":{"all":[{"lhs":{"s":"close"},"op":"<","rhs":{"value":0}}]},"exit":{"any":[]}},"execution":{"prices":{"buy":"close","sell":"high","short":"low","cover":"average"},"delays":{"buy":0,"sell":3,"short":2,"cover":1}}}"#;
        let s = parse_spec(both).unwrap();
        assert_eq!((s.exec.market, s.short_side.unwrap().exec.market), (Some([(Px::Close, 0), (Px::High, 3)]), Some([(Px::Low, 2), (Px::Average, 1)])));
    }
    #[test] fn delay_zero_needs_close_and_the_two_forms_do_not_mix() {
        for p in ["open", "high", "low", "average"] {
            assert!(msg(&format!(r#"{{"prices":{{"buy":"{p}"}},"delays":{{"buy":0}}}}"#)).ends_with("with delay 0 would use prices before the signal (a signal is known at the bar's close); use \"close\" or a delay of 1 or more"));
            assert_eq!(ex(&format!(r#"{{"prices":{{"buy":"{p}"}},"delays":{{"buy":0}}}}"#)).unwrap_err().status(), "invalid");
        }
        assert!(msg(r#"{"delays":{"sell":0}}"#).starts_with(r#"execution.prices.sell "open" with delay 0"#)); // the default field is the open
        for (execution, needle) in [
            (r#"{"fill":"close","prices":{"buy":"close"}}"#, "replace execution.fill, exit_fill and delay_bars; give one form"),
            (r#"{"delay_bars":2,"delays":{"buy":2}}"#, "give one form"),
            (r#"{"exit_fill":"close","delays":{"sell":1}}"#, "give one form"),
            (r#"{"prices":{"buy":"vwap"}}"#, r#"execution.prices.buy must be "open", "high", "low", "close" or "average""#),
            (r#"{"delays":{"sell":21}}"#, "execution.delays.sell must be a whole number from 0 to 20"),
            (r#"{"delays":{"sell":1.5}}"#, "execution.delays.sell must be a whole number"),
            (r#"{"prices":{"entry":"open"}}"#, "unknown field 'entry' in execution.prices"),
        ] { assert!(msg(execution).contains(needle), "{needle:?} not in {:?}", msg(execution)); }
        let limit = BASE.replace(r#""all":[{"#, r#""limit":{"pct":1},"all":[{"#).replace(r#""execution":{}"#, r#""execution":{"prices":{"buy":"close"},"delays":{"buy":1}}"#);
        assert!(parse_spec(&limit).unwrap_err().message().contains("execution.prices.buy / execution.delays.buy apply to market entries"));
        let limit_exit = limit.replace(r#""prices":{"buy":"close"},"delays":{"buy":1}"#, r#""prices":{"sell":"close"},"delays":{"sell":0}"#);
        assert_eq!(parse_spec(&limit_exit).unwrap().exec.market, Some([(Px::Open, 1), (Px::Close, 0)]));
    }
    #[test] fn trailing_profit_pct_parses_and_optimizes() {
        let t = |trailing: &str| parse_spec(&BASE.replace(r#""exit":{"any":["#, &format!(r#""exit":{{"trailing":{trailing},"any":["#)));
        assert_eq!(t(r#"{"profit_pct":40}"#).unwrap().exec.trailing, Some(Dist::ProfitPct(40.)));
        assert_eq!(t(r#"{"profit_pct":40,"activate":{"pct":2}}"#).unwrap().exec.trail_activate, Some(Dist::Pct(2.)));
        for (bad, needle) in [(r#"{"profit_pct":0}"#, "exit.trailing.profit_pct must be a number above 0 and below 100"), (r#"{"profit_pct":100}"#, "below 100"), (r#"{"profit_pct":40,"pct":2}"#, "unknown field 'pct' in exit.trailing")] {
            assert!(t(bad).unwrap_err().message().contains(needle), "{needle}");
        }
        let opt = BASE.replace(r#""exit":{"any":["#, r#""exit":{"trailing":{"profit_pct":40},"any":["#).replace(r#""v":1,"#, r#""v":1,"optimize":[{"path":"exit.trailing.profit_pct","from":20,"to":60,"step":20}],"#);
        let s = parse_spec(&opt).unwrap();
        assert_eq!((s.optimize[0].values.clone(), current(&s, &s.optimize[0].target)), (vec![20., 40., 60.], 40.));
        assert_eq!(apply(&s, &s.optimize, &[60.]).exec.trailing, Some(Dist::ProfitPct(60.)));
        assert!(parse_spec(&opt.replace(r#""to":60"#, r#""to":100"#)).unwrap_err().message().contains("exit.trailing value 100 is out of range"));
        assert!(parse_spec(&opt.replace("exit.trailing.profit_pct", "exit.trailing.pct")).unwrap_err().message().contains("exit.trailing is set by profit_pct"));
    }
    #[test] fn bars_held_may_sit_on_the_right_hand_side() {
        let s = parse_spec(&BASE.replace(r#"{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}"#, r#"{"lhs":{"value":5},"op":"<=","rhs":{"s":"bars_held"}}"#)).unwrap();
        assert_eq!(s.exit[0].cmp().unwrap().rhs, Operand::BarsHeld);
    }
    #[test] fn reentry_parses_and_portfolios_refuse_the_new_fields() {
        assert_eq!(ex(r#"{"reentry":{"stop_loss":3,"bars":2}}"#).unwrap().exec.reentry, Some([3, 0, 0, 2]));
        assert_eq!(ex(r#"{"reentry":{"stop_loss":1,"target":2,"trailing":3,"bars":4},"cooldown":{"bars":5}}"#).unwrap().exec.reentry, Some([1, 2, 3, 4]));
        assert!(msg(r#"{"reentry":{"breakeven":1}}"#).contains("unknown field 'breakeven' in execution.reentry"));
        assert!(msg(r#"{"reentry":{"target":-1}}"#).contains("execution.reentry.target must be a whole number from 0"));
        let port = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":10}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"capital":1000000},"portfolio":{"universe":{"basket":"nifty50"},"score":{"abs":{"s":"rsi","p":{"length":14}}},"rank":"lowest","max_open":5}}"#;
        for extra in [r#""prices":{"buy":"close"}"#, r#""delays":{"sell":2}"#, r#""reentry":{"stop_loss":2}"#] {
            assert_eq!(parse_spec(&port.replace(r#""capital":1000000"#, &format!(r#""capital":1000000,{extra}"#))).unwrap_err().status(), "unsupported");
        }
    }
}

#[cfg(test)]
mod formula_operand_tests {
    use super::*;
    const F: &str = r#"Fast = Param(\"Fast\", 9, 2, 50, 1)\nPlot(EMA(C, Fast) - EMA(C, 21), \"Osc\")\nPlot(C, \"Px\", \"price\")"#;
    fn spec(operand: &str, optimize: &str) -> String {
        format!(r#"{{"v":1,"entry":{{"all":[{{"lhs":{operand},"op":">","rhs":{{"value":0}}}}]}},"exit":{{"any":[{{"lhs":{{"s":"bars_held"}},"op":">=","rhs":{{"value":5}}}}]}}{optimize}}}"#)
    }
    fn op(extra: &str) -> String { format!(r#"{{"formula":"{F}","name":"Mom"{extra}}}"#) }
    #[test] fn formula_operands_parse_with_timing_params_and_output() {
        let s = parse_spec(&spec(&op(r#","p":{"Fast":12},"out":"Px","tf":"W","offset":2"#), "")).unwrap();
        let Operand::Series(r) = &s.entry[0].cmp().unwrap().lhs else { panic!() };
        assert_eq!((r.id.as_str(), r.out, r.offset, r.tf, r.params["Fast"]), ("formula", 1, 2, Tf::W, 12.));
        assert_eq!(r.formula.as_ref().unwrap().name, "Mom");
        let s = parse_spec(&spec(&op(r#","tf":"W","offset":1,"expand":"first""#), "")).unwrap();
        assert!(matches!(&s.entry[0].cmp().unwrap().lhs, Operand::Expr(e) if matches!(**e, Expr::Expand(..))));
        assert!(series_ids(&s).contains(&"formula".to_string()));
        // a formula operand sits inside expressions, groups and the portfolio score like any series operand
        assert!(parse_spec(&spec(&format!(r#"{{"sum":{},"bars":3}}"#, op("")), "")).is_ok());
    }
    #[test] fn formula_operand_errors_name_the_field_and_position() {
        let e = |extra: &str| parse_spec(&spec(&op(extra), "")).unwrap_err().message().to_string();
        assert_eq!(e(r#","p":{"Fast":1}"#), "entry.all[0].lhs.formula: Fast must be between 2 and 50");
        assert_eq!(e(r#","p":{"Slow":3}"#), "entry.all[0].lhs.formula: unknown parameter 'Slow'; parameters: Fast");
        assert_eq!(e(r#","out":"Nope""#), "entry.all[0].lhs.formula: out: Mom has plots Osc, Px");
        assert!(e(r#","symbol":"X""#).contains("symbol"));
        assert!(e(r#","expand":"first","tf":"W""#).contains("lookahead"));
        let bad = parse_spec(&spec(r#"{"formula":"x = C\nPlot(Ref(x, 2), \"A\")"}"#, "")).unwrap_err();
        assert_eq!(bad.message(), "entry.all[0].lhs.formula: line 2, col 13: Ref(x, 2) would read 2 bars in the future; use Ref(x, -n) for n bars ago");
        assert!(parse_spec(&spec(r#"{"formula":3}"#, "")).unwrap_err().message().contains("formula must be a string"));
    }
    #[test] fn formula_text_counts_toward_the_spec_size() {
        let long = format!(r#"{{"formula":"Plot(C, \"X\") // {}"}}"#, "x".repeat(3900));
        let pad = format!(r#"{{"add":[{{"add":[{long},{long}]}},{long}]}}"#);
        assert!(parse_spec(&spec(&long, "")).is_ok());
        assert!(parse_spec(&spec(&pad, "")).unwrap_err().message().contains("the limit is 8192"));
    }
    #[test] fn optimize_reaches_formula_params() {
        let opt = r#","optimize":[{"path":"entry.all[0].lhs.p.Fast","from":5,"to":15,"step":5}]"#;
        let s = parse_spec(&spec(&op(""), opt)).unwrap();
        assert_eq!(s.optimize[0].values, vec![5., 10., 15.]);
        assert_eq!(current(&s, &s.optimize[0].target), 9.);
        let a = apply(&s, &s.optimize, &[15.]);
        let Operand::Series(r) = &a.entry[0].cmp().unwrap().lhs else { panic!() };
        assert_eq!(r.params["Fast"], 15.);
        assert_eq!(crate::lab::series::describe_node(&a.entry[0]), "Mom(Fast=15).Osc > 0");
        let out = |o: &str| parse_spec(&spec(&op(""), &opt.replace(r#""from":5,"to":15,"step":5"#, o))).unwrap_err().message().to_string();
        assert!(out(r#""from":1,"to":5,"step":1"#).contains("Fast must be between 2 and 50"));
        assert!(out(r#""from":2,"to":3,"step":0.5"#).contains("Fast must be a whole number"));
        let missing = parse_spec(&spec(&op(""), &opt.replace("p.Fast", "p.Slow"))).unwrap_err();
        assert!(missing.message().contains("Mom has no parameter 'Slow'"), "{}", missing.message());
    }
}
