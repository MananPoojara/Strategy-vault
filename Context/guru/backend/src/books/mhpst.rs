//! Michael Harris, Profitability and Systematic Trading (Wiley, 2008), Appendix 'Codes for Selected Price Patterns' (p.137-178):
//! APS-generated QQQQ, SPY, DAX and FTSE patterns (EasyLanguage, Metastock, Wealth-Lab). Entry next open, % target/stop.
//! Source pages, oracle and results: library/mhpst/.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mhpst";
// Admitted 2026-10-24 (authors/harris/admission/SOURCE-CONTRACT.md): complete pattern rules with independent US/India ledgers.
pub const REVIEWED: &[&str] = &["mhp_qqqq_1_20050218", "mhp_qqqq_11_20041012", "mhp_qqqq_7_20041101", "mhp_qqqq_7_20041001", "mhp_qqqq_5_20050207", "mhp_qqqq_6_20050120", "mhp_qqqq_2_20050218", "mhp_qqqq_4_20040430", "mhp_qqqq_12_20041111", "mhp_qqqq_4_20041203", "mhp_spy_2_20070807", "mhp_spy_2_20070703", "mhp_spy_10_20061101", "mhp_spy_8_20070511", "mhp_spy_9_20070808", "mhp_spy_2_20070724", "mhp_spy_2_20070327", "mhp_spy_1_20070703", "mhp_spy_1_20070621", "mhp_spy_1_20070424", "mhp_dax_11_20050121", "mhp_dax_10_20050331", "mhp_dax_6_20050223", "mhp_dax_4_20050202", "mhp_ftse_3_20040712", "mhp_ftse_10_20040730", "mhp_ftse_5_20040922", "mhp_ftse_5_20041019", "mhp_ftse_12_20040917", "mhp_ftse_3_20041007", "mhp_ftse_12_20040930", "mhp_ftse_12_20040728"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("mhp_qqqq_1_20050218", "QQQQ pattern 1/20050218 · long · 7%/7% · next open", "Appendix · QQQQ patterns", "Long", Status::Live,
            &["F|APS header {File:QQQQ.txt Index:1 Index Date:20050218 PL:67.57% PS:32.43% Trades:37 CL:3} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.140", "E|EasyLanguage: \"if h[1] > o[1] AND o[1] > h[0] AND h[0] > o[0] AND o[0] > c[1] AND c[1] > l[1] AND l[1] > c[0] AND c[0] > l[0] then begin Buy Next Bar at open\" p.140", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.140"],
            "Book market: QQQQ (Nasdaq-100 tracking stock); run on NDX / QQQ; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("37","67.57%","n/a","max 3 consecutive losers","APS QQQQ.txt, period not printed")),
        d("mhp_qqqq_11_20041012", "QQQQ pattern 11/20041012 · long · 7%/7% · next open", "Appendix · QQQQ patterns", "Long", Status::Live,
            &["F|APS header {File:QQQQ.txt Index:11 Index Date:20041012 PL:71.43% PS:28.57% Trades:35 CL:3} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.140", "E|EasyLanguage: \"if l[3] > h[0] AND h[0] > l[1] AND l[1] > l[2] then begin Buy Next Bar at open\" p.140", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.140"],
            "Book market: QQQQ (Nasdaq-100 tracking stock); run on NDX / QQQ; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("35","71.43%","n/a","max 3 consecutive losers","APS QQQQ.txt, period not printed")),
        d("mhp_qqqq_7_20041101", "QQQQ pattern 7/20041101 · long · 7%/7% · next open", "Appendix · QQQQ patterns", "Long", Status::Live,
            &["F|APS header {File:QQQQ.txt Index:7 Index Date:20041101 PL:70.97% PS:29.03% Trades:31 CL:2} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.141", "E|EasyLanguage: \"if c[0] > c[2] AND c[2] > c[1] AND c[1] > c[3] AND c[3] > c[4] AND c[4] > c[5] then begin Buy Next Bar at open\" p.141", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.141"],
            "Book market: QQQQ (Nasdaq-100 tracking stock); run on NDX / QQQ; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("31","70.97%","n/a","max 2 consecutive losers","APS QQQQ.txt, period not printed")),
        d("mhp_qqqq_7_20041001", "QQQQ pattern 7/20041001 · long · 7%/7% · next open", "Appendix · QQQQ patterns", "Long", Status::Live,
            &["F|APS header {File:QQQQ.txt Index:7 Index Date:20041001 PL:70.59% PS:29.41% Trades:34 CL:2} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.141", "E|EasyLanguage: \"if c[0] > c[1] AND c[1] > c[2] AND c[2] > c[5] AND c[5] > c[3] AND c[3] > c[4] then begin Buy Next Bar at open\" p.141", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.141"],
            "Book market: QQQQ (Nasdaq-100 tracking stock); run on NDX / QQQ; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("34","70.59%","n/a","max 2 consecutive losers","APS QQQQ.txt, period not printed")),
        d("mhp_qqqq_5_20050207", "QQQQ pattern 5/20050207 · long · 7%/7% · next open", "Appendix · QQQQ patterns", "Long", Status::Live,
            &["F|APS header {File:QQQQ.txt Index:5 Index Date:20050207 PL:69.09% PS:30.91% Trades:55 CL:2} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.142", "E|EasyLanguage: \"if c[1] > c[0] AND c[0] > c[3] AND c[3] > c[2] then begin Buy Next Bar at open\" p.142", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.142"],
            "Book market: QQQQ (Nasdaq-100 tracking stock); run on NDX / QQQ; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("55","69.09%","n/a","max 2 consecutive losers","APS QQQQ.txt, period not printed")),
        d("mhp_qqqq_6_20050120", "QQQQ pattern 6/20050120 · long · 7%/7% · next open", "Appendix · QQQQ patterns", "Long", Status::Live,
            &["F|APS header {File:QQQQ.txt Index:6 Index Date:20050120 PL:68.57% PS:31.43% Trades:35 CL:3} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.142", "E|EasyLanguage: \"if c[2] > c[3] AND c[3] > c[1] AND c[1] > c[4] AND c[4] > c[0] then begin Buy Next Bar at open\" p.142", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.142"],
            "Book market: QQQQ (Nasdaq-100 tracking stock); run on NDX / QQQ; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("35","68.57%","n/a","max 3 consecutive losers","APS QQQQ.txt, period not printed")),
        d("mhp_qqqq_2_20050218", "QQQQ pattern 2/20050218 · long · 7%/7% · next open", "Appendix · QQQQ patterns", "Long", Status::Live,
            &["F|APS header {File:QQQQ.txt Index:2 Index Date:20050218 PL:68.57% PS:31.43% Trades:35 CL:3} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.143", "E|EasyLanguage: \"if h[2] > h[1] AND h[1] > c[2] AND c[2] > l[2] AND l[2] > h[0] AND h[0] > c[1] AND c[1] > l[1] AND l[1] > c[0] AND c[0] > l[0] then begin Buy Next Bar at open\" p.143", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.143"],
            "Book market: QQQQ (Nasdaq-100 tracking stock); run on NDX / QQQ; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("35","68.57%","n/a","max 3 consecutive losers","APS QQQQ.txt, period not printed")),
        d("mhp_qqqq_4_20040430", "QQQQ pattern 4/20040430 · long · 7%/7% · next open", "Appendix · QQQQ patterns", "Long", Status::Live,
            &["F|APS header {File:QQQQ.txt Index:4 Index Date:20040430 PL:68.52% PS:31.48% Trades:54 CL:3} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.143", "E|EasyLanguage: \"if h[3] > h[2] AND h[2] > l[3] AND l[3] > h[1] AND h[1] > l[2] AND l[2] > h[0] AND h[0] > l[1] AND l[1] > l[0] then begin Buy Next Bar at open\" p.143", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.143"],
            "Book market: QQQQ (Nasdaq-100 tracking stock); run on NDX / QQQ; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("54","68.52%","n/a","max 3 consecutive losers","APS QQQQ.txt, period not printed")),
        d("mhp_qqqq_12_20041111", "QQQQ pattern 12/20041111 · long · 7%/7% · next open", "Appendix · QQQQ patterns", "Long", Status::Live,
            &["F|APS header {File:QQQQ.txt Index:12 Index Date:20041111 PL:68.09% PS:31.91% Trades:47 CL:3} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.144", "E|EasyLanguage: \"if h[0] > l[3] AND l[3] > l[2] AND l[2] > l[1] AND l[1] > l[4] then begin Buy Next Bar at open\" p.144", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.144"],
            "Book market: QQQQ (Nasdaq-100 tracking stock); run on NDX / QQQ; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("47","68.09%","n/a","max 3 consecutive losers","APS QQQQ.txt, period not printed")),
        d("mhp_qqqq_4_20041203", "QQQQ pattern 4/20041203 · long · 7%/7% · next open", "Appendix · QQQQ patterns", "Long", Status::Live,
            &["F|APS header {File:QQQQ.txt Index:4 Index Date:20041203 PL:67.27% PS:32.73% Trades:55 CL:3} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.144", "E|EasyLanguage: \"if h[0] > h[1] AND h[1] > l[0] AND l[0] > h[2] AND h[2] > l[1] AND l[1] > h[3] AND h[3] > l[2] AND l[2] > l[3]  then begin Buy Next Bar at open\" p.144", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.144"],
            "Book market: QQQQ (Nasdaq-100 tracking stock); run on NDX / QQQ; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("55","67.27%","n/a","max 3 consecutive losers","APS QQQQ.txt, period not printed")),
        d("mhp_spy_2_20070807", "SPY pattern 2/20070807 · long · 7%/7% · next open", "Appendix · SPY patterns", "Long", Status::Live,
            &["F|APS header {File:SPY.txt Index:2 Index Date:20070807 PL:74.19% PS:25.81% Trades:31 CL:2} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.153", "E|EasyLanguage: \"if h[0] > c[0] AND c[0] > h[2] AND h[2] > h[1] AND h[1] > l[0] AND l[0] > l[2] AND l[2] > l[1] then begin Buy Next Bar at open\" p.153", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.153"],
            "Book market: SPY; run on SPY / SPX; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("31","74.19%","n/a","max 2 consecutive losers","APS SPY.txt, period not printed")),
        d("mhp_spy_2_20070703", "SPY pattern 2/20070703 · long · 7%/7% · next open", "Appendix · SPY patterns", "Long", Status::Live,
            &["F|APS header {File:SPY.txt Index:2 Index Date:20070703 PL:67.74% PS:32.26% Trades:31 CL:2} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.153", "E|EasyLanguage: \"if h[0] > c[0] AND c[0] > l[0] AND l[0] > h[1] AND h[1] > h[2] AND h[2] > l[1] AND l[1] > l[2] then begin Buy Next Bar at open\" p.153", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.153"],
            "Book market: SPY; run on SPY / SPX; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("31","67.74%","n/a","max 2 consecutive losers","APS SPY.txt, period not printed")),
        d("mhp_spy_10_20061101", "SPY pattern 10/20061101 · long · 7%/7% · next open", "Appendix · SPY patterns", "Long", Status::Live,
            &["F|APS header {File:SPY.txt Index:10 Index Date:20061101 PL:77.42% PS:22.58% Trades:31 CL:2} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.154", "E|EasyLanguage: \"if c[4] > c[5] AND c[5] > c[3] AND c[3] > c[2] AND c[2] > c[1] AND c[1] > c[0] then begin Buy Next Bar at open\" p.154", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.154"],
            "Book market: SPY; run on SPY / SPX; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("31","77.42%","n/a","max 2 consecutive losers","APS SPY.txt, period not printed")),
        d("mhp_spy_8_20070511", "SPY pattern 8/20070511 · long · 7%/7% · next open", "Appendix · SPY patterns", "Long", Status::Live,
            &["F|APS header {File:SPY.txt Index:8 Index Date:20070511 PL:67.74% PS:32.26% Trades:31 CL:3} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.154", "E|EasyLanguage: \"if c[2] > c[0] AND c[0] > c[3] AND c[3] > c[1] then begin Buy Next Bar at open\" p.154", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.154"],
            "Book market: SPY; run on SPY / SPX; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("31","67.74%","n/a","max 3 consecutive losers","APS SPY.txt, period not printed")),
        d("mhp_spy_9_20070808", "SPY pattern 9/20070808 · long · 7%/7% · next open", "Appendix · SPY patterns", "Long", Status::Live,
            &["F|APS header {File:SPY.txt Index:9 Index Date:20070808 PL:78.79% PS:21.21% Trades:33 CL:2} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.155", "E|EasyLanguage: \"if c[0] > c[1] AND c[1] > c[4] AND c[4] > c[2] AND c[2] > c[3] then begin Buy Next Bar at open\" p.155", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.155"],
            "Book market: SPY; run on SPY / SPX; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("33","78.79%","n/a","max 2 consecutive losers","APS SPY.txt, period not printed")),
        d("mhp_spy_2_20070724", "SPY pattern 2/20070724 · long · 7%/7% · next open", "Appendix · SPY patterns", "Long", Status::Live,
            &["F|APS header {File:SPY.txt Index:2 Index Date:20070724 PL:78.79% PS:21.21% Trades:33 CL:3} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.155", "E|EasyLanguage: \"if h[2] > h[1] AND h[1] > h[0] AND h[0] > l[1] AND l[1] > l[2] AND l[2] > c[0] AND c[0] > l[0] then begin Buy Next Bar at open\" p.155", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.155"],
            "Book market: SPY; run on SPY / SPX; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("33","78.79%","n/a","max 3 consecutive losers","APS SPY.txt, period not printed")),
        d("mhp_spy_2_20070327", "SPY pattern 2/20070327 · long · 7%/7% · next open", "Appendix · SPY patterns", "Long", Status::Live,
            &["F|APS header {File:SPY.txt Index:2 Index Date:20070327 PL:67.65% PS:32.35% Trades:34 CL:3} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.156", "E|EasyLanguage: \"if h[2] > h[1] AND h[1] > h[0] AND h[0] > l[2] AND l[2] > c[0] AND c[0] > l[0] AND l[0] > l[1] then begin Buy Next Bar at open\" p.156", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.156"],
            "Book market: SPY; run on SPY / SPX; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("34","67.65%","n/a","max 3 consecutive losers","APS SPY.txt, period not printed")),
        d("mhp_spy_1_20070703", "SPY pattern 1/20070703 · long · 7%/7% · next open", "Appendix · SPY patterns", "Long", Status::Live,
            &["F|APS header {File:SPY.txt Index:1 Index Date:20070703 PL:70.59% PS:29.41% Trades:34 CL:2} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.156", "E|EasyLanguage: \"if h[0] > l[0] AND l[0] > h[1] AND h[1] > h[2] AND h[2] > l[1] AND l[1] > l[2] then begin Buy Next Bar at open\" p.156", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.156"],
            "Book market: SPY; run on SPY / SPX; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("34","70.59%","n/a","max 2 consecutive losers","APS SPY.txt, period not printed")),
        d("mhp_spy_1_20070621", "SPY pattern 1/20070621 · long · 7%/7% · next open", "Appendix · SPY patterns", "Long", Status::Live,
            &["F|APS header {File:SPY.txt Index:1 Index Date:20070621 PL:77.78% PS:22.22% Trades:36 CL:3} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.157", "E|EasyLanguage: \"if h[1] > h[2] AND h[2] > l[2] AND l[2] > h[0] AND h[0] > l[1] AND l[1] > l[0] then begin Buy Next Bar at open\" p.157", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.157"],
            "Book market: SPY; run on SPY / SPX; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("36","77.78%","n/a","max 3 consecutive losers","APS SPY.txt, period not printed")),
        d("mhp_spy_1_20070424", "SPY pattern 1/20070424 · long · 7%/7% · next open", "Appendix · SPY patterns", "Long", Status::Live,
            &["F|APS header {File:SPY.txt Index:1 Index Date:20070424 PL:83.33% PS:16.67% Trades:36 CL:2} {LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 0} PST p.157", "E|EasyLanguage: \"if h[1] > h[2] AND h[2] > h[0] AND h[0] > l[1] AND l[1] > l[0] AND l[0] > l[2] then begin Buy Next Bar at open\" p.157", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.157"],
            "Book market: SPY; run on SPY / SPX; NIFTY 50 for India. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("36","83.33%","n/a","max 2 consecutive losers","APS SPY.txt, period not printed")),
        d("mhp_dax_11_20050121", "DAX pattern 11/20050121 · long · 3%/3% · next open", "Appendix · DAX patterns", "Long", Status::Live,
            &["F|APS header {File:DAX.txt Index:11 Index Date:20050121 PL:68.42% PS:31.58% Trades:38 CL:3} {LONG, %, TARGET: 3, STOP: 3, ENTRY PRICE: OPEN, DELAY: 0} PST p.163", "E|EasyLanguage: \"if l[2] > h[0] AND h[0] > l[3] AND l[3] > l[1] then begin Buy Next Bar at open\" p.163", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.163"],
            "Book market: DAX index futures; run on an equity index (instrument swap; no DAX data in the vault): SPX / NIFTY 50. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("38","68.42%","n/a","max 3 consecutive losers","APS DAX.txt, period not printed")),
        d("mhp_dax_10_20050331", "DAX pattern 10/20050331 · short · 3%/3% · next open", "Appendix · DAX patterns", "Short", Status::Live,
            &["F|APS header {File:DAX.txt Index:10 Index Date:20050331 PL:32.26% PS:67.74% Trades:31 CL:2} {SHORT, %, TARGET: 3, STOP: 3, ENTRY PRICE: OPEN, DELAY: 0} PST p.164", "E|EasyLanguage: \"if h[0] > h[1] AND h[1] > h[2] AND h[2] > c[0] AND c[0] > l[0] AND l[0] > l[1] AND l[1] > l[2] then begin Sell Short Next Bar at open\" p.164", "X|\"profitprice = O of tomorrow * (1-ptarget/100); stopprice = O of tomorrow * (1+stopl/100); buy to cover next bar at profitprice limit; buy to cover next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.164"],
            "Book market: DAX index futures; run on an equity index (instrument swap; no DAX data in the vault): SPX / NIFTY 50. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("31","67.74%","n/a","max 2 consecutive losers","APS DAX.txt, period not printed")),
        d("mhp_dax_6_20050223", "DAX pattern 6/20050223 · short · 2%/4% · next open", "Appendix · DAX patterns", "Short", Status::Live,
            &["F|APS header {File:DAX.txt Index:6 Index Date:20050223 PL:15.63% PS:84.38% Trades:32 CL:1} {SHORT, %, TARGET: 2, STOP: 4, ENTRY PRICE: OPEN, DELAY: 0} PST p.164", "E|EasyLanguage: \"if c[4] > c[2] AND c[2] > c[3] AND c[3] > c[1] AND c[1] > c[0] then begin Sell Short Next Bar at open\" p.164", "X|\"profitprice = O of tomorrow * (1-ptarget/100); stopprice = O of tomorrow * (1+stopl/100); buy to cover next bar at profitprice limit; buy to cover next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.164"],
            "Book market: DAX index futures; run on an equity index (instrument swap; no DAX data in the vault): SPX / NIFTY 50. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("32","84.38%","n/a","max 1 consecutive losers","APS DAX.txt, period not printed")),
        d("mhp_dax_4_20050202", "DAX pattern 4/20050202 · long · 2%/4% · next open", "Appendix · DAX patterns", "Long", Status::Live,
            &["F|APS header {File:DAX.txt Index:4 Index Date:20050202 PL:80.00% PS:20.00% Trades:30 CL:1} {LONG, %, TARGET: 2, STOP: 4, ENTRY PRICE: OPEN, DELAY: 0} PST p.165", "E|EasyLanguage: \"if h[0] > h[1] AND h[1] > l[0] AND l[0] > h[2] AND h[2] > l[1] AND l[1] > h[3] AND h[3] > l[2] AND l[2] > l[3] then begin Buy Next Bar at open\" p.165", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.165"],
            "Book market: DAX index futures; run on an equity index (instrument swap; no DAX data in the vault): SPX / NIFTY 50. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("30","80.00%","n/a","max 1 consecutive losers","APS DAX.txt, period not printed")),
        d("mhp_ftse_3_20040712", "FTSE pattern 3/20040712 · short · 2%/4% · next open", "Appendix · FTSE patterns", "Short", Status::Live,
            &["F|APS header {File:FTSE.txt Index:3 Index Date:20040712 PL:15.71% PS:84.29% Trades:70 CL:2} {SHORT, %, TARGET: 2, STOP: 4, ENTRY PRICE: OPEN, DELAY: 0} PST p.169", "E|EasyLanguage: \"if h[1] > h[0] AND h[0] > h[2] AND h[2] > l[1] AND l[1] > l[0] AND l[0] > l[2] then begin Sell Short Next Bar at open\" p.169", "X|\"profitprice = O of tomorrow * (1-ptarget/100); stopprice = O of tomorrow * (1+stopl/100); buy to cover next bar at profitprice limit; buy to cover next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.169"],
            "Book market: FTSE 100 index futures; run on an equity index (instrument swap; no FTSE data in the vault): SPX / NIFTY 50. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("70","84.29%","n/a","max 2 consecutive losers","APS FTSE.txt, period not printed")),
        d("mhp_ftse_12_20041015", "FTSE pattern 12/20041015 · long · 2%/4% · next open", "Appendix · FTSE patterns", "Long", Status::Live,
            &["F|APS header {File:FTSE.txt Index:12 Index Date:20041015 PL:81.97% PS:18.03% Trades:61 CL:2} {LONG, %, TARGET: 2, STOP: 4, ENTRY PRICE: OPEN, DELAY: 0} PST p.169", "E|EasyLanguage: \"if l[4] > h[0] AND h[0] > l[3] AND l[3] > l[2] AND l[2] > l[1] then begin Buy Next Bar at open\" p.169", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.169", "X|[intraday wave 2b] Default p.path=1: on each day the trade is held, real 1-minute bars decide whether the target or the stop came first (fill at the level, or the minute's open when it opened through it; both in one minute: the level nearer the minute's open [convention]). Days without minute data, and NIFTY 50 days from 2026-08-11 (bad closing minutes), keep the daily reading, which is exact unless both levels are inside that day's bar (then the stop is assumed first, as before). Runs on SPY, SPX (from 2014, real daily opens), NIFTY 50, NIFTYBANK within their minute periods. p.path=0 = the old daily reading [Not from the book]."],
            "Book market: FTSE 100 index futures; run on an equity index (instrument swap; no FTSE data in the vault): SPX / NIFTY 50. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately. Same pattern as Fig 7.3-7.6 (Example 1: 82% profitable, profit factor 2.2, RWL 0.48 in Metastock, p.116-118).", claim("61","81.97%","n/a","max 2 consecutive losers","APS FTSE.txt, period not printed")),
        d("mhp_ftse_10_20040730", "FTSE pattern 10/20040730 · short · 2%/4% · next open", "Appendix · FTSE patterns", "Short", Status::Live,
            &["F|APS header {File:FTSE.txt Index:10 Index Date:20040730 PL:19.64% PS:80.36% Trades:56 CL:1} {SHORT, %, TARGET: 2, STOP: 4, ENTRY PRICE: OPEN, DELAY: 1} PST p.170", "E|EasyLanguage: \"if h[1] > h[2] AND h[2] > c[1] AND c[1] > l[1] AND l[1] > h[3] AND h[3] > l[2] AND l[2] > l[3] then begin Sell Short Next Bar at open\" p.170", "X|\"profitprice = O of tomorrow * (1-ptarget/100); stopprice = O of tomorrow * (1+stopl/100); buy to cover next bar at profitprice limit; buy to cover next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.170"],
            "Book market: FTSE 100 index futures; run on an equity index (instrument swap; no FTSE data in the vault): SPX / NIFTY 50. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("56","80.36%","n/a","max 1 consecutive losers","APS FTSE.txt, period not printed")),
        d("mhp_ftse_5_20040922", "FTSE pattern 5/20040922 · short · 2%/4% · next open", "Appendix · FTSE patterns", "Short", Status::Live,
            &["F|APS header {File:FTSE.txt Index:5 Index Date:20040922 PL:18.18% PS:81.82% Trades:55 CL:2} {SHORT, %, TARGET: 2, STOP: 4, ENTRY PRICE: OPEN, DELAY: 0} PST p.170", "E|EasyLanguage: \"if c[1] > c[0] AND c[0] > c[3] AND c[3] > c[2] then begin Sell Short Next Bar at open\" p.170", "X|\"profitprice = O of tomorrow * (1-ptarget/100); stopprice = O of tomorrow * (1+stopl/100); buy to cover next bar at profitprice limit; buy to cover next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.170"],
            "Book market: FTSE 100 index futures; run on an equity index (instrument swap; no FTSE data in the vault): SPX / NIFTY 50. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("55","81.82%","n/a","max 2 consecutive losers","APS FTSE.txt, period not printed")),
        d("mhp_ftse_5_20041019", "FTSE pattern 5/20041019 · short · 2%/4% · next open", "Appendix · FTSE patterns", "Short", Status::Live,
            &["F|APS header {File:FTSE.txt Index:5 Index Date:20041019 PL:15.38% PS:84.62% Trades:52 CL:1} {SHORT, %, TARGET: 2, STOP: 4, ENTRY PRICE: OPEN, DELAY: 2} PST p.171", "E|EasyLanguage: \"if c[2] > c[5] AND c[5] > c[4] AND c[4] > c[3] then begin Sell Short Next Bar at open\" p.171", "X|\"profitprice = O of tomorrow * (1-ptarget/100); stopprice = O of tomorrow * (1+stopl/100); buy to cover next bar at profitprice limit; buy to cover next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.171"],
            "Book market: FTSE 100 index futures; run on an equity index (instrument swap; no FTSE data in the vault): SPX / NIFTY 50. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("52","84.62%","n/a","max 1 consecutive losers","APS FTSE.txt, period not printed")),
        d("mhp_ftse_12_20040917", "FTSE pattern 12/20040917 · short · 2%/4% · next open", "Appendix · FTSE patterns", "Short", Status::Live,
            &["F|APS header {File:FTSE.txt Index:12 Index Date:20040917 PL:19.61% PS:80.39% Trades:51 CL:2} {SHORT, %, TARGET: 2, STOP: 4, ENTRY PRICE: OPEN, DELAY: 2} PST p.171", "E|EasyLanguage: \"if h[2] > l[6] AND l[6] > l[3] AND l[3] > l[4] AND l[4] > l[5] then begin Sell Short Next Bar at open\" p.171", "X|\"profitprice = O of tomorrow * (1-ptarget/100); stopprice = O of tomorrow * (1+stopl/100); buy to cover next bar at profitprice limit; buy to cover next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.171"],
            "Book market: FTSE 100 index futures; run on an equity index (instrument swap; no FTSE data in the vault): SPX / NIFTY 50. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("51","80.39%","n/a","max 2 consecutive losers","APS FTSE.txt, period not printed")),
        d("mhp_ftse_3_20041007", "FTSE pattern 3/20041007 · long · 2%/4% · next open", "Appendix · FTSE patterns", "Long", Status::Live,
            &["F|APS header {File:FTSE.txt Index:3 Index Date:20041007 PL:83.67% PS:16.33% Trades:49 CL:2} {LONG, %, TARGET: 2, STOP: 4, ENTRY PRICE: OPEN, DELAY: 3} PST p.172", "E|EasyLanguage: \"if h[3] > h[4] AND h[4] > h[5] AND h[5] > l[4] AND l[4] > l[3] AND l[3] > l[5] then begin Buy Next Bar at open\" p.172", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.172"],
            "Book market: FTSE 100 index futures; run on an equity index (instrument swap; no FTSE data in the vault): SPX / NIFTY 50. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("49","83.67%","n/a","max 2 consecutive losers","APS FTSE.txt, period not printed")),
        d("mhp_ftse_12_20040930", "FTSE pattern 12/20040930 · long · 2%/4% · next open", "Appendix · FTSE patterns", "Long", Status::Live,
            &["F|APS header {File:FTSE.txt Index:12 Index Date:20040930 PL:81.40% PS:18.60% Trades:43 CL:3} {LONG, %, TARGET: 2, STOP: 4, ENTRY PRICE: OPEN, DELAY: 3} PST p.172", "E|EasyLanguage: \"if h[3] > l[4] AND l[4] > l[7] AND l[7] > l[6] AND l[6] > l[5] then begin Buy Next Bar at open\" p.172", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.172"],
            "Book market: FTSE 100 index futures; run on an equity index (instrument swap; no FTSE data in the vault): SPX / NIFTY 50. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("43","81.40%","n/a","max 3 consecutive losers","APS FTSE.txt, period not printed")),
        d("mhp_ftse_12_20040728", "FTSE pattern 12/20040728 · short · 2%/4% · next open", "Appendix · FTSE patterns", "Short", Status::Live,
            &["F|APS header {File:FTSE.txt Index:12 Index Date:20040728 PL:16.28% PS:83.72% Trades:43 CL:1} {SHORT, %, TARGET: 2, STOP: 4, ENTRY PRICE: OPEN, DELAY: 2} PST p.172", "E|EasyLanguage: \"if h[2] > l[5] AND l[5] > l[6] AND l[6] > l[3] AND l[3] > l[4] then begin Sell Short Next Bar at open\" p.172", "X|\"profitprice = O of tomorrow * (1-ptarget/100); stopprice = O of tomorrow * (1+stopl/100); buy to cover next bar at profitprice limit; buy to cover next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.172"],
            "Book market: FTSE 100 index futures; run on an equity index (instrument swap; no FTSE data in the vault): SPX / NIFTY 50. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("43","83.72%","n/a","max 1 consecutive losers","APS FTSE.txt, period not printed")),
        d("mhp_ftse_3_20040804", "FTSE pattern 3/20040804 · long · 2%/4% · next open", "Appendix · FTSE patterns", "Long", Status::Live,
            &["F|APS header {File:FTSE.txt Index:3 Index Date:20040804 PL:80.95% PS:19.05% Trades:42 CL:2} {LONG, %, TARGET: 2, STOP: 4, ENTRY PRICE: OPEN, DELAY: 3} PST p.173", "E|EasyLanguage: \"if h[4] > h[3] AND h[3] > h[5] AND h[5] > l[4] AND l[4] > l[5] AND l[5] > l[3] then begin Buy Next Bar at open\" p.173", "X|\"profitprice = O of tomorrow * (1+ptarget/100); stopprice = O of tomorrow * (1-stopl/100); sell next bar at profitprice limit; sell next bar at stopprice stop\" (orders placed on the signal bar, so they work on the entry bar too) p.173", "X|[intraday wave 2b] Default p.path=1: on each day the trade is held, real 1-minute bars decide whether the target or the stop came first (fill at the level, or the minute's open when it opened through it; both in one minute: the level nearer the minute's open [convention]). Days without minute data, and NIFTY 50 days from 2026-08-11 (bad closing minutes), keep the daily reading, which is exact unless both levels are inside that day's bar (then the stop is assumed first, as before). Runs on SPY, SPX (from 2014, real daily opens), NIFTY 50, NIFTYBANK within their minute periods. p.path=0 = the old daily reading [Not from the book]."],
            "Book market: FTSE 100 index futures; run on an equity index (instrument swap; no FTSE data in the vault): SPX / NIFTY 50. EasyLanguage code wins over the Metastock header (Metastock DELAY counts the next-bar entry; EasyLanguage DELAY = Metastock DELAY - 1 with the bars already shifted). One position at a time (EasyLanguage 'if Marketposition = 0'; Wealth-Lab 'if not LastPositionActive'). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. The APS search period of this file is not printed (Index Date is APS's internal classification), so header stats are compared approximately.", claim("42","80.95%","n/a","max 2 consecutive losers","APS FTSE.txt, period not printed")),
        d("mhp_rsi_ma_example", "RSI(14) < 30 AND MA(5) > MA(30) (synthesis illustration)", "Ch.7 The process of synthesis", "Long", Status::NotTestable("Illustrative output the synthesis process 'may determine' (p.109); never tested or reported, and order timing, RSI type and sizing are not stated."),
            &["E|\"Long entry signal if: RSI(14) < 30 AND [MA(5) > MA(30)]\" PST p.109", "X|\"Long exit signal if: MA(5) < MA(30)\" p.109"],
            "Illustrative output the synthesis process 'may determine' (p.109); never tested or reported, and order timing, RSI type and sizing are not stated.", None),
        d("mhp_qqq_2002", "QQQ patterns found in 2002 (Fig 7.13, Table 7.6)", "Ch.7 Example 3", "Long", Status::NotTestable("The 8 patterns' logic is not printed (only APS index numbers and stats, Table 7.6, p.129)."),
            &["F|Search 07/11/1990-05/07/2002, open of next day, 7%/7%, min 66%, >=30 trades, CL < 4 (Table 7.5) p.128", "F|Forward test 05/07/2002-09/28/2007 (Table 7.6) p.129"],
            "The 8 patterns' logic is not printed (only APS index numbers and stats, Table 7.6, p.129).", None),
        d("mhp_qqqq_2007", "QQQQ patterns of Fig 7.14 (2007 search)", "Ch.7 Example 3", "Long", Status::NotTestable("Fig 7.14 lists 15 patterns (index dates 2006-2007) by APS index only; the appendix code is for a different, earlier selection (index dates 2004-2005), already coded as mhp_qqqq_*."),
            &["F|Search 07/11/1990-09/28/2007, 7%/7% (Fig 7.14) p.130"],
            "Fig 7.14 lists 15 patterns (index dates 2006-2007) by APS index only; the appendix code is for a different, earlier selection (index dates 2004-2005), already coded as mhp_qqqq_*.", None),
        d("mhp_msft_system", "MSFT 5-pattern trend-following system (Fig 7.15-7.18)", "Ch.7 Example 4", "Long", Status::NotTestable("The five MSFT patterns are not printed (Fig 7.15 lists APS index numbers only)."),
            &["F|\"Profitability > 66%, Number of trades > 30, Consecutive losers < 4, Trade input: open of next day, Profit target/stop-loss = 7% of entry price\" (MSFT Jan 1990-Feb 14 2003) p.131", "F|Combined system: 65% profitable, RWL 0.73, PF 1.34 p.132"],
            "The five MSFT patterns are not printed (Fig 7.15 lists APS index numbers only).", None),
        d("mhp_spy_trend_system", "SPY price-pattern system 1994-1998 (Fig 7.19-7.20)", "Ch.7 Example 4", "Long", Status::NotTestable("The patterns behind the 18 SPY trades are not identified."),
            &["F|Fig 7.20: 18 closed trades, 18 winners, +68.838 points, 1411 days (SPY 1994-1998) p.135"],
            "The patterns behind the 18 SPY trades are not identified.", None),
        d("mhp_mtf_intraday", "Intraday/short-term multi-timeframe method (INTC)", "Ch.3 Trading in multiple time frames", "Long", Status::NotTestable("Uses hourly-bar MA slope, a daily inside day and a hand-drawn 15-minute channel/trendline: discretionary and intraday, no exact parameters."),
            &["F|Hourly simple moving average slope as short-term trend; inside-day entry; exits by 15-minute rising-channel trendline (Fig 3.4-3.6) p.38-40"],
            "Uses hourly-bar MA slope, a daily inside day and a hand-drawn 15-minute channel/trendline: discretionary and intraday, no exact parameters.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "mhp_qqqq_1_20050218" => ("mhp_qqqq", "QQQQ price patterns (APS)", "Index 1 · 20050218"),
        "mhp_qqqq_11_20041012" => ("mhp_qqqq", "QQQQ price patterns (APS)", "Index 11 · 20041012"),
        "mhp_qqqq_7_20041101" => ("mhp_qqqq", "QQQQ price patterns (APS)", "Index 7 · 20041101"),
        "mhp_qqqq_7_20041001" => ("mhp_qqqq", "QQQQ price patterns (APS)", "Index 7 · 20041001"),
        "mhp_qqqq_5_20050207" => ("mhp_qqqq", "QQQQ price patterns (APS)", "Index 5 · 20050207"),
        "mhp_qqqq_6_20050120" => ("mhp_qqqq", "QQQQ price patterns (APS)", "Index 6 · 20050120"),
        "mhp_qqqq_2_20050218" => ("mhp_qqqq", "QQQQ price patterns (APS)", "Index 2 · 20050218"),
        "mhp_qqqq_4_20040430" => ("mhp_qqqq", "QQQQ price patterns (APS)", "Index 4 · 20040430"),
        "mhp_qqqq_12_20041111" => ("mhp_qqqq", "QQQQ price patterns (APS)", "Index 12 · 20041111"),
        "mhp_qqqq_4_20041203" => ("mhp_qqqq", "QQQQ price patterns (APS)", "Index 4 · 20041203"),
        "mhp_spy_2_20070807" => ("mhp_spy", "SPY price patterns (APS)", "Index 2 · 20070807"),
        "mhp_spy_2_20070703" => ("mhp_spy", "SPY price patterns (APS)", "Index 2 · 20070703"),
        "mhp_spy_10_20061101" => ("mhp_spy", "SPY price patterns (APS)", "Index 10 · 20061101"),
        "mhp_spy_8_20070511" => ("mhp_spy", "SPY price patterns (APS)", "Index 8 · 20070511"),
        "mhp_spy_9_20070808" => ("mhp_spy", "SPY price patterns (APS)", "Index 9 · 20070808"),
        "mhp_spy_2_20070724" => ("mhp_spy", "SPY price patterns (APS)", "Index 2 · 20070724"),
        "mhp_spy_2_20070327" => ("mhp_spy", "SPY price patterns (APS)", "Index 2 · 20070327"),
        "mhp_spy_1_20070703" => ("mhp_spy", "SPY price patterns (APS)", "Index 1 · 20070703"),
        "mhp_spy_1_20070621" => ("mhp_spy", "SPY price patterns (APS)", "Index 1 · 20070621"),
        "mhp_spy_1_20070424" => ("mhp_spy", "SPY price patterns (APS)", "Index 1 · 20070424"),
        "mhp_dax_11_20050121" => ("mhp_dax", "DAX price patterns (APS)", "Index 11 · 20050121"),
        "mhp_dax_10_20050331" => ("mhp_dax", "DAX price patterns (APS)", "Index 10 · 20050331"),
        "mhp_dax_6_20050223" => ("mhp_dax", "DAX price patterns (APS)", "Index 6 · 20050223"),
        "mhp_dax_4_20050202" => ("mhp_dax", "DAX price patterns (APS)", "Index 4 · 20050202"),
        "mhp_ftse_3_20040712" => ("mhp_ftse", "FTSE price patterns (APS)", "Index 3 · 20040712"),
        "mhp_ftse_12_20041015" => ("mhp_ftse", "FTSE price patterns (APS)", "Index 12 · 20041015"),
        "mhp_ftse_10_20040730" => ("mhp_ftse", "FTSE price patterns (APS)", "Index 10 · 20040730"),
        "mhp_ftse_5_20040922" => ("mhp_ftse", "FTSE price patterns (APS)", "Index 5 · 20040922"),
        "mhp_ftse_5_20041019" => ("mhp_ftse", "FTSE price patterns (APS)", "Index 5 · 20041019"),
        "mhp_ftse_12_20040917" => ("mhp_ftse", "FTSE price patterns (APS)", "Index 12 · 20040917"),
        "mhp_ftse_3_20041007" => ("mhp_ftse", "FTSE price patterns (APS)", "Index 3 · 20041007"),
        "mhp_ftse_12_20040930" => ("mhp_ftse", "FTSE price patterns (APS)", "Index 12 · 20040930"),
        "mhp_ftse_12_20040728" => ("mhp_ftse", "FTSE price patterns (APS)", "Index 12 · 20040728"),
        "mhp_ftse_3_20040804" => ("mhp_ftse", "FTSE price patterns (APS)", "Index 3 · 20040804"),
        "mhp_rsi_ma_example" => ("mhp_other", "Not testable", "RSI(14) < 30 AND MA(5) > MA(30) (synthesis illustration)"),
        "mhp_qqq_2002" => ("mhp_other", "Not testable", "QQQ patterns found in 2002 (Fig 7.13, Table 7.6)"),
        "mhp_qqqq_2007" => ("mhp_other", "Not testable", "QQQQ patterns of Fig 7.14 (2007 search)"),
        "mhp_msft_system" => ("mhp_other", "Not testable", "MSFT 5-pattern trend-following system (Fig 7.15-7.18)"),
        "mhp_spy_trend_system" => ("mhp_other", "Not testable", "SPY price-pattern system 1994-1998 (Fig 7.19-7.20)"),
        "mhp_mtf_intraday" => ("mhp_other", "Not testable", "Intraday/short-term multi-timeframe method (INTC)"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "mhp_qqqq_1_20050218" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_qqqq_11_20041012" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_qqqq_7_20041101" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_qqqq_7_20041001" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_qqqq_5_20050207" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_qqqq_6_20050120" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_qqqq_2_20050218" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_qqqq_4_20040430" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_qqqq_12_20041111" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_qqqq_4_20041203" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_spy_2_20070807" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_spy_2_20070703" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_spy_10_20061101" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_spy_8_20070511" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_spy_9_20070808" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_spy_2_20070724" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_spy_2_20070327" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_spy_1_20070703" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_spy_1_20070621" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_spy_1_20070424" => vec![p("target","Profit target (% of entry)",7.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,0.5,20.0,0.5)],
        "mhp_dax_11_20050121" => vec![p("target","Profit target (% of entry)",3.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",3.0,0.5,20.0,0.5)],
        "mhp_dax_10_20050331" => vec![p("target","Profit target (% of entry)",3.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",3.0,0.5,20.0,0.5)],
        "mhp_dax_6_20050223" => vec![p("target","Profit target (% of entry)",2.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",4.0,0.5,20.0,0.5)],
        "mhp_dax_4_20050202" => vec![p("target","Profit target (% of entry)",2.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",4.0,0.5,20.0,0.5)],
        "mhp_ftse_3_20040712" => vec![p("target","Profit target (% of entry)",2.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",4.0,0.5,20.0,0.5)],
        "mhp_ftse_12_20041015" => vec![p("target","Profit target (% of entry)",2.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",4.0,0.5,20.0,0.5),
            p("path","1 = real 1-minute bars decide stop vs target (default), 0 = [Not from the book] daily bar, stop assumed first",1.0,0.0,1.0,1.0)], // intraday wave 2b
        "mhp_ftse_10_20040730" => vec![p("target","Profit target (% of entry)",2.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",4.0,0.5,20.0,0.5)],
        "mhp_ftse_5_20040922" => vec![p("target","Profit target (% of entry)",2.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",4.0,0.5,20.0,0.5)],
        "mhp_ftse_5_20041019" => vec![p("target","Profit target (% of entry)",2.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",4.0,0.5,20.0,0.5)],
        "mhp_ftse_12_20040917" => vec![p("target","Profit target (% of entry)",2.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",4.0,0.5,20.0,0.5)],
        "mhp_ftse_3_20041007" => vec![p("target","Profit target (% of entry)",2.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",4.0,0.5,20.0,0.5)],
        "mhp_ftse_12_20040930" => vec![p("target","Profit target (% of entry)",2.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",4.0,0.5,20.0,0.5)],
        "mhp_ftse_12_20040728" => vec![p("target","Profit target (% of entry)",2.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",4.0,0.5,20.0,0.5)],
        "mhp_ftse_3_20040804" => vec![p("target","Profit target (% of entry)",2.0,0.5,20.0,0.5), p("stop","Stop-loss (% of entry)",4.0,0.5,20.0,0.5),
            p("path","1 = real 1-minute bars decide stop vs target (default), 0 = [Not from the book] daily bar, stop assumed first",1.0,0.0,1.0,1.0)], // intraday wave 2b
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if !checked(p, &schema) || from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let (logic, dir) = match id {
        "mhp_qqqq_1_20050218" => ("H1>O1&O1>H0&H0>O0&O0>C1&C1>L1&L1>C0&C0>L0", 1.0),
        "mhp_qqqq_11_20041012" => ("L3>H0&H0>L1&L1>L2", 1.0),
        "mhp_qqqq_7_20041101" => ("C0>C2&C2>C1&C1>C3&C3>C4&C4>C5", 1.0),
        "mhp_qqqq_7_20041001" => ("C0>C1&C1>C2&C2>C5&C5>C3&C3>C4", 1.0),
        "mhp_qqqq_5_20050207" => ("C1>C0&C0>C3&C3>C2", 1.0),
        "mhp_qqqq_6_20050120" => ("C2>C3&C3>C1&C1>C4&C4>C0", 1.0),
        "mhp_qqqq_2_20050218" => ("H2>H1&H1>C2&C2>L2&L2>H0&H0>C1&C1>L1&L1>C0&C0>L0", 1.0),
        "mhp_qqqq_4_20040430" => ("H3>H2&H2>L3&L3>H1&H1>L2&L2>H0&H0>L1&L1>L0", 1.0),
        "mhp_qqqq_12_20041111" => ("H0>L3&L3>L2&L2>L1&L1>L4", 1.0),
        "mhp_qqqq_4_20041203" => ("H0>H1&H1>L0&L0>H2&H2>L1&L1>H3&H3>L2&L2>L3", 1.0),
        "mhp_spy_2_20070807" => ("H0>C0&C0>H2&H2>H1&H1>L0&L0>L2&L2>L1", 1.0),
        "mhp_spy_2_20070703" => ("H0>C0&C0>L0&L0>H1&H1>H2&H2>L1&L1>L2", 1.0),
        "mhp_spy_10_20061101" => ("C4>C5&C5>C3&C3>C2&C2>C1&C1>C0", 1.0),
        "mhp_spy_8_20070511" => ("C2>C0&C0>C3&C3>C1", 1.0),
        "mhp_spy_9_20070808" => ("C0>C1&C1>C4&C4>C2&C2>C3", 1.0),
        "mhp_spy_2_20070724" => ("H2>H1&H1>H0&H0>L1&L1>L2&L2>C0&C0>L0", 1.0),
        "mhp_spy_2_20070327" => ("H2>H1&H1>H0&H0>L2&L2>C0&C0>L0&L0>L1", 1.0),
        "mhp_spy_1_20070703" => ("H0>L0&L0>H1&H1>H2&H2>L1&L1>L2", 1.0),
        "mhp_spy_1_20070621" => ("H1>H2&H2>L2&L2>H0&H0>L1&L1>L0", 1.0),
        "mhp_spy_1_20070424" => ("H1>H2&H2>H0&H0>L1&L1>L0&L0>L2", 1.0),
        "mhp_dax_11_20050121" => ("L2>H0&H0>L3&L3>L1", 1.0),
        "mhp_dax_10_20050331" => ("H0>H1&H1>H2&H2>C0&C0>L0&L0>L1&L1>L2", -1.0),
        "mhp_dax_6_20050223" => ("C4>C2&C2>C3&C3>C1&C1>C0", -1.0),
        "mhp_dax_4_20050202" => ("H0>H1&H1>L0&L0>H2&H2>L1&L1>H3&H3>L2&L2>L3", 1.0),
        "mhp_ftse_3_20040712" => ("H1>H0&H0>H2&H2>L1&L1>L0&L0>L2", -1.0),
        "mhp_ftse_12_20041015" => ("L4>H0&H0>L3&L3>L2&L2>L1", 1.0),
        "mhp_ftse_10_20040730" => ("H1>H2&H2>C1&C1>L1&L1>H3&H3>L2&L2>L3", -1.0),
        "mhp_ftse_5_20040922" => ("C1>C0&C0>C3&C3>C2", -1.0),
        "mhp_ftse_5_20041019" => ("C2>C5&C5>C4&C4>C3", -1.0),
        "mhp_ftse_12_20040917" => ("H2>L6&L6>L3&L3>L4&L4>L5", -1.0),
        "mhp_ftse_3_20041007" => ("H3>H4&H4>H5&H5>L4&L4>L3&L3>L5", 1.0),
        "mhp_ftse_12_20040930" => ("H3>L4&L4>L7&L7>L6&L6>L5", 1.0),
        "mhp_ftse_12_20040728" => ("H2>L5&L5>L6&L6>L3&L3>L4", -1.0),
        "mhp_ftse_3_20040804" => ("H4>H3&H3>H5&H5>L4&L4>L5&L5>L3", 1.0),
        _ => return None,
    };
    // EasyLanguage places the limit/stop on the signal bar from "O of tomorrow" -> working on the entry bar
    let minutes = matches!(id, "mhp_ftse_12_20041015" | "mhp_ftse_3_20040804") && g("path") >= 0.5; // intraday wave 2b
    Some(simulate_path(x, from, to, logic, dir, true, true, level_fn(dir, g("target"), g("stop"), true), minutes))
}

// ---- pattern engine (one position per pattern; same code in mhstp/mhpst/mhart so each module stands alone) ----
type Conj = Vec<(u8, usize, u8, usize)>;
/// "L1>H2&H1>L0|C0>C3" = OR of AND-groups of strict inequalities between bar fields (0 = today).
fn parse(logic: &str) -> Vec<Conj> {
    logic.split('|').map(|c| c.split('&').map(|t| {
        let (a, b) = t.split_once('>').expect("pattern term");
        let f = |s: &str| (s.as_bytes()[0], s[1..].parse::<usize>().expect("bar index"));
        let ((x, i), (y, j)) = (f(a.trim()), f(b.trim()));
        (x, i, y, j)
    }).collect()).collect()
}
fn px(x: &Ctx, f: u8, i: usize) -> f64 { match f { b'O' => x.o[i], b'H' => x.h[i], b'L' => x.l[i], _ => x.c[i] } }
fn signal(x: &Ctx, dnf: &[Conj], i: usize) -> bool {
    dnf.iter().any(|c| c.iter().all(|&(a, ai, b, bi)| ai <= i && bi <= i && px(x, a, i - ai) > px(x, b, i - bi)))
}
/// Fixed target/stop around the fill. Same bar: a gap through a level fills at the open; if the bar's range
/// holds both levels the stop is assumed first (book silent; conservative).
fn exit_on(x: &Ctx, i: usize, dir: f64, tp: f64, sp: f64) -> Option<(f64, &'static str)> {
    let (o, h, l) = (x.o[i], x.h[i], x.l[i]);
    if dir > 0. {
        if o <= sp { Some((o, "stop (opened through)")) } else if o >= tp { Some((o, "target (opened through)")) }
        else if l <= sp { Some((sp, "stop")) } else if h >= tp { Some((tp, "target")) } else { None }
    } else if o >= sp { Some((o, "stop (opened through)")) } else if o <= tp { Some((o, "target (opened through)")) }
    else if h >= sp { Some((sp, "stop")) } else if l <= tp { Some((tp, "target")) } else { None }
}
/// at_open: signal on bar i's close, fill at bar i+1's open; else fill at bar i's close.
/// entry_bar_exits: target/stop already working on the open-entry bar (otherwise from the next bar).
fn simulate(x: &Ctx, from: usize, to: usize, logic: &str, dir: f64, at_open: bool, entry_bar_exits: bool,
            levels: impl Fn(f64) -> (f64, f64)) -> Vec<Trade> { simulate_path(x, from, to, logic, dir, at_open, entry_bar_exits, levels, false) }
// ---- intraday wave 2b block: target/stop order from real 1-minute bars
/// On a held day with minute data the first of stop/target touched in time order decides (at the level, or the minute's
/// open when it opened through it; both in one minute: the level nearer the minute's open [convention]). A day without
/// minutes keeps the daily reading, which is exact unless both levels are inside that day's bar (stop assumed first).
fn exit_minutes(x: &Ctx, i: usize, dir: f64, tp: f64, sp: f64) -> Option<(f64, &'static str)> {
    use crate::intraday::Level;
    let Some(b) = crate::intraday_w2b::day(x, i) else { return exit_on(x, i, dir, tp, sp) };
    let lv = [Level { up: dir < 0., px: sp }, Level { up: dir > 0., px: tp }];
    crate::intraday_w2b::first_exit(&b, 0, &lv).map(|(k, bar, px, _)| (px, match (k, bar == 0 && px == b[0].o) {
        (0, true) => "stop (opened through)", (0, false) => "stop", (_, true) => "target (opened through)", _ => "target" }))
}
// ---- end intraday wave 2b block ----
fn simulate_path(x: &Ctx, from: usize, to: usize, logic: &str, dir: f64, at_open: bool, entry_bar_exits: bool,
            levels: impl Fn(f64) -> (f64, f64), minutes: bool) -> Vec<Trade> {
    let dnf = parse(logic);
    let mut out = vec![];
    let mut cur: Option<(Trade, f64, f64, usize)> = None;
    for i in from..=to {
        if let Some((t, tp, sp, active)) = cur.as_mut() {
            if i >= *active {
                if let Some((p, why)) = if minutes { exit_minutes(x, i, dir, *tp, *sp) } else { exit_on(x, i, dir, *tp, *sp) } { // intraday wave 2b
                    t.exit_i = i; t.exit_px = p; t.signal_i = i; t.reason = why.into();
                    out.push(cur.take().unwrap().0);
                }
            }
        }
        if cur.is_some() || !signal(x, &dnf, i) { continue; }
        let (k, fill) = if at_open { if i >= to { continue; } (i + 1, x.o[i + 1]) } else { (i, x.c[i]) };
        if !fill.is_finite() { continue; }
        let (tp, sp) = levels(fill);
        let active = if at_open && entry_bar_exits { k } else { k + 1 };
        let t = Trade { dir, legs: vec![Leg { i: k, px: fill, w: 1. }], exit_i: k, exit_px: fill, reason: String::new(),
            entry_kind: if at_open { "open" } else { "close" }, signal_i: i, open: false, exits: vec![] };
        cur = Some((t, tp, sp, active));
    }
    if let Some((mut t, ..)) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}
fn level_fn(dir: f64, target: f64, stop: f64, pct: bool) -> impl Fn(f64) -> (f64, f64) {
    move |e| if pct { (e * (1. + dir * target / 100.), e * (1. - dir * stop / 100.)) } else { (e + dir * target, e - dir * stop) }
}
fn checked(p: &Params, schema: &[ParamDef]) -> bool {
    p.iter().all(|(k, v)| schema.iter().any(|d| d.key == k && v.is_finite() && *v >= d.min && *v <= d.max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(bars: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        bars.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("{:04}-01-01", 2000 + i), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn parser_and_signal_follow_bar_labels() {
        // bar 0 = today; "C0>C1" true when today's close is above yesterday's
        let r = rows(&[(1., 2., 0.5, 1.), (1., 3., 0.5, 2.)]);
        let x = Ctx::new(&r, &r, &[]);
        assert!(signal(&x, &parse("C0>C1"), 1));
        assert!(!signal(&x, &parse("C1>C0"), 1));
        assert!(!signal(&x, &parse("C0>C2"), 1)); // not enough history
        assert!(signal(&x, &parse("C1>C0|H0>H1"), 1)); // OR group
    }
    #[test]
    fn open_entry_target_stop_and_gap_fills() {
        // signal at bar 1 (C0>C1), fill bar 2 open 100; target 105 / stop 95 (5 points)
        let r = rows(&[(10., 11., 9., 10.), (10., 12., 9., 11.), (100., 104., 96., 100.), (100., 106., 99., 101.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = simulate(&x, 0, 3, "C0>C1", 1., true, true, level_fn(1., 5., 5., false));
        assert_eq!(t[0].legs[0].i, 2); assert_eq!(t[0].legs[0].px, 100.);
        assert_eq!(t[0].exit_i, 3); assert_eq!(t[0].exit_px, 105.);
        // short, opened through the stop
        let r = rows(&[(10., 11., 9., 10.), (10., 12., 8., 9.), (100., 101., 99., 100.), (110., 112., 108., 111.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = simulate(&x, 0, 3, "C1>C0", -1., true, true, level_fn(-1., 5., 5., false));
        assert_eq!(t[0].exit_i, 3); assert_eq!(t[0].exit_px, 110.); assert_eq!(t[0].reason, "stop (opened through)");
    }
    #[test]
    fn both_levels_in_one_bar_assume_stop_and_close_entry_waits_a_bar() {
        let r = rows(&[(10., 11., 9., 10.), (10., 12., 9., 100.), (100., 120., 80., 100.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = simulate(&x, 0, 2, "C0>C1", 1., false, false, level_fn(1., 10., 10., true));
        assert_eq!(t[0].legs[0].px, 100.); assert_eq!(t[0].exit_i, 2); assert_eq!(t[0].exit_px, 90.);
        // entry_bar_exits=false: the open-entry bar's range is ignored, exit on the next bar
        let r = rows(&[(10., 11., 9., 10.), (10., 12., 9., 11.), (100., 120., 80., 100.), (100., 101., 99., 100.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = simulate(&x, 0, 3, "C0>C1", 1., true, false, level_fn(1., 10., 10., true));
        assert!(t[0].open);
    }
    #[test]
    fn catalogue_ids_unique_and_params_default_to_book() {
        let cat = catalogue();
        let mut ids: Vec<_> = cat.iter().map(|d| d.id).collect(); ids.sort(); ids.dedup();
        assert_eq!(ids.len(), cat.len());
        for d in cat.iter().filter(|d| d.status == Status::Live) {
            assert!(params(d.id).is_some() && family_of(d.id).is_some());
            let r = rows(&[(1., 2., 0.5, 1.); 12]); let x = Ctx::new(&r, &r, &[]);
            assert!(run(d.id, &x, 0, 11, &Params::new()).is_some());
        }
    }
}

// Instrument admission: Harris 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/harris/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"mhpst"|"mhart") && (crate::books::mhpst::reviewed(def.id) || crate::books::mhart::reviewed(def.id)),
    label: "Michael Harris tested these price patterns on QQQQ, SPY, the DAX and the FTSE (Profitability and Systematic Trading appendix) and on Intel and Dell (Active Trader, Sep 2001); runs on other indices, ETFs and stocks, US and India, are a declared application, not the author's test. Short patterns on a cash index are hypothetical exposure.",
    basis: "Michael Harris: Profitability and Systematic Trading (Wiley 2008) appendix price patterns with percentage targets and stops (p.137-178, EasyLanguage listings) and the 3L-R and GAP-2H patterns (Active Trader, Sep 2001, Tables 2-3). Reviewed in authors/harris/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Entry at the next open, exits at the printed limit and stop; same-bar target and stop take the stop first (book silent); no costs.",
};
