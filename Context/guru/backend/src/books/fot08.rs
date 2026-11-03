//! Futures & Options Trader magazine (futuresandoptionstrader.com), systems of Apr 2008 - Mar 2010 ("Trading Strategies" and
//! "Futures Trading System Lab"). Research share Trading Knowledge I/@Newsletters/Futures & Options Trader Magazine 2005 - 2010
//! (futuresandoptionstrader.com)/2008, /2009, /2010; text authors/titles-1030/K15w/src/fo*.txt (pdftotext of the article pages).
//! Book id `fot08`. Lane K15 (titles loop, 2 Nov 2026), to-code rows 276-288; lead decisions 88, 19; independent check
//! authors/titles-1030/K15w/k15_ref.py. Orders use the engine in `ctm`.
use crate::connors::{atr, dmi, rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::ctm::{ent, ext, fin, g, orders, K};
/// Simple averages as plain window sums (msws::sma), so equal averages stay exactly equal.
use super::msws::sma;
use super::lc3::{hi, lo, week_no};
use super::tselhs::Book;

/// Admitted ids (authors/titles-1030/K15w/k15_ref.py (header)).
pub const REVIEWED: &[&str] = &["fot08_keyrev", "fot08_volbo", "fot08_pretrend", "fot08_3bar", "fot08_gapfade", "fot08_nerves", "fot08_bbtrend",
    "fot08_rsi9", "fot08_tnote", "fot08_six", "fot08_triple", "fot08_keltner_w", "fot08_hlr"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! fo { () => { " Orders placed at a close are for the next bar only (market = the open; a stop at its price or the open if the open is through it; a limit at its price or the better open; one fill per bar, the order the price reaches first, the extreme nearer the open first; a market exit at the open comes first). One position, one unit; the articles' contract counts and equity-risk sizing are not applied. The magazine tested futures (or the ETFs/stocks named on the card); daily indices, ETFs and US/India stocks are a declared application (lead decision 88); the vault's standard costs; the articles' figures are theirs (gate book field), not check targets." } }

const KR: &str = concat!("[Declared readings] F&OT Apr 2008 p.8-10, 'Key reversal' (daily-bar strategy): a bullish key reversal day (low below yesterday's low, close above yesterday's high) -> buy at that close (market on close) with a GTC sell stop 1.5 x the key reversal day's range below the entry and a GTC limit 2 x the range above it, kept until one fills (from the next bar); bearish (high above yesterday's high, close below yesterday's low) is the mirror. Flat only. The 10-minute version is intraday and held.", fo!());
const VB: &str = concat!("[Declared readings] F&OT May 2008 p.8-11, 'Trading volatility breakouts' (soybeans): setup when the 6-day historical volatility is below half the 100-day one, HV = the (population) standard deviation of closing prices ('Historical volatility is typically represented by the standard deviation of closing prices'). Then for the next day a buy stop at the close + 0.5 x ATR(14) (Wilder) and a sell-short stop at the close - 0.5 x ATR(14), whichever is reached first. The rules box says 'above the opening price' while the text and the worked example (entry 13.69 1/2 = 22 1/2 cents above the Feb. 13 close) use the previous close: the close is the default, p.ref = 1 the next open; p.k = 0.68 is the article's 'more conservative' 68 percent. Protective stop 0.5 x ATR(14) (signal day's) from the entry, from the next bar; exit at the close when it is below (long) / above (short) the 7-day EMA (seeded with the first close), checked from the entry day's close. SUBSET: the add-ons every 0.5 ATR (3/2/1 contracts) and their trailing of the stop are not applied (single unit).", fo!());
const PT: &str = concat!("[Declared readings] F&OT Jul 2008 p.8-10, 'Pre-trend reversal' (Joe Krutsinger), the printed TradeStation code: IF c < c[2] and Range < Average(Range,3) and c < o Then Buy Next Bar at c Stop; the mirror Sell Short; IF MarketPosition <> 0 and BarsSinceEntry >= Walk Then exit Next Bar at Market. Range = high - low; the 3-day average includes today; an opposite stop reverses; a same-side stop while in a trade is ignored. Walk = 1 is the code's default input (p.walk); the article optimised it per market (Table 1: SPY 13 long only, QQQQ 8 long only, S&P futures 49 long only, Nasdaq 100 futures 8, E-mini S&P 32, T-bonds 16, gold 3); p.long = 1 runs the long side only.", fo!());
const TB: &str = concat!("[Declared readings] F&OT Aug 2008 p.8-12, 'Improving the three-bar pullback pattern', the printed 'Three-bar short-term strategy' TradeStation code with its improved entry: TrixLine = trix(C, 9), SignalLine = xaverage(TrixLine, 3), TrendState = up when TrixLine > SignalLine; if TrendState[3] = up and the last three bars closed below their opens: BuyTrigger, BuyPrice = H, SellTrigger off (sell side mirrored with closes above opens, SellPrice = L); 'buy next bar at BuyPrice stop' while BuyTrigger, 'sellshort next bar at SellPrice stop' while SellTrigger; flat -> both triggers off; long -> sell next bar at highest(h,60) limit and at lowest(l,60) stop, BuyTrigger off (short mirrored). The triggers carry over bars as in the code (a SellTrigger set while long stays until filled or flat). TRIX read as Hutson's: 100 x the one-bar change of the triple 9-bar EMA of the close (EMAs seeded with the first close, TRIX = 0 on the first bar); TradeStation's built-in source is not in the share. HELD: the trend-following exit (trailing stop after a $2,000 open profit, lead decision 19).", fo!());
const GF: &str = "[Declared readings] F&OT Sep 2008 p.6-8, 'Fading the opening gap': buy at the open when it is below the lowest low of the past N days (not counting today), sell short at the open when it is above the highest high of the past N days; exit at the same day's close. N = 30, the best of the article's S&P futures table (ROA 764%); p.n = 10 ... 100 are the other tested values. One-day trades. Daily indices, ETFs and US/India stocks are a declared application (lead decision 88); the vault's standard costs.";
const NV: &str = concat!("[Declared readings] F&OT Oct 2008 p.26-28, 'Nerves of steel' (Futures Trading System Lab, E-mini S&P 1997-2003): long at the next open when today's low < yesterday's low < the low before, today's high < yesterday's high < the high before, today's close < yesterday's < the previous < the one before, and today's low is at least 2% below the low two days ago. Exit at the close on a seven-day momentum signal, as the article defines it: 'Signals are triggered when the current close is above the close seven days ago and is in the upper 25 percent of the high-low range of the past seven days' (p.27), i.e. close > the close 7 days ago and (close - lowest low of 7 days)/(highest high of 7 - lowest low of 7) >= 0.75; or at the close 10 days after the entry day. Option mom = 1 uses the rules box's shorter wording ('Exit when the seven-day momentum calculation is 0.75 or higher', p.28): the range ratio alone. SUBSET: 'up to five long positions' (more signals on the following days) is not applied: one unit, flat only.", fo!());
const BB: &str = concat!("[Declared readings] F&OT Nov 2008 p.10-12, 'Two systems are better than one', Bollinger Band breakout: Bollinger Bands = 20-day SMA +/- 2 (population) standard deviations. Flat, close above the upper band -> long at that close; close below the lower band -> short at that close. Exit with a stop order at the 20-day SMA (re-set every day, from the next bar).", fo!());
const RS: &str = concat!("[Declared readings] F&OT Nov 2008 p.10-12, 'Two systems are better than one', RSI countertrend: flat, RSI(9) (Wilder) below 35, close above its 200-day SMA and today's volume below yesterday's -> long at the next open; RSI(9) above 65, close below the 200-day SMA, volume below yesterday's -> short at the next open. Exit at the next open after the RSI closes above 65 (long) / below 35 (short), as the article's example ('The RSI closed above 65 on May 22 and the trade was exited the next day'). STOP HELD: the $7,500 stop-loss (lead decision 19). Needs the instrument's own volume; a day without volume gives no signal.", fo!());
const TN: &str = "[Declared readings] F&OT Jan 2009 p.8-10, 'T-note pullback' long and short patterns, as the article's Table 2 tests them ('applying both the long and short setups', Dec 31 2001 - Jan 2 2009). Long: close > close 2 days ago, close > close 40 days ago, yesterday's close < the close before, which is < the close before that -> buy at the close; exit at the close 7 days after the entry (1-20 days tested, 7 for the longs; p.hold). Short: close > yesterday's close, close < the close 40 days ago, yesterday's close > the close two days ago, today's high > yesterday's high > the high two days ago -> sell short at the close; exit at the close 2 days after the entry ('an arbitrary exit length of two days', p.shold). Reading: the short rule 2 is coded as the text states it twice ('below the close n days ago', 'the current close is less than the close 40 days ago'); its printed '(Close > Close[40])' is taken as a typo (lead decision 163). One position; an opposite signal while in a trade reverses it at that close (TradeStation's behaviour for buy / sell short orders; Table 2's 60 long trades earn less than Table 1's 60, which fits longs cut short by short signals). Option side = 1: the long pattern alone (Table 1). Table 3 (bear market Oct 1998 - Jan 2000) swaps the exits: hold 2, shold 7. 10-year T-note futures in the article; daily indices, ETFs and US/India stocks are a declared application (lead decision 88); the vault's standard costs.";
const SX: &str = concat!("[Declared readings] F&OT Jul 2009 p.10-12, 'The six-signal composite indicator' (Art Collins; Active Trader Nov 2006), the printed TradeStation code: aa = +1 if average(c,2) < average(c,5), -1 if above; bb = +1 if c > average(c,40), -1 if below; cc = +1 if highestbar(c,50) > lowestbar(c,50) (the 50-day low close is more recent: 'This signal buys if the lowest close of the past 50 days was more recent than the highest close'), -1 if less; dd = +1 if (range < average(range,10) and c > c[1]) or (range > average(range,10) and c < c[1]), -1 for the mirror; ee = +1 if c > (average(h,15) + average(l,15))/2, -1 if below; ff = +1 if average(e,3) < 0, -1 if above, with e = +1 on an up open-to-close day, -1 on a down one. Each variable keeps its value when neither side holds (TradeStation variables); ties of highestbar/lowestbar take the most recent bar; the components start once 50 bars exist. Sum > 0 -> buy next bar at market, < 0 -> sell short; setexitonclose: every trade is one day, open to close. p.comp = 1..6 runs one component alone (the article tests each).", fo!());
const TS: &str = concat!("[Declared readings] F&OT Jan 2010 p.8-10, 'The triple screen futures system' (Wealth-Lab, DAX example): long setup at the close: 200-bar SMA above its value 10 bars ago; the low below the 15-bar SMA, 100-bar SMA above its value 10 bars ago and ADX(14) (Wilder) above its value 10 bars ago -> buy stop at the high + 1 tick for the next bar (tick 0.01 US, 0.05 NSE, lead decision 57). Short = the mirror (high above the 15-bar SMA, falling SMAs, rising ADX, sell stop at the low - 1 tick). Exits from the next bar: stop 5% and limit 5% from the signal bar's close; exit at the next open when the 100-bar SMA turns against the trade (below its value 10 bars ago for a long). Flat only. Same Wealth-Lab triple screen as atm03_triple5 (Active Trader Aug 2003) except screen 2: this article says the low 'must be below' the 15-bar SMA where atm03 reads 'cross below'; own card (lead decision 48).", fo!());
const KW: &str = concat!("[Declared readings] F&OT Feb 2010 p.6-8, 'Weekly Keltner channel system' (Active Trader Feb 2004): weekly bars (Monday-based calendar weeks of the daily bars: first open, highest high, lowest low, last close); middle = 10-week SMA of (H+L+C)/3, upper/lower = middle +/- the 10-week SMA of (H-L). At a week's last daily bar: a weekly close above the upper band -> long at the next week's first open, below the lower band -> short (stop and reverse, always in after the first signal); 'crosses above' read as Keltner's own 'buy when price closes above the upper band' (p.6). A week counts once its next week has begun. Same channel as scevkc_a (Evens, daily bars, S&C 1999); this card is the weekly test.", fo!());
const HL: &str = concat!("[Declared readings] F&OT Mar 2010 p.6-8, 'Channel breakout with HLR' (Active Trader): HLR = (close - lowest low of 100 days)/(highest high - lowest low of 100 days), today included ('price's relative location within the high-low range of the past n bars'). Orders for the next day: a buy stop at the highest high of the past 100 days (today included) unless long, a sell-short stop at the lowest low unless short (an opposite breakout reverses); a long exits at the next open when HLR < 0.2 and the close is below the entry price ('the current position is unprofitable'), a short when HLR > 0.8 and the close is above it. The column also supplies the HLR formula that the held card atmmisc_hlr (TSL Sep 2006) lacks; that card is not changed here.", fo!());

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, vol, rules, notes| StrategyDef { id, name, group: "Futures & Options Trader 2008-10 (Trading Strategies, Futures Trading System Lab)", session: 0, dir, status: Status::Live,
        needs_volume: vol, needs_vix: false, needs_series: &[], book: "fot08", rules, notes, claim: None };
    vec![
        d("fot08_keyrev", "Key reversal day: enter at the close, GTC stop 1.5 ranges away and target 2 ranges away", "Long/Short", false, &[
            "F|\"Bullish: a bar with a low below the previous bar's low and a close above the previous bar's high\" (F&OT Apr 2008 p.8)",
            "E|\"1. When a bullish key reversal day forms, buy market on close (MOC) and place a good-till-canceled (GTC) sell-stop order at the entry price minus 1.5 times the key reversal day's range.\" (p.10)",
            "X|\"2. Place a GTC limit order to sell at the entry price plus 2 times the key reversal day's range. As with the 10-minute strategy, the rules are reversed for short trades.\" (p.10)"], KR),
        d("fot08_volbo", "Volatility breakout: after 6-day HV falls under half the 100-day HV, stop entries 0.5 ATR beyond the close; 7-day EMA exit", "Long/Short", false, &[
            "F|\"Setup — If the six-day HV is less than half the 100-day HV\" (F&OT May 2008 p.9)",
            "E|\"Buy when price moves more than 50 percent of the 14-day ATR above the opening price. Sell short when price moves more than 50 percent of the 14-day ATR below the opening price.\" (p.9); text: \"a stop order to buy or sell more than 50 percent of the ATR ... above or below the previous day's close\" (p.8)",
            "X|\"Exit with a loss when price moves 50 percent of the ATR below (for longs) or above (for shorts) the entry price.\" (p.9); \"exit when price closes below (for a long trade) or above (for a short trade) the EMA\" (seven-day EMA, p.10)"], VB),
        d("fot08_pretrend", "Pre-trend reversal (Krutsinger): after a quiet bar against the 2-day move, stop entry at the close, time exit", "Long/Short", false, &[
            "E|\"IF c< c[2] and Range < Average(Range,3) and c <o Then Buy Next Bar at c Stop; IF c >c[2] and Range < Average(Range,3) and c >o Then Sell Short Next Bar at c Stop;\" (F&OT Jul 2008 p.8)",
            "X|\"Input: Walk(1); IF MarketPosition=1 and BarsSinceEntry >=Walk Then Sell Next Bar at Market; IF MarketPosition=-1 and BarsSinceEntry >=Walk Then Buy to Cover Next Bar at Market;\" (p.8)"], PT),
        d("fot08_3bar", "Three-bar pullback (improved entry): in a TRIX up (down) trend, buy (short) a stop at the third pullback bar's high (low); 60-day exits", "Long/Short", false, &[
            "F|\"TrixLine = trix( Price, TrixLength ); SignalLine = xaverage(TrixLine, SignalLineLength); LongCondition = c < o and c[1] < o[1] and c[2] < o[2];\" TrixLength(9), SignalLineLength(3), History(60) (F&OT Aug 2008 p.10)",
            "E|\"if TrendState[3] = UpTrend and LongCondition then begin BuyTrigger = true; BuyPrice = h; ... if BuyTrigger then buy next bar at BuyPrice stop;\" (p.10)",
            "X|\"if marketposition = 1 then begin sell next bar at highest(h, History) limit; sell next bar at lowest(l, History) stop;\" (p.10)"], TB),
        d("fot08_gapfade", "Fading the opening gap: buy an open under the 30-day low, short an open over the 30-day high, out at the close", "Long/Short", false, &[
            "E|\"1. Go long if price opens below the lowest low of the past 10, 20, 30…100 days. 2. Sell short if price opens above the highest high of the past 10, 20, 30…100 days.\" (F&OT Sep 2008 p.7)",
            "X|\"3. Exit at the close.\" (p.7)"], GF),
        d("fot08_nerves", "'Nerves of steel' pullback: three falling bars with a 2% drop, buy the next open; out on 7-day momentum (close above 7 days ago, top quarter of the range) or after 10 days", "Long", false, &[
            "E|\"Go long on the next day's open when: 1. Today's low is below yesterday's low and yesterday's low is below the previous day's low. 2. Today's high is below yesterday's high ... 3. Today's close is below yesterday's close ... 4. Today's low is at least 2 percent below the low two days ago.\" (F&OT Oct 2008 p.28)",
            "X|\"6. Exit when the seven-day momentum calculation is 0.75 or higher. 7. Exit any remaining open positions after 10 days on the close.\" (p.28); \"Signals are triggered when the current close is above the close seven days ago and is in the upper 25 percent of the high-low range of the past seven days. ... Seven-day momentum = (close today-lowest(low,7))/(highest(high,7)-lowest(low,7))\" (p.27)"], NV),
        d("fot08_bbtrend", "Bollinger Band breakout: close outside the 20-day 2-SD band, out on a stop at the 20-day SMA", "Long/Short", false, &[
            "E|\"1. Go long if price closes above the upper Bollinger Band (using a 20-day SMA and two-standard-deviation bands). 2. Sell short if price closes below the lower Bollinger Band.\" (F&OT Nov 2008 p.10)",
            "X|\"3. Exit all trades with a stop-loss order when price reaches the 20-day SMA.\" (p.10)"], BB),
        d("fot08_rsi9", "RSI(9) countertrend with the 200-day trend and falling volume; out when RSI crosses 65 / 35 (dollar stop held)", "Long/Short", true, &[
            "E|\"1. Go long if the nine-day RSI is below 35, price closes above its 200-day SMA, and today's volume is less than yesterday's volume. ... 3. Sell short if the nine-day RSI is above 65, price closes below the 200-day SMA, and today's volume is less than yesterday's volume.\" (F&OT Nov 2008 p.11)",
            "X|\"2. Exit long trade if RSI closes above 65, or with a $7,500 stop-loss. ... 4. Exit short trade if the RSI closes below 35 or with a $7,500 stop-loss.\" (p.11; the dollar stop is held)"], RS),
        d("fot08_tnote", "T-note pullback: buy an up close over 2 and 40 days after two down closes (out 7 days later), short two higher highs and closes below the 40-day close (out 2 days later)", "Long/Short", false, &[
            "E|\"1. Today's close must be above the close two days ago (Close > Close[2]). 2. Today's close must be above the close n days ago (40 in this case; Close > Close[40]). 3. Yesterday's close must be below the previous day's close (Close[1] < Close[2]). 4. The previous day's close must be below the close the day before that (Close[2] < Close[3]). If these conditions are met, buy this bar at the close.\" (F&OT Jan 2009 p.8)",
            "E|\"1. Today's close must be above yesterday's close (Close > Close[1]). 2. Today's close must be below the close n days ago (40 in this case; Close > Close[40]). 3. Yesterday's close must be above the close two days ago (Close[1] > Close[2]). 4. Today's high must be above yesterday's high (High > High[1]). 5. Yesterday's high must be above the high two days ago (High[1] > High[2]). If these conditions are met, sell short on the close.\" (p.8-9; rule 2 as the text states it, lead decision 163)",
            "X|\"Long trades were exited after seven days, short trades after two days.\" (Figure 3 caption, p.9)"], TN),
        d("fot08_six", "Six-signal composite (Collins): sum of six +1/-1 daily biases, trade the next day open to close", "Long/Short", false, &[
            "E|\"if aa+bb+cc+dd+ee+ff>0 then buy next bar at market; if aa+bb+cc+dd+ee+ff<0 then sell short next bar at market;\" with the six component formulas (F&OT Jul 2009 p.10)",
            "X|\"setexitonclose;\" (p.10): \"buy the next open and sell the close\" (Component 1 rule, p.10)"], SX),
        d("fot08_triple", "Triple screen (Wealth-Lab, 2010 version): 200/100-bar SMA trend, low under the 15-bar SMA, rising ADX; 5% stop and target", "Long/Short", false, &[
            "F|\"Screen 1: The 200-bar SMA must be above its value 10 bars ago. Screen 2: The low must be below the 15-bar SMA, the 100-bar SMA must be above its value 10 bars ago, and the 14-bar average directional movement index (ADX) must be above its value 10 bars ago.\" (F&OT Jan 2010 p.8)",
            "E|Screen 3: a buy stop at the previous bar's high plus one tick (p.8)",
            "X|\"1. Stop-loss: Place a stop-loss order 5 percent below the closing price of the bar that generated the entry signal. 2. Profit target: Place a limit order 5 percent above ... 3. Trend reversal: Exit the position if the 100-bar SMA is less than its value 10 bars ago.\" (p.8)"], TS),
        d("fot08_keltner_w", "Weekly Keltner channel: stop and reverse on a weekly close outside the 10-week typical-price channel", "Long/Short", false, &[
            "F|\"The middle line is a 10-bar simple moving average (SMA) of the bar's typical price ... The upper band is the middle line plus the 10-day moving average of the high minus low; the lower band is the middle line minus [it]\" (F&OT Feb 2010 p.6)",
            "E|\"1. Enter long and exit short when price crosses above upper Keltner channel. 2. Enter short and exit long when price crosses below lower Keltner channel.\" (p.6); tested \"on weekly data\" (p.7)", "X|\"a true stop-and-reverse system\" (p.6): the opposite signal"], KW),
        d("fot08_hlr", "Channel breakout with HLR: 100-day breakout stops, early exit of losing trades when HLR reaches the far 20% of the channel", "Long/Short", false, &[
            "E|\"1. Go long tomorrow with a stop at the highest high of the past 100 days. ... 3. Go short tomorrow with a stop at the lowest low of the past 100 days.\" (F&OT Mar 2010 p.6)",
            "X|\"2. Exit long tomorrow at the market if the HLR indicator drops below 0.2 and the current position is unprofitable. ... 4. Exit short tomorrow at the market if the HLR indicator climbs above 0.8 and the current position is unprofitable.\" (p.6)"], HL),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = match id {
        "fot08_keyrev" => ("F&OT key reversal", "1.5-range stop, 2-range target"),
        "fot08_volbo" => ("F&OT volatility breakout", "HV6 < HV100/2, 0.5 ATR stops"),
        "fot08_pretrend" => ("F&OT pre-trend reversal", "quiet counter bar, stop at the close"),
        "fot08_3bar" => ("F&OT three-bar pullback", "TRIX trend, 60-day exits"),
        "fot08_gapfade" => ("F&OT opening gap fade", "open beyond the 30-day range"),
        "fot08_nerves" => ("F&OT nerves of steel", "three falling bars, 2% drop"),
        "fot08_bbtrend" => ("F&OT Bollinger breakout", "close outside 2 SD, SMA stop"),
        "fot08_rsi9" => ("F&OT RSI(9) countertrend", "RSI 35/65, 200-day SMA, volume"),
        "fot08_tnote" => ("F&OT T-note pullback", "two down closes in an uptrend"),
        "fot08_six" => ("F&OT six-signal composite", "six daily biases, open to close"),
        "fot08_triple" => ("F&OT triple screen 2010", "SMA trend, ADX, 5% exits"),
        "fot08_keltner_w" => ("F&OT weekly Keltner", "10-week typical-price channel"),
        "fot08_hlr" => ("F&OT HLR channel breakout", "100-day breakout, HLR exit"),
        _ => return None,
    };
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, f.0, f.1))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "fot08_volbo" => vec![p("k", "Entry distance in ATR(14) (0.5 basic; 0.68 the article's conservative option)", 0.5, 0.3, 1., 0.01),
            p("ref", "Entry reference: 0 = the previous close (text and example); 1 = the next open (rules box)", 0., 0., 1., 1.)],
        "fot08_pretrend" => vec![p("walk", "Time exit Walk (bars since entry; code default 1; article Table 1 per market)", 1., 1., 50., 1.),
            p("long", "1 = long side only (the article's SPY, QQQQ, S&P futures runs); 0 = both", 0., 0., 1., 1.)],
        "fot08_gapfade" => vec![p("n", "Look-back days (30 best on S&P futures; 10-100 tested)", 30., 10., 100., 10.)],
        "fot08_tnote" => vec![p("hold", "Long exit at the close this many days after entry (7; Table 3 uses 2)", 7., 1., 20., 1.),
            p("shold", "Short exit at the close this many days after entry (2; Table 3 uses 7)", 2., 1., 20., 1.),
            p("side", "0 = long and short (Table 2); 1 = long pattern only (Table 1)", 0., 0., 1., 1.)],
        "fot08_nerves" => vec![p("mom", "Momentum exit: 0 = close above the close 7 days ago and in the top 25% of the 7-day range (text); 1 = the range ratio alone (rules box)", 0., 0., 1., 1.)],
        "fot08_six" => vec![p("comp", "0 = composite of all six; 1-6 = that component alone", 0., 0., 6., 1.)],
        _ if reviewed(id) => vec![],
        _ => return None,
    })
}

/// Exponential average seeded with the first value (TradeStation XAverage).
fn xavg(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut out = vec![f64::NAN; v.len()];
    for i in 0..v.len() { out[i] = if i == 0 { v[0] } else { out[i - 1] + a * (v[i] - out[i - 1]) }; }
    out
}
/// Population standard deviation of the last n values (NaN before n).
fn psd(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else {
        let w = &v[i + 1 - n..=i]; let m = w.iter().sum::<f64>() / n as f64;
        (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n as f64).sqrt()
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = x.r.len();
    let gp = |k: &str| g(p, k, params(id));
    let tick = x.market_tick();
    Some(match id {
        "fot08_keyrev" => {
            let mut pos = Book::new();
            let (mut tg, mut st) = (f64::NAN, f64::NAN);
            for t in from..=to {
                let d = pos.side();
                if t > from && d != 0. {
                    let (ft, fs) = (super::ctm::reach(x, t, &ext(-d, K::Lim, tg)), super::ctm::reach(x, t, &ext(-d, K::Stop, st)));
                    match (ft, fs) {
                        (Some(u), s) if s.map_or(true, |s| u.0 < s.0) => pos.exit(t, u.1, "target 2 x range"),
                        (_, Some(s)) => pos.exit(t, s.1, "stop 1.5 x range"),
                        _ => {}
                    }
                }
                if pos.side() == 0. && t < to && t >= 1 {
                    let r = h[t] - l[t];
                    if l[t] < l[t - 1] && c[t] > h[t - 1] { pos.enter(1., t, c[t], "close"); tg = c[t] + 2. * r; st = c[t] - 1.5 * r; }
                    else if h[t] > h[t - 1] && c[t] < l[t - 1] { pos.enter(-1., t, c[t], "close"); tg = c[t] - 2. * r; st = c[t] + 1.5 * r; }
                }
            }
            pos.done(to, c[to])
        }
        "fot08_volbo" => {
            let (e7, s6, s100, a) = (xavg(c, 7), psd(c, 6), psd(c, 100), atr(x.r, 14));
            let (k, rf) = (gp("k"), gp("ref").round() == 1.);
            let mut sa = f64::NAN;
            orders(x, from, to, |t, pos| {
                let mut sd = pos.side();
                if (sd > 0. && c[t] < e7[t]) || (sd < 0. && c[t] > e7[t]) { pos.exit(t, c[t], "close beyond the 7-day EMA"); sd = 0.; }
                if sd == 0. {
                    if fin(s100[t]) && fin(a[t]) && s6[t] < 0.5 * s100[t] {
                        if t + 1 >= n { return vec![]; }
                        let base = if rf { o[t + 1] } else { c[t] };
                        sa = a[t];
                        return vec![ent(1., K::Stop, base + k * a[t]), ent(-1., K::Stop, base - k * a[t])];
                    }
                    return vec![];
                }
                vec![ext(-sd, K::Stop, pos.entry_px() - sd * 0.5 * sa)]
            })
        }
        "fot08_pretrend" => {
            let (walk, lonly) = (gp("walk").round().max(1.) as usize, gp("long").round() == 1.);
            orders(x, from, to, |t, pos| {
                let sd = pos.side();
                let mut z = vec![];
                if sd != 0. && t - pos.entry_bar().unwrap() >= walk { z.push(ext(-sd, K::Mkt, f64::NAN)); }
                if t < 2 { return z; }
                let ar = ((h[t - 2] - l[t - 2]) + (h[t - 1] - l[t - 1]) + (h[t] - l[t])) / 3.;
                let r = h[t] - l[t];
                if c[t] < c[t - 2] && r < ar && c[t] < o[t] && sd != 1. { z.push(ent(1., K::Stop, c[t])); }
                if !lonly && c[t] > c[t - 2] && r < ar && c[t] > o[t] && sd != -1. { z.push(ent(-1., K::Stop, c[t])); }
                z
            })
        }
        "fot08_3bar" => {
            let e3 = xavg(&xavg(&xavg(c, 9), 9), 9);
            let tx: Vec<f64> = (0..n).map(|i| if i == 0 { 0. } else { 100. * (e3[i] / e3[i - 1] - 1.) }).collect();
            let sg = xavg(&tx, 3);
            let up: Vec<bool> = (0..n).map(|i| tx[i] > sg[i]).collect();
            let (mut bt, mut st, mut bp, mut sp) = (false, false, f64::NAN, f64::NAN);
            orders(x, from, to, |t, pos| {
                if t >= 5 {
                    if up[t - 3] && (t - 2..=t).all(|j| c[j] < o[j]) { bt = true; bp = h[t]; st = false; }
                    if !up[t - 3] && (t - 2..=t).all(|j| c[j] > o[j]) { st = true; sp = l[t]; bt = false; }
                }
                let mut z = vec![];
                if bt { z.push(ent(1., K::Stop, bp)); }
                if st { z.push(ent(-1., K::Stop, sp)); }
                let mp = pos.side();
                let w0 = t.saturating_sub(59);
                if mp == 0. { bt = false; st = false; }
                if mp > 0. { z.push(ext(-1., K::Lim, hi(h, w0, t))); z.push(ext(-1., K::Stop, lo(l, w0, t))); bt = false; }
                if mp < 0. { z.push(ext(1., K::Lim, lo(l, w0, t))); z.push(ext(1., K::Stop, hi(h, w0, t))); st = false; }
                z
            })
        }
        "fot08_gapfade" => {
            let m = gp("n").round().max(1.) as usize;
            let mut pos = Book::new();
            for t in from.max(m)..=to {
                let d = if o[t] < lo(l, t - m, t - 1) { 1. } else if o[t] > hi(h, t - m, t - 1) { -1. } else { continue };
                pos.enter(d, t, o[t], "open");
                pos.exit(t, c[t], "same-day close");
            }
            pos.out
        }
        "fot08_nerves" => {
            let rbox = gp("mom") > 0.5;
            let mut pos = Book::new();
            let mut pend = false;
            for t in from..=to {
                if pend { pos.enter(1., t, o[t], "open"); pend = false; }
                if pos.side() != 0. {
                    let (hh, ll) = (hi(h, t.saturating_sub(6), t), lo(l, t.saturating_sub(6), t));
                    let m = if t >= 6 && hh > ll && (rbox || (t >= 7 && c[t] > c[t - 7])) { (c[t] - ll) / (hh - ll) } else { f64::NAN };
                    if m >= 0.75 || t - pos.entry_bar().unwrap() >= 10 { pos.exit(t, c[t], if m >= 0.75 { "7-day momentum >= 0.75 · close" } else { "10 days · close" }); }
                }
                if pos.side() == 0. && t < to && t >= 3 && l[t] < l[t - 1] && l[t - 1] < l[t - 2] && h[t] < h[t - 1] && h[t - 1] < h[t - 2]
                    && c[t] < c[t - 1] && c[t - 1] < c[t - 2] && c[t - 2] < c[t - 3] && l[t] <= 0.98 * l[t - 2] { pend = true; }
            }
            pos.done(to, c[to])
        }
        "fot08_bbtrend" => {
            let (m, s) = (sma(c, 20), psd(c, 20));
            let mut pos = Book::new();
            let mut ords = vec![];
            for t in from..=to {
                if t > from && !ords.is_empty() { super::ctm::bar(&mut pos, x, t, &ords); }
                ords = vec![];
                if pos.side() == 0. && t < to && fin(m[t]) {
                    if c[t] > m[t] + 2. * s[t] { pos.enter(1., t, c[t], "close"); } else if c[t] < m[t] - 2. * s[t] { pos.enter(-1., t, c[t], "close"); }
                }
                if t == to { break; }
                if pos.side() != 0. { ords = vec![ext(-pos.side(), K::Stop, m[t])]; }
            }
            pos.done(to, c[to])
        }
        "fot08_rsi9" => {
            let (r9, m200) = (rsi(c, 9), sma(c, 200));
            let v = |i: usize| x.r[i].volume;
            orders(x, from, to, |t, pos| {
                let sd = pos.side();
                if sd > 0. { return if r9[t] > 65. { vec![ext(-1., K::Mkt, f64::NAN)] } else { vec![] }; }
                if sd < 0. { return if r9[t] < 35. { vec![ext(1., K::Mkt, f64::NAN)] } else { vec![] }; }
                let quiet = t >= 1 && matches!((v(t), v(t - 1)), (Some(a), Some(b)) if a < b);
                if !quiet || !fin(m200[t]) || !fin(r9[t]) { return vec![]; }
                if r9[t] < 35. && c[t] > m200[t] { vec![ent(1., K::Mkt, f64::NAN)] } else if r9[t] > 65. && c[t] < m200[t] { vec![ent(-1., K::Mkt, f64::NAN)] } else { vec![] }
            })
        }
        "fot08_tnote" => {
            let (hold, shold, both) = (gp("hold").round().max(1.) as usize, gp("shold").round().max(1.) as usize, gp("side") < 0.5);
            let mut pos = Book::new();
            for t in from..=to {
                if pos.side() != 0. && t - pos.entry_bar().unwrap() >= if pos.side() > 0. { hold } else { shold } { pos.exit(t, c[t], "time exit · close"); }
                if t >= to || t < 40 { continue; }
                // Book::enter ignores a signal for the side already held and reverses on the other side's signal.
                if c[t] > c[t - 2] && c[t] > c[t - 40] && c[t - 1] < c[t - 2] && c[t - 2] < c[t - 3] { pos.enter(1., t, c[t], "close"); }
                else if both && c[t] > c[t - 1] && c[t] < c[t - 40] && c[t - 1] > c[t - 2] && h[t] > h[t - 1] && h[t - 1] > h[t - 2] { pos.enter(-1., t, c[t], "close"); }
            }
            pos.done(to, c[to])
        }
        "fot08_six" => {
            let rg: Vec<f64> = (0..n).map(|i| h[i] - l[i]).collect();
            let (s2, s5, s40, r10, h15, l15) = (sma(c, 2), sma(c, 5), sma(c, 40), sma(&rg, 10), sma(h, 15), sma(l, 15));
            let comp = gp("comp").round() as usize;
            let mut e = vec![0f64; n];
            let mut v = [0f64; 6];
            let mut tot = vec![0f64; n];
            for i in 0..n {
                e[i] = if c[i] > o[i] { 1. } else if c[i] < o[i] { -1. } else if i > 0 { e[i - 1] } else { 0. };
                if i < 49 { continue; }
                if s2[i] < s5[i] { v[0] = 1.; } if s2[i] > s5[i] { v[0] = -1.; }
                if c[i] > s40[i] { v[1] = 1.; } if c[i] < s40[i] { v[1] = -1.; }
                let (mx, mn) = (hi(c, i - 49, i), lo(c, i - 49, i));
                let hb = (0..50).find(|&k| c[i - k] == mx).unwrap_or(0);
                let lb = (0..50).find(|&k| c[i - k] == mn).unwrap_or(0);
                if hb > lb { v[2] = 1.; } if hb < lb { v[2] = -1.; }
                if (rg[i] < r10[i] && c[i] > c[i - 1]) || (rg[i] > r10[i] && c[i] < c[i - 1]) { v[3] = 1.; }
                if (rg[i] < r10[i] && c[i] < c[i - 1]) || (rg[i] > r10[i] && c[i] > c[i - 1]) { v[3] = -1.; }
                let mid = (h15[i] + l15[i]) / 2.;
                if c[i] > mid { v[4] = 1.; } if c[i] < mid { v[4] = -1.; }
                let ae = (e[i] + e[i - 1] + e[i - 2]) / 3.;
                if ae < 0. { v[5] = 1.; } if ae > 0. { v[5] = -1.; }
                tot[i] = if comp == 0 { v.iter().sum() } else { v[comp.min(6) - 1] };
            }
            let mut pos = Book::new();
            for t in from..to {
                if tot[t] == 0. { continue; }
                pos.enter(tot[t].signum(), t + 1, o[t + 1], "open");
                pos.exit(t + 1, c[t + 1], "exit on close");
            }
            pos.out
        }
        "fot08_triple" => {
            let (s200, s100, s15, ax) = (sma(c, 200), sma(c, 100), sma(c, 15), dmi(x.r, 14).1);
            let mut basis = f64::NAN;
            orders(x, from, to, |t, pos| {
                if t < 10 || !(fin(s200[t - 10]) && fin(ax[t - 10])) { return vec![]; }
                let sd = pos.side();
                if sd == 0. {
                    if s200[t] > s200[t - 10] && l[t] < s15[t] && s100[t] > s100[t - 10] && ax[t] > ax[t - 10] { basis = c[t]; return vec![ent(1., K::Stop, h[t] + tick)]; }
                    if s200[t] < s200[t - 10] && h[t] > s15[t] && s100[t] < s100[t - 10] && ax[t] > ax[t - 10] { basis = c[t]; return vec![ent(-1., K::Stop, l[t] - tick)]; }
                    return vec![];
                }
                let mut z = vec![];
                if (sd > 0. && s100[t] < s100[t - 10]) || (sd < 0. && s100[t] > s100[t - 10]) { z.push(ext(-sd, K::Mkt, f64::NAN)); }
                z.push(ext(-sd, K::Stop, basis * (1. - sd * 0.05)));
                z.push(ext(-sd, K::Lim, basis * (1. + sd * 0.05)));
                z
            })
        }
        "fot08_keltner_w" => {
            // weekly bars: (last daily bar, high, low, close)
            let mut wk: Vec<(usize, f64, f64, f64)> = vec![];
            let mut s0 = 0;
            for i in 0..n {
                if i == n - 1 || week_no(&x.r[i + 1].date) != week_no(&x.r[i].date) { wk.push((i, hi(h, s0, i), lo(l, s0, i), c[i])); s0 = i + 1; }
            }
            let tp: Vec<f64> = wk.iter().map(|w| (w.1 + w.2 + w.3) / 3.).collect();
            let rg: Vec<f64> = wk.iter().map(|w| w.1 - w.2).collect();
            let (mm, bb) = (sma(&tp, 10), sma(&rg, 10));
            let mut pos = Book::new();
            for (j, w) in wk.iter().enumerate() {
                let e = w.0;
                if e < from || e >= to || e == n - 1 || !fin(mm[j]) { continue; }
                if pos.side() != 1. && w.3 > mm[j] + bb[j] { pos.enter(1., e + 1, o[e + 1], "next week's open"); }
                else if pos.side() != -1. && w.3 < mm[j] - bb[j] { pos.enter(-1., e + 1, o[e + 1], "next week's open"); }
            }
            pos.done(to, c[to])
        }
        "fot08_hlr" => orders(x, from, to, |t, pos| {
            if t < 99 { return vec![]; }
            let (hh, ll) = (hi(h, t - 99, t), lo(l, t - 99, t));
            let r = if hh > ll { (c[t] - ll) / (hh - ll) } else { f64::NAN };
            let sd = pos.side();
            let mut z = vec![];
            if sd > 0. && r < 0.2 && c[t] < pos.entry_px() { z.push(ext(-1., K::Mkt, f64::NAN)); }
            if sd < 0. && r > 0.8 && c[t] > pos.entry_px() { z.push(ext(1., K::Mkt, f64::NAN)); }
            if sd != 1. { z.push(ent(1., K::Stop, hh)); }
            if sd != -1. { z.push(ent(-1., K::Stop, ll)); }
            z
        }),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fot08_systems_trade() {
        let r = super::super::ctm::tests::walk(4000);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(!t.is_empty(), "{id}");
            for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i, "{id}"); }
            assert!(family_of(id).is_some() && params(id).is_some());
        }
        // one-day trades
        for id in ["fot08_gapfade", "fot08_six"] { assert!(run(id, &x, 0, r.len() - 1, &Params::new()).unwrap().iter().all(|z| z.exit_i == z.legs[0].i)); }
        // T-note: both sides by default (short held 2 days unless reversed), long pattern only with side = 1
        let tn = run("fot08_tnote", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(tn.iter().any(|z| z.dir < 0.) && tn.iter().filter(|z| z.dir < 0. && !z.open).all(|z| z.exit_i - z.legs[0].i <= 2));
        let mut p = Params::new(); p.insert("side".into(), 1.);
        assert!(run("fot08_tnote", &x, 0, r.len() - 1, &p).unwrap().iter().all(|z| z.dir > 0.));
        // nerves: the text's momentum exit also needs the close above the close 7 days ago
        let nv = run("fot08_nerves", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(nv.iter().filter(|z| z.reason.starts_with("7-day")).all(|z| x.c[z.exit_i] > x.c[z.exit_i - 7]));
        assert_eq!(xavg(&[2., 4.], 3), vec![2., 3.]);
        assert!(run("fot08_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Futures & Options Trader 2008-10 2026-11-02 (titles lane K15).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "fot08" && reviewed(def.id),
    label: "Futures & Options Trader systems tested on futures (and the ETFs or stocks named on the card). Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Futures & Options Trader magazine, Apr 2008 - Mar 2010, article pages cited per card; independent check authors/titles-1030/K15w/k15_ref.py",
};
