//! Michael Harris, Short-Term Trading with Price Patterns (Traders Press, 1998/2000).
//! Pattern Library (Ch.14, p.131-206) + Ch.7 example. Each pattern: exact bar inequalities, entry at the close or next open,
//! fixed profit target / protective stop in contract points (GTC). Source pages, oracle and results: library/mhstp/.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mhstp";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("mhs_ua830406_1", "UA830406-1 · Exact long · 1/1 pts · close", "Pattern Library · T-Bond (day session)", "Long", Status::NeedsData("Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion."),
            &["F|Pattern UA830406-1 (T-Bond (day session)), type Exact, position Long STTPP p.138", "E|\"Logic: L(1) > H(2) and H(1) > L(0) and H(3) > L(2) and L(2) > L(3) and L(0) > L(1) and H(0) > H(1) and H(3) > L(2)\" p.138", "E|\"Order Entry: Close, C(0)\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.138", "X|\"Profit Target: 1 point\" and \"Protective Stop: 1 point\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_ua830407_1", "UA830407-1 · Exact long · 1/1 pts · next open", "Pattern Library · T-Bond (day session)", "Long", Status::NeedsData("Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion."),
            &["F|Pattern UA830407-1 (T-Bond (day session)), type Exact, position Long STTPP p.140", "E|\"Logic: C(0) > C(3) and C(1) > C(2) and C(3) > C(4) and C(2) > C(0)\" p.140", "E|\"Order Entry: Open of Tomorrow, O\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.140", "X|\"Profit Target: 1 point\" and \"Protective Stop: 1 point\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_ua830826_1", "UA830826-1 · Delay-Exact long · 1/1 pts · close", "Pattern Library · T-Bond (day session)", "Long", Status::NeedsData("Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion."),
            &["F|Pattern UA830826-1 (T-Bond (day session)), type Delay-Exact, position Long STTPP p.142", "E|\"Logic: H(4) > H(5) and H(3) > L(4) and L(5) > H(3) and L(4) > L(3) and H(5) > L(5)\" p.142", "E|\"Order Entry: Close, C(0)\" ; \"Order delay: 3\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.142", "X|\"Profit Target: 1 point\" and \"Protective Stop: 1 point\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_ua830131_1", "UA830131-1 · Exact short · 1/1 pts · close", "Pattern Library · T-Bond (day session)", "Short", Status::NeedsData("Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion."),
            &["F|Pattern UA830131-1 (T-Bond (day session)), type Exact, position Short STTPP p.144", "E|\"Logic: C(3) > C(0) and C(1) > C(3) and C(2) > C(1) and C(4) > C(2)\" p.144", "E|\"Order Entry: Close, C(0)\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.144", "X|\"Profit Target: 1 point\" and \"Protective Stop: 1 point\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_ua830304_1", "UA830304-1 · Delay-Exact long · 2/2 pts · close", "Pattern Library · T-Bond (day session)", "Long", Status::NeedsData("Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion."),
            &["F|Pattern UA830304-1 (T-Bond (day session)), type Delay-Exact, position Long STTPP p.146", "E|\"Logic: C(2) > C(4) and C(1) > C(3) and C(4) > C(1)\" p.146", "E|\"Order Entry: Close, C(0)\" ; \"Order delay: 1\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.146", "X|\"Profit Target: 2 points\" and \"Protective Stop: 2 points\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_ua830125_1", "UA830125-1 · Exact long · 0.5/0.5 pts · close", "Pattern Library · T-Bond (day session)", "Long", Status::NeedsData("Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion."),
            &["F|Pattern UA830125-1 (T-Bond (day session)), type Exact, position Long STTPP p.148", "E|\"Logic: H(1) > L(0) and H(0) > L(2) and H(2) > H(0) and L(0) > L(1) and L(2) > H(1)\" p.148", "E|\"Order Entry: Close, C(0)\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.148", "X|\"Profit Target: 0.5 points\" and \"Protective Stop: 0.5 points\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_ua830418_1", "UA830418-1 · Exact short · 0.5/0.5 pts · close", "Pattern Library · T-Bond (day session)", "Short", Status::NeedsData("Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion."),
            &["F|Pattern UA830418-1 (T-Bond (day session)), type Exact, position Short STTPP p.150", "E|\"Logic: H(3) > L(2) and H(2) > L(0) and L(1) > H(3) and H(0) > H(1) and L(2) > L(3) and L(0) > L(1) and H(1) > H(2)\" p.150", "E|\"Order Entry: Close, C(0)\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.150", "X|\"Profit Target: 0.5 points\" and \"Protective Stop: 0.5 points\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_us800310_1", "US800310-1 · Exact long · 1/1 pts · close", "Pattern Library · T-Bond (incl. evening session)", "Long", Status::NeedsData("Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion."),
            &["F|Pattern US800310-1 (T-Bond (incl. evening session)), type Exact, position Long STTPP p.152", "E|\"Logic: C(0) > O(2) and O(0) > C(1) and C(2) > O(1) and C(1) > C(2) and O(2) > O(0)\" p.152", "E|\"Order Entry: Close, C(0)\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.152", "X|\"Profit Target: 1 point\" and \"Protective Stop: 1 point\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_us801701_1", "US801701-1 · Exact short · 1/1 pts · close", "Pattern Library · T-Bond (incl. evening session)", "Short", Status::NeedsData("Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion."),
            &["F|Pattern US801701-1 (T-Bond (incl. evening session)), type Exact, position Short STTPP p.154", "E|\"Logic: C(3) > C(0) and C(2) > C(1) and C(4) > C(2) and C(1) > C(3)\" p.154", "E|\"Order Entry: Close, C(0)\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.154", "X|\"Profit Target: 1 point\" and \"Protective Stop: 1 point\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_us810220_1", "US810220-1 · Exact short · 1/1 pts · next open", "Pattern Library · T-Bond (incl. evening session)", "Short", Status::NeedsData("Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion."),
            &["F|Pattern US810220-1 (T-Bond (incl. evening session)), type Exact, position Short STTPP p.156", "E|\"Logic: C(1) > H(2) and C(2) > L(1) and H(0) > C(0) and H(2) > L(0) and H(1) > C(1) and L(0) > C(2) and C(0) > H(1) and L(1) > L(2)\" p.156", "E|\"Order Entry: Open of Tomorrow, O\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.156", "X|\"Profit Target: 1 point\" and \"Protective Stop: 1 point\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_us800111_1", "US800111-1 · Exact short · 2/2 pts · close", "Pattern Library · T-Bond (incl. evening session)", "Short", Status::NeedsData("Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion."),
            &["F|Pattern US800111-1 (T-Bond (incl. evening session)), type Exact, position Short STTPP p.158", "E|\"Logic: C(2) > C(0) and C(3) > C(2) and C(1) > C(3)\" p.158", "E|\"Order Entry: Close, C(0)\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.158", "X|\"Profit Target: 2 points\" and \"Protective Stop: 2 points\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_cl840221_1", "CL840221-1 · Exact long · 1/1 pts · close", "Pattern Library · NYMEX Crude Oil", "Long", Status::NeedsData("Needs NYMEX crude oil futures daily OHLC (1984-1999). Targets/stops are in crude points (1 point = $1,000); no equity-index equivalent without inventing a conversion."),
            &["F|Pattern CL840221-1 (NYMEX Crude Oil), type Exact, position Long STTPP p.162", "E|\"Logic: L(1) > H(2) and H(1) > L(0) and H(3) > L(2) and L(2) > L(3) and L(0) > L(1) and H(0) > H(1) and H(3) > L(2)\" p.162", "E|\"Order Entry: Close, C(0)\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.162", "X|\"Profit Target: 1 point\" and \"Protective Stop: 1 point\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs NYMEX crude oil futures daily OHLC (1984-1999). Targets/stops are in crude points (1 point = $1,000); no equity-index equivalent without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_cl850210_1", "CL850210-1 · Exact long · 1/1 pts · close", "Pattern Library · NYMEX Crude Oil", "Long", Status::NeedsData("Needs NYMEX crude oil futures daily OHLC (1984-1999). Targets/stops are in crude points (1 point = $1,000); no equity-index equivalent without inventing a conversion."),
            &["F|Pattern CL850210-1 (NYMEX Crude Oil), type Exact, position Long STTPP p.164", "E|\"Logic: C(2) > C(1) and C(0) > C(3) and C(1) > C(4) and C(3) > C(2)\" p.164", "E|\"Order Entry: Close, C(0)\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.164", "X|\"Profit Target: 1 point\" and \"Protective Stop: 1 point\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs NYMEX crude oil futures daily OHLC (1984-1999). Targets/stops are in crude points (1 point = $1,000); no equity-index equivalent without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_cl850919_1", "CL850919-1 · Exact long · 1/1 pts · close", "Pattern Library · NYMEX Crude Oil", "Long", Status::NeedsData("Needs NYMEX crude oil futures daily OHLC (1984-1999). Targets/stops are in crude points (1 point = $1,000); no equity-index equivalent without inventing a conversion."),
            &["F|Pattern CL850919-1 (NYMEX Crude Oil), type Exact, position Long STTPP p.166", "E|\"Logic: H(1) > H(2) and L(0) > L(1) and H(0) > C(0) and H(2) > O(0) and O(0) > L(0) and L(0) > L(1) and L(1) > L(2)\" p.166", "E|\"Order Entry: Close, C(0)\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.166", "X|\"Profit Target: 1 point\" and \"Protective Stop: 1 point\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs NYMEX crude oil futures daily OHLC (1984-1999). Targets/stops are in crude points (1 point = $1,000); no equity-index equivalent without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_cl841206_1", "CL841206-1 · Exact long · 1/1 pts · next open", "Pattern Library · NYMEX Crude Oil", "Long", Status::NeedsData("Needs NYMEX crude oil futures daily OHLC (1984-1999). Targets/stops are in crude points (1 point = $1,000); no equity-index equivalent without inventing a conversion."),
            &["F|Pattern CL841206-1 (NYMEX Crude Oil), type Exact, position Long STTPP p.168", "E|\"Logic: H(2) > C(2) and L(0) > H(2) and H(0) > H(1) and C(0) > L(0) and H(1) > C(1) and C(2) > L(1) and L(1) > L(2) and C(1) > C(0)\" p.168", "E|\"Order Entry: Open of Tomorrow, O\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.168", "X|\"Profit Target: 1 point\" and \"Protective Stop: 1 point\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs NYMEX crude oil futures daily OHLC (1984-1999). Targets/stops are in crude points (1 point = $1,000); no equity-index equivalent without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_sf830520_1", "SF830520-1 · Exact short · 1/1 pts · close", "Pattern Library · IMM Swiss Franc", "Short", Status::NeedsData("Needs IMM Swiss franc futures daily OHLC (1983-1999). Targets/stops are in franc points (1 point = $1,250); no equity-index equivalent without inventing a conversion."),
            &["F|Pattern SF830520-1 (IMM Swiss Franc), type Exact, position Short STTPP p.172", "E|\"Logic: C(1) > C(0) and C(2) > O(1) and O(0) > C(1) and O(2) > C(2) and O(1) > O(0)\" p.172", "E|\"Order Entry: Close, C(0)\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.172", "X|\"Profit Target: 1 point\" and \"Protective Stop: 1 point\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs IMM Swiss franc futures daily OHLC (1983-1999). Targets/stops are in franc points (1 point = $1,250); no equity-index equivalent without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_sf831011_3", "SF831011-3 · Matched short · 1/1 pts · next open", "Pattern Library · IMM Swiss Franc", "Short", Status::NeedsData("Needs IMM Swiss franc futures daily OHLC (1983-1999). Targets/stops are in franc points (1 point = $1,250); no equity-index equivalent without inventing a conversion."),
            &["F|Pattern SF831011-3 (IMM Swiss Franc), type Matched, position Short STTPP p.174", "E|\"Logic: C(3) > C(2) and C(2) > C(1) and C(4) > C(0) and C(0) > C(3) OR C(3) > C(2) and C(2) > C(1) and C(4) > C(0) and C(0) > C(2) and C(3) > C(4)\" p.174", "E|\"Order Entry: Open of Tomorrow, O\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.174", "X|\"Profit Target: 1 point\" and \"Protective Stop: 1 point\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs IMM Swiss franc futures daily OHLC (1983-1999). Targets/stops are in franc points (1 point = $1,250); no equity-index equivalent without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_sf830519_1", "SF830519-1 · Delay-Exact long · 1/1 pts · close", "Pattern Library · IMM Swiss Franc", "Long", Status::NeedsData("Needs IMM Swiss franc futures daily OHLC (1983-1999). Targets/stops are in franc points (1 point = $1,250); no equity-index equivalent without inventing a conversion."),
            &["F|Pattern SF830519-1 (IMM Swiss Franc), type Delay-Exact, position Long STTPP p.176", "E|\"Logic: C(7) > C(3) and C(6) > C(5) and C(4) > C(7) and C(5) > C(4)\" p.176", "E|\"Order Entry: Close, C(0)\" ; \"Order delay: 3\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.176", "X|\"Profit Target: 1 point\" and \"Protective Stop: 1 point\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs IMM Swiss franc futures daily OHLC (1983-1999). Targets/stops are in franc points (1 point = $1,250); no equity-index equivalent without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_sp830825_1", "SP830825-1 · Exact long · 10/10 pts · next open", "Pattern Library · CME S&P 500 Index", "Long", Status::Live,
            &["F|Pattern SP830825-1 (CME S&P 500 Index), type Exact, position Long STTPP p.180", "E|\"Logic: H(0) > L(1) and L(2) > H(0) and H(1) > L(2) and H(2) > L(3) and L(1) > L(0) and L(3) > H(1) and H(3) > H(2)\" p.180", "E|\"Order Entry: Open of Tomorrow, O\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.180", "X|\"Profit Target: 10 points\" and \"Protective Stop: 10 points\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "S&P 500 futures pattern run on the cash index / ETF (instrument swap; NIFTY 50 for India). Book defaults are literal S&P index POINTS (10 = $2,500 on the 1983-99 contract); on today's index levels 10 points is a tiny move, so the [Not from the book] pct_target/pct_stop options express them as % of entry instead (0 = book points). Open entry: target/stop work on the entry day (GTC with the entry, p.63). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. One position per pattern; signals while in a position are ignored (p.65). SPX/DJI/NDX opens before ~1999 in the vault data equal the prior close, so SPX trades before then fill at the prior close, not the real open.", claim("27","70%","$1,021.35 per contract","22 bars winners / 6 losers","2/15/83-1/7/99 S&P futures")),
        d("mhs_sp830120_1", "SP830120-1 · Exact long · 10/10 pts · next open", "Pattern Library · CME S&P 500 Index", "Long", Status::Live,
            &["F|Pattern SP830120-1 (CME S&P 500 Index), type Exact, position Long STTPP p.182", "E|\"Logic: H(0) > C(0) and L(2) > L(0) and H(1) > H(0) and H(2) > H(1) and L(0) > L(1) and C(0) > L(2)\" p.182", "E|\"Order Entry: Open of Tomorrow, O\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.182", "X|\"Profit Target: 10 points\" and \"Protective Stop: 10 points\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "S&P 500 futures pattern run on the cash index / ETF (instrument swap; NIFTY 50 for India). Book defaults are literal S&P index POINTS (10 = $2,500 on the 1983-99 contract); on today's index levels 10 points is a tiny move, so the [Not from the book] pct_target/pct_stop options express them as % of entry instead (0 = book points). Open entry: target/stop work on the entry day (GTC with the entry, p.63). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. One position per pattern; signals while in a position are ignored (p.65). SPX/DJI/NDX opens before ~1999 in the vault data equal the prior close, so SPX trades before then fill at the prior close, not the real open.", claim("50","64%","$687.75 per contract","16 bars winners / 10 losers","2/15/83-1/7/99 S&P futures")),
        d("mhs_sp830330_1", "SP830330-1 · Exact long · 10/10 pts · next open", "Pattern Library · CME S&P 500 Index", "Long", Status::Live,
            &["F|Pattern SP830330-1 (CME S&P 500 Index), type Exact, position Long STTPP p.184", "E|\"Logic: H(0) > C(0) and L(2) > L(1) and H(1) > H(2) and H(2) > L(0) and C(0) > H(1) and L(0) > L(2)\" p.184", "E|\"Order Entry: Open of Tomorrow, O\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.184", "X|\"Profit Target: 10 points\" and \"Protective Stop: 10 points\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "S&P 500 futures pattern run on the cash index / ETF (instrument swap; NIFTY 50 for India). Book defaults are literal S&P index POINTS (10 = $2,500 on the 1983-99 contract); on today's index levels 10 points is a tiny move, so the [Not from the book] pct_target/pct_stop options express them as % of entry instead (0 = book points). Open entry: target/stop work on the entry day (GTC with the entry, p.63). Same-bar target+stop: stop assumed first (book silent). Opened through a level: fill at the open. One position per pattern; signals while in a position are ignored (p.65). SPX/DJI/NDX opens before ~1999 in the vault data equal the prior close, so SPX trades before then fill at the prior close, not the real open.", claim("37","70%","$991.46 per contract","11 bars winners / 15 losers","2/15/83-1/7/99 S&P futures")),
        d("mhs_si830520_1", "SI830520-1 · Exact short · 40/40 pts · next open", "Pattern Library · COMEX Silver", "Short", Status::NeedsData("Needs COMEX silver futures daily OHLC (1983-1999). Targets/stops are in silver points (cents; 20 points = $1,000); no equity-index equivalent without inventing a conversion."),
            &["F|Pattern SI830520-1 (COMEX Silver), type Exact, position Short STTPP p.188", "E|\"Logic: H(0) > C(1) and L(2) > H(0) and H(1) > C(2) and C(2) > L(2) and L(1) > C(0) and C(0) > L(0) and H(2) > H(1) and C(1) > L(1)\" p.188", "E|\"Order Entry: Open of Tomorrow, O\" ; \"Order delay: 0\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.188", "X|\"Profit Target: 40 points\" and \"Protective Stop: 40 points\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs COMEX silver futures daily OHLC (1983-1999). Targets/stops are in silver points (cents; 20 points = $1,000); no equity-index equivalent without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_si830630_1", "SI830630-1 · Delay-Exact short · 20/20 pts · next open", "Pattern Library · COMEX Silver", "Short", Status::NeedsData("Needs COMEX silver futures daily OHLC (1983-1999). Targets/stops are in silver points (cents; 20 points = $1,000); no equity-index equivalent without inventing a conversion."),
            &["F|Pattern SI830630-1 (COMEX Silver), type Delay-Exact, position Short STTPP p.190", "E|\"Logic: C(4) > C(3) and C(6) > C(4) and C(7) > C(6) and C(3) > C(5)\" p.190", "E|\"Order Entry: Open of Tomorrow, O\" ; \"Order delay: 3\" (bar labels already include the delay, Ch.8 p.77 / Ch.14 p.132) p.190", "X|\"Profit Target: 20 points\" and \"Protective Stop: 20 points\" from the entry price, good-til-cancelled (p.63: \"a profit target and a stop loss order are also put in place as a good 'til cancelled order\")"],
            "Needs COMEX silver futures daily OHLC (1983-1999). Targets/stops are in silver points (cents; 20 points = $1,000); no equity-index equivalent without inventing a conversion. Code is in place (same engine as the live S&P patterns) and runs on a ticker of that futures market when its data is added.", None),
        d("mhs_hd_rt_long", "H&D RT LONG (Ch.7 example) · T-Bond · close", "Ch.7 Developing a Trading System", "Long", Status::NeedsData("Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion."),
            &["E|\"Buy today on the close if Close of yesterday < Low of two days ago and Low of yesterday < Low of today and Close of today > High of two days ago and Close of today > Open of today\" STTPP p.64", "E|EasyLanguage Fig 7-4: condition1=c(1) < l(2); condition2=l(1) < l(0); condition3=c(0) > o(0); condition4=c(0) > h(2); \"BUY TODAY GVALUE1 CONTRACTS ON THE CLOSE\" p.66", "X|\"PROFIT TARGET AT CLOSE OF TODAY + GVALUE32 POINTS; PROTECTIVE STOP AT CLOSE OF TODAY - GVALUE32 POINTS\" (32/32 = 1 full T-Bond point = $1,000, p.56) p.66"],
            "Book test: CBOT T-Bond futures 01/01/86-01/01/98, $14 commission (p.67, Fig 7-6: 22 trades, 72%, net $9,692.80). Needs CBOT 30-year T-Bond futures daily OHLC (day session or incl. evening session as the pattern states; back-adjusted continuous, Feb 1983-Jan 1999). Targets/stops are in T-Bond points (1 point = $1,000) and cannot be carried to an equity index without inventing a conversion.", claim("22","72%","$440.55 per contract","4 bars winners / 4 losers","01/01/86-01/01/98 T-Bond futures")),
        d("mhs_sp830128_2", "SP830128-2 · inter-market · S&P 500 pattern -> long T-Bonds (day session)", "Pattern Library · Inter-Market", "Long", Status::NeedsData("Inter-market: the pattern forms on one contract and the position is taken in T-Bonds; needs both futures series (daily, 1983-1999) and T-Bonds point targets. The vault runs one instrument per strategy."),
            &["E|\"Logic: H(2) > C(2) and L(0) > L(1) and H(0) > H(1) and C(0) > L(1) and H(1) > C(1) and C(2) > L(2) and L(1) > H(2) and C(1) > C(0)\" (bars of the signal market) STTPP p.194", "E|\"Order Entry: Open of Tomorrow, O\"; \"Order delay: 0\"; position in T-Bonds p.194", "X|\"Profit Target: 1 point\"; \"Protective Stop: 1 point\" (T-Bonds points) p.194"],
            "Inter-market: the pattern forms on one contract and the position is taken in T-Bonds; needs both futures series (daily, 1983-1999) and T-Bonds point targets. The vault runs one instrument per strategy.", None),
        d("mhs_sp830107_2", "SP830107-2 · inter-market · S&P 500 pattern -> long T-Bonds (day session)", "Pattern Library · Inter-Market", "Long", Status::NeedsData("Inter-market: the pattern forms on one contract and the position is taken in T-Bonds; needs both futures series (daily, 1983-1999) and T-Bonds point targets. The vault runs one instrument per strategy."),
            &["E|\"Logic: H(2) > C(2) and L(0) > H(2) and H(0) > H(1) and C(0) > C(1) and H(1) > C(0) and C(2) > L(1) and L(1) > L(2) and C(1) > L(0)\" (bars of the signal market) STTPP p.196", "E|\"Order Entry: Open of Tomorrow, O\"; \"Order delay: 0\"; position in T-Bonds p.196", "X|\"Profit Target: 1 point\"; \"Protective Stop: 1 point\" (T-Bonds points) p.196"],
            "Inter-market: the pattern forms on one contract and the position is taken in T-Bonds; needs both futures series (daily, 1983-1999) and T-Bonds point targets. The vault runs one instrument per strategy.", None),
        d("mhs_sp830825_2", "SP830825-2 · inter-market · S&P 500 pattern -> long T-Bonds", "Pattern Library · Inter-Market", "Long", Status::NeedsData("Inter-market: the pattern forms on one contract and the position is taken in T-Bonds; needs both futures series (daily, 1983-1999) and T-Bonds point targets. The vault runs one instrument per strategy."),
            &["E|\"Logic: H(2) > L(3) and L(2) > H(0) and H(1) > L(2) and H(0) > L(1) and L(3) > H(1) and H(3) > H(2) and L(1) > L(0)\" (bars of the signal market) STTPP p.198", "E|\"Order Entry: Open of Tomorrow, O\"; \"Order delay: 0\"; position in T-Bonds p.198", "X|\"Profit Target: 2 points\"; \"Protective Stop: 2 points\" (T-Bonds points) p.198"],
            "Inter-market: the pattern forms on one contract and the position is taken in T-Bonds; needs both futures series (daily, 1983-1999) and T-Bonds point targets. The vault runs one instrument per strategy.", None),
        d("mhs_sp830405_2", "SP830405-2 · inter-market · S&P 500 pattern -> long T-Bonds (day session)", "Pattern Library · Inter-Market", "Long", Status::NeedsData("Inter-market: the pattern forms on one contract and the position is taken in T-Bonds; needs both futures series (daily, 1983-1999) and T-Bonds point targets. The vault runs one instrument per strategy."),
            &["E|\"Logic: H(1) > L(1) and L(2) > L(0) and H(0) > H(1) and H(2) > H(0) and L(1) > L(0)\" (bars of the signal market) STTPP p.200", "E|\"Order Entry: Open of Tomorrow, O\"; \"Order delay: 0\"; position in T-Bonds p.200", "X|\"Profit Target: 1 point\"; \"Protective Stop: 1 point\" (T-Bonds points) p.200"],
            "Inter-market: the pattern forms on one contract and the position is taken in T-Bonds; needs both futures series (daily, 1983-1999) and T-Bonds point targets. The vault runs one instrument per strategy.", None),
        d("mhs_us881011_2", "US881011-2 · inter-market · T-Bond pattern -> long S&P 500 (model 'US TO SP')", "Pattern Library · Inter-Market", "Long", Status::NeedsData("Inter-market: the pattern forms on one contract and the position is taken in S&P 500; needs both futures series (daily, 1983-1999) and S&P 500 point targets. The vault runs one instrument per strategy."),
            &["E|\"Logic: C(0) > C(3) and C(2) > C(1) and C(1) > C(0)\" (bars of the signal market) STTPP p.202", "E|\"Order Entry: Open of Tomorrow, O\"; \"Order delay: 0\"; position in S&P 500 p.202", "X|\"Profit Target: 10 points\"; \"Protective Stop: 10 points\" (S&P 500 points) p.202"],
            "Inter-market: the pattern forms on one contract and the position is taken in S&P 500; needs both futures series (daily, 1983-1999) and S&P 500 point targets. The vault runs one instrument per strategy.", None),
        d("mhs_sf850419_2", "SF850419-2 · inter-market · Swiss Franc pattern -> short COMEX Silver", "Pattern Library · Inter-Market", "Short", Status::NeedsData("Inter-market: the pattern forms on one contract and the position is taken in COMEX Silver; needs both futures series (daily, 1983-1999) and COMEX Silver point targets. The vault runs one instrument per strategy."),
            &["E|\"Logic: L(1) > L(2) and H(1) > H(0) and H(2) > L(1) and H(0) > L(0) and L(0) > H(2)\" (bars of the signal market) STTPP p.204", "E|\"Order Entry: Open of Tomorrow, O\"; \"Order delay: 0\"; position in COMEX Silver p.204", "X|\"Profit Target: 20 points\"; \"Protective Stop: 20 points\" (COMEX Silver points) p.204"],
            "Inter-market: the pattern forms on one contract and the position is taken in COMEX Silver; needs both futures series (daily, 1983-1999) and COMEX Silver point targets. The vault runs one instrument per strategy.", None),
        d("mhs_p_indicator", "p-Indicator (Ch.13)", "Ch.13 The p-Indicator", "Long/Short", Status::NotTestable("The p-Indicator is a weighted profitability of a set of patterns (10 pattern forms x 2 target/stop pairs, p.123) whose logic is never printed; the long/short thresholds (a=60, b=54) cannot be applied without the pattern set."),
            &["F|\"PIL = sum(PLi*Ti)/sum(Ti)\" over N pattern forms; \"Long Position if PIL > a, a > 50\"; example a = 60, b = 54 on CBOT T-Bonds 1/18/90-9/21/98 (103 trades, 67%) p.120-124", "E|\"If PIL > a then buy at the open of tomorrow with profit target at open of tomorrow + 1 point, protective stop at open of tomorrow - 1 point\" p.124"],
            "The p-Indicator is a weighted profitability of a set of patterns (10 pattern forms x 2 target/stop pairs, p.123) whose logic is never printed; the long/short thresholds (a=60, b=54) cannot be applied without the pattern set.", None),
        d("mhs_daily_report", "Daily-report candidate patterns (Fig 7-7, 10-2)", "Ch.7/Ch.10 Daily operation", "Long/Short", Status::NotTestable("The daily reports list only the next-day price conditions of candidate patterns (e.g. 'CLOSE > 16.45') for one date; the full pattern logic is not printed, so no backtest is possible."),
            &["E|Fig 7-7 (T-Bonds, 02/18/98: BOTTOM, REVERSE, RECOVER, REVERSAL, COVERUP, SLIDE, INSIDE, UNDER, SPIRAL) and Fig 10-2 (crude oil cl55cl0 / cl11cl0) show only next-day conditions p.70, p.93"],
            "The daily reports list only the next-day price conditions of candidate patterns (e.g. 'CLOSE > 16.45') for one date; the full pattern logic is not printed, so no backtest is possible.", None),
        d("mhs_pattern_model", "Multi-pattern trading model / trend following (Ch.7, Ch.12)", "Ch.12 Advanced ways of using price patterns", "Long/Short", Status::NotTestable("The author's T-Bond model of 30 long and 25 short patterns (p.59) and the trend-following test of Fig 12-4/12-5 use patterns that are not listed."),
            &["F|\"The trading system model I developed back in the early 90's had 30 patterns that could generate long entry signals and 25 that could generate short entry signals\" p.59", "X|Coincident signals: trader \"may decide to ignore the new signal, while moving the profit target and protective stop to a new position\" (a discretionary choice) p.115"],
            "The author's T-Bond model of 30 long and 25 short patterns (p.59) and the trend-following test of Fig 12-4/12-5 use patterns that are not listed.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "mhs_ua830406_1" => ("mhs_tb", "T-Bond patterns", "UA830406-1"),
        "mhs_ua830407_1" => ("mhs_tb", "T-Bond patterns", "UA830407-1"),
        "mhs_ua830826_1" => ("mhs_tb", "T-Bond patterns", "UA830826-1"),
        "mhs_ua830131_1" => ("mhs_tb", "T-Bond patterns", "UA830131-1"),
        "mhs_ua830304_1" => ("mhs_tb", "T-Bond patterns", "UA830304-1"),
        "mhs_ua830125_1" => ("mhs_tb", "T-Bond patterns", "UA830125-1"),
        "mhs_ua830418_1" => ("mhs_tb", "T-Bond patterns", "UA830418-1"),
        "mhs_us800310_1" => ("mhs_tb", "T-Bond patterns", "US800310-1"),
        "mhs_us801701_1" => ("mhs_tb", "T-Bond patterns", "US801701-1"),
        "mhs_us810220_1" => ("mhs_tb", "T-Bond patterns", "US810220-1"),
        "mhs_us800111_1" => ("mhs_tb", "T-Bond patterns", "US800111-1"),
        "mhs_cl840221_1" => ("mhs_cl", "Crude oil patterns", "CL840221-1"),
        "mhs_cl850210_1" => ("mhs_cl", "Crude oil patterns", "CL850210-1"),
        "mhs_cl850919_1" => ("mhs_cl", "Crude oil patterns", "CL850919-1"),
        "mhs_cl841206_1" => ("mhs_cl", "Crude oil patterns", "CL841206-1"),
        "mhs_sf830520_1" => ("mhs_sf", "Swiss franc patterns", "SF830520-1"),
        "mhs_sf831011_3" => ("mhs_sf", "Swiss franc patterns", "SF831011-3"),
        "mhs_sf830519_1" => ("mhs_sf", "Swiss franc patterns", "SF830519-1"),
        "mhs_sp830825_1" => ("mhs_sp", "S&P 500 patterns", "SP830825-1"),
        "mhs_sp830120_1" => ("mhs_sp", "S&P 500 patterns", "SP830120-1"),
        "mhs_sp830330_1" => ("mhs_sp", "S&P 500 patterns", "SP830330-1"),
        "mhs_si830520_1" => ("mhs_si", "Silver patterns", "SI830520-1"),
        "mhs_si830630_1" => ("mhs_si", "Silver patterns", "SI830630-1"),
        "mhs_hd_rt_long" => ("mhs_tb", "T-Bond patterns", "H&D RT LONG"),
        "mhs_sp830128_2" => ("mhs_inter", "Inter-market patterns", "SP830128-2"),
        "mhs_sp830107_2" => ("mhs_inter", "Inter-market patterns", "SP830107-2"),
        "mhs_sp830825_2" => ("mhs_inter", "Inter-market patterns", "SP830825-2"),
        "mhs_sp830405_2" => ("mhs_inter", "Inter-market patterns", "SP830405-2"),
        "mhs_us881011_2" => ("mhs_inter", "Inter-market patterns", "US881011-2"),
        "mhs_sf850419_2" => ("mhs_inter", "Inter-market patterns", "SF850419-2"),
        "mhs_p_indicator" => ("mhs_other", "Not testable", "p-Indicator (Ch.13)"),
        "mhs_daily_report" => ("mhs_other", "Not testable", "Daily-report candidate patterns (Fig 7-7, 10-2)"),
        "mhs_pattern_model" => ("mhs_other", "Not testable", "Multi-pattern trading model / trend following (Ch.7, Ch.12)"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "mhs_ua830406_1" => vec![p("target","Profit target (points)",1.0,0.25,5.0,0.25), p("stop","Protective stop (points)",1.0,0.25,5.0,0.25)],
        "mhs_ua830407_1" => vec![p("target","Profit target (points)",1.0,0.25,5.0,0.25), p("stop","Protective stop (points)",1.0,0.25,5.0,0.25)],
        "mhs_ua830826_1" => vec![p("target","Profit target (points)",1.0,0.25,5.0,0.25), p("stop","Protective stop (points)",1.0,0.25,5.0,0.25)],
        "mhs_ua830131_1" => vec![p("target","Profit target (points)",1.0,0.25,5.0,0.25), p("stop","Protective stop (points)",1.0,0.25,5.0,0.25)],
        "mhs_ua830304_1" => vec![p("target","Profit target (points)",2.0,0.5,10.0,0.5), p("stop","Protective stop (points)",2.0,0.5,10.0,0.5)],
        "mhs_ua830125_1" => vec![p("target","Profit target (points)",0.5,0.125,2.5,0.125), p("stop","Protective stop (points)",0.5,0.125,2.5,0.125)],
        "mhs_ua830418_1" => vec![p("target","Profit target (points)",0.5,0.125,2.5,0.125), p("stop","Protective stop (points)",0.5,0.125,2.5,0.125)],
        "mhs_us800310_1" => vec![p("target","Profit target (points)",1.0,0.25,5.0,0.25), p("stop","Protective stop (points)",1.0,0.25,5.0,0.25)],
        "mhs_us801701_1" => vec![p("target","Profit target (points)",1.0,0.25,5.0,0.25), p("stop","Protective stop (points)",1.0,0.25,5.0,0.25)],
        "mhs_us810220_1" => vec![p("target","Profit target (points)",1.0,0.25,5.0,0.25), p("stop","Protective stop (points)",1.0,0.25,5.0,0.25)],
        "mhs_us800111_1" => vec![p("target","Profit target (points)",2.0,0.5,10.0,0.5), p("stop","Protective stop (points)",2.0,0.5,10.0,0.5)],
        "mhs_cl840221_1" => vec![p("target","Profit target (points)",1.0,0.25,5.0,0.25), p("stop","Protective stop (points)",1.0,0.25,5.0,0.25)],
        "mhs_cl850210_1" => vec![p("target","Profit target (points)",1.0,0.25,5.0,0.25), p("stop","Protective stop (points)",1.0,0.25,5.0,0.25)],
        "mhs_cl850919_1" => vec![p("target","Profit target (points)",1.0,0.25,5.0,0.25), p("stop","Protective stop (points)",1.0,0.25,5.0,0.25)],
        "mhs_cl841206_1" => vec![p("target","Profit target (points)",1.0,0.25,5.0,0.25), p("stop","Protective stop (points)",1.0,0.25,5.0,0.25)],
        "mhs_sf830520_1" => vec![p("target","Profit target (points)",1.0,0.25,5.0,0.25), p("stop","Protective stop (points)",1.0,0.25,5.0,0.25)],
        "mhs_sf831011_3" => vec![p("target","Profit target (points)",1.0,0.25,5.0,0.25), p("stop","Protective stop (points)",1.0,0.25,5.0,0.25)],
        "mhs_sf830519_1" => vec![p("target","Profit target (points)",1.0,0.25,5.0,0.25), p("stop","Protective stop (points)",1.0,0.25,5.0,0.25)],
        "mhs_sp830825_1" => vec![p("target","Profit target (points)",10.0,2.5,50.0,2.5), p("stop","Protective stop (points)",10.0,2.5,50.0,2.5), p("pct_target","[Not from the book] target as % of entry (0 = book points)",0.0,0.0,20.0,0.5), p("pct_stop","[Not from the book] stop as % of entry (0 = book points)",0.0,0.0,20.0,0.5)],
        "mhs_sp830120_1" => vec![p("target","Profit target (points)",10.0,2.5,50.0,2.5), p("stop","Protective stop (points)",10.0,2.5,50.0,2.5), p("pct_target","[Not from the book] target as % of entry (0 = book points)",0.0,0.0,20.0,0.5), p("pct_stop","[Not from the book] stop as % of entry (0 = book points)",0.0,0.0,20.0,0.5)],
        "mhs_sp830330_1" => vec![p("target","Profit target (points)",10.0,2.5,50.0,2.5), p("stop","Protective stop (points)",10.0,2.5,50.0,2.5), p("pct_target","[Not from the book] target as % of entry (0 = book points)",0.0,0.0,20.0,0.5), p("pct_stop","[Not from the book] stop as % of entry (0 = book points)",0.0,0.0,20.0,0.5)],
        "mhs_si830520_1" => vec![p("target","Profit target (points)",40.0,10.0,200.0,10.0), p("stop","Protective stop (points)",40.0,10.0,200.0,10.0)],
        "mhs_si830630_1" => vec![p("target","Profit target (points)",20.0,5.0,100.0,5.0), p("stop","Protective stop (points)",20.0,5.0,100.0,5.0)],
        "mhs_hd_rt_long" => vec![p("target","Profit target (points)",1.0,0.25,5.0,0.25), p("stop","Protective stop (points)",1.0,0.25,5.0,0.25)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if !checked(p, &schema) || from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let (logic, dir, at_open) = match id {
        "mhs_ua830406_1" => ("L1>H2&H1>L0&H3>L2&L2>L3&L0>L1&H0>H1&H3>L2", 1.0, false),
        "mhs_ua830407_1" => ("C0>C3&C1>C2&C3>C4&C2>C0", 1.0, true),
        "mhs_ua830826_1" => ("H4>H5&H3>L4&L5>H3&L4>L3&H5>L5", 1.0, false),
        "mhs_ua830131_1" => ("C3>C0&C1>C3&C2>C1&C4>C2", -1.0, false),
        "mhs_ua830304_1" => ("C2>C4&C1>C3&C4>C1", 1.0, false),
        "mhs_ua830125_1" => ("H1>L0&H0>L2&H2>H0&L0>L1&L2>H1", 1.0, false),
        "mhs_ua830418_1" => ("H3>L2&H2>L0&L1>H3&H0>H1&L2>L3&L0>L1&H1>H2", -1.0, false),
        "mhs_us800310_1" => ("C0>O2&O0>C1&C2>O1&C1>C2&O2>O0", 1.0, false),
        "mhs_us801701_1" => ("C3>C0&C2>C1&C4>C2&C1>C3", -1.0, false),
        "mhs_us810220_1" => ("C1>H2&C2>L1&H0>C0&H2>L0&H1>C1&L0>C2&C0>H1&L1>L2", -1.0, true),
        "mhs_us800111_1" => ("C2>C0&C3>C2&C1>C3", -1.0, false),
        "mhs_cl840221_1" => ("L1>H2&H1>L0&H3>L2&L2>L3&L0>L1&H0>H1&H3>L2", 1.0, false),
        "mhs_cl850210_1" => ("C2>C1&C0>C3&C1>C4&C3>C2", 1.0, false),
        "mhs_cl850919_1" => ("H1>H2&L0>L1&H0>C0&H2>O0&O0>L0&L0>L1&L1>L2", 1.0, false),
        "mhs_cl841206_1" => ("H2>C2&L0>H2&H0>H1&C0>L0&H1>C1&C2>L1&L1>L2&C1>C0", 1.0, true),
        "mhs_sf830520_1" => ("C1>C0&C2>O1&O0>C1&O2>C2&O1>O0", -1.0, false),
        "mhs_sf831011_3" => ("C3>C2&C2>C1&C4>C0&C0>C3|C3>C2&C2>C1&C4>C0&C0>C2&C3>C4", -1.0, true),
        "mhs_sf830519_1" => ("C7>C3&C6>C5&C4>C7&C5>C4", 1.0, false),
        "mhs_sp830825_1" => ("H0>L1&L2>H0&H1>L2&H2>L3&L1>L0&L3>H1&H3>H2", 1.0, true),
        "mhs_sp830120_1" => ("H0>C0&L2>L0&H1>H0&H2>H1&L0>L1&C0>L2", 1.0, true),
        "mhs_sp830330_1" => ("H0>C0&L2>L1&H1>H2&H2>L0&C0>H1&L0>L2", 1.0, true),
        "mhs_si830520_1" => ("H0>C1&L2>H0&H1>C2&C2>L2&L1>C0&C0>L0&H2>H1&C1>L1", -1.0, true),
        "mhs_si830630_1" => ("C4>C3&C6>C4&C7>C6&C3>C5", -1.0, true),
        "mhs_hd_rt_long" => ("L2>C1&L0>L1&C0>O0&C0>H2", 1.0, false),
        _ => return None,
    };
    let (pt, ps) = (g("pct_target"), g("pct_stop"));
    let (target, stop, pct) = if pt > 0. && ps > 0. { (pt, ps, true) } else { (g("target"), g("stop"), false) };
    // open entries: GTC target/stop placed with the order (p.63) -> active on the entry bar
    Some(simulate(x, from, to, logic, dir, at_open, true, level_fn(dir, target, stop, pct)))
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
            levels: impl Fn(f64) -> (f64, f64)) -> Vec<Trade> {
    let dnf = parse(logic);
    let mut out = vec![];
    let mut cur: Option<(Trade, f64, f64, usize)> = None;
    for i in from..=to {
        if let Some((t, tp, sp, active)) = cur.as_mut() {
            if i >= *active {
                if let Some((p, why)) = exit_on(x, i, dir, *tp, *sp) {
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
