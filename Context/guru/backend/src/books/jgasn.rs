//! Lane K81 (13 Nov 2026; authors/titles-1030/K81.tsv): Jack Gillen, "Astro Stats for the New York Stock Exchange" (Real-Time Market Forecast, (c) 2002).
//! Book id "jgasn". Source copy: research share 'USB HDD 2/tRADING KNO-2/Jack Gillen - Astro Stats for the New York Stock
//! Exchange/ASNYSE.pdf' (104 PDF pages, text layer; printed page = PDF page - 7). Read: contents, ch.1-7 (PDF p.5-48) in
//! full, ch.8-15 (stock selection) headings and rules. Reader row: authors/o1/O1.tsv (RULES-COMPLETE).
//! Every coded rule is a close-to-close trade on the Dow: enter at the close of a start date, exit at the close of an end
//! date, the dates fixed by the calendar (ch.1, ch.7 Jupiter-line text) or printed in the book's tables (ch.2-7, to 2050).
//! The tables are below verbatim (text layer of the PDF, column order kept); the readings applied to them are stated once
//! here and on every card:
//!  R1 a date that is not a trading day uses the close of the last trading day before it, at most 5 calendar days back
//!     (decision #302 (3): a longer gap, e.g. in a stock's file, is no trade). Table 1.1 prints exactly those
//!     closes (17 May 1981, a Sunday: 985.95 = Friday 15 May; 17 Nov 1984, a Saturday: 1187.94 = Friday 16 Nov).
//!  R2 a start and end on the same close is no trade; one position at a time (a window starting before the last exit is
//!     skipped). A date after the last bar is not known yet: the trade stays open.
//!  R3 year-row tables keep their printed order; an entry that is not a real date (2003 Moon '11/18-31', 2024 '7/410-24')
//!     or starts before the end of the entry kept before it (misprints such as 2003 '1/21-11/3', 2016 '1/26-11/9' and 2018
//!     '1/6-20' among the October rows, the repeated 2013 '1/2-15', 2018 '2/2-16') is skipped, never repaired by guesswork.
//!     Full-date tables (m/d/yy) are sorted by start date and then use the same overlap rule.
//!  R4 the 2003 Moon row '1/12-2/3' is read '1/21-2/3': the text quotes this row as 'January 21st - February 3rd' (p.4).
//!  R5 Table 6.1 '6/20.82' and '8/21.87' (a full stop for the slash) and '10/30/9' (= 10/30/93: the printed 3,680.59 is the
//!     DJIA close of Friday 29 Oct 1993) are read so. Two-digit years: 70-99 = 19xx, 00-69 = 20xx.
//!  R6 Jupiter/Saturn aspects (p.37): two predictions per aspect (16 days before -> the aspect date, the aspect date -> 16
//!     days after), up for sextile/square, down for trine/opposition; Tables 7.3/7.4 print only the aspect date, so 16B/16A =
//!     16 calendar days before/after (their printed 16B/16A closes are those dates' closes).
use crate::books::atm03::{civil, day_num};
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "jgasn";
/// The ten coded cards (close-to-close calendar trades).
pub const REVIEWED: &[&str] = &["jgasn_sun_jul", "jgasn_sun_dec", "jgasn_sun_opp", "jgasn_moon", "jgasn_mercury", "jgasn_jupiter_line",
    "jgasn_saturn_line", "jgasn_saturn_retro", "jgasn_jupiter_retro", "jgasn_js_aspects"];
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && REVIEWED.contains(&def.id),
    label: "Gillen's astro-date rules are stated for the Dow Jones Industrial Average (DJIA): buy or sell at the close of one date and close out at the close of a later date. They run on the DJIA itself and, as a declared application, on the other US and Indian indices, ETFs and single stocks with the same dates; only closing prices are used.",
    basis: "Jack Gillen, Astro Stats for the New York Stock Exchange (2002) ch.1-7, PDF p.8-48 (Tables 1.1-7.6); readings R1-R6 in books/jgasn.rs; independent check authors/titles-1030/K81w/k81_ref.py (parses the book's text layer itself).",
};

/// Tables 2.1-2.10 (PDF p.11-14), Virgo to Pisces Moon Cycle 2003-2050: each year's entries in printed order, verbatim.
const MOON: &str = "\
2003 1/12-2/3 2/17-3/2 3/17-30 4/13-26 5/11-23 6/7-19 7/4-17 8/1-13 8/27-9/9 9/24-10/7 1/21-11/3 11/18-31 12/15-28\n\
2004 1/12-24 2/7-20 3/6-19 4/2-15 4/30-5/13 5/27-6/8 6/23-7/6 7/21-8/3 8/17-30 9/13-26 10/11-24 11/7-20 12/4-17\n\
2005 1/1-14 1/28-2/10 2/14-3/11 3/24-4/5 4/20-5/3 5/17-30 6/14-27 7/11-24 8/7-20 9/3-17 9/30-10/14 10/28-11/11 11/24-12/8 12/22-1/4\n\
2006 1/18-31 2/14-28 3/13-27 4/10-24 5/7-21 6/3-17 7/1-14 7/27-8/10 8/24-9/7 9/21-10/15 10/18-31 11/14-28 12/11-25\n\
2007 1/8-22 2/4-18 3/3-18 3/30-4/14 4/28-5/11 5/24-6/8 6/21-7/15 7/18-8/1 8/14-29 9/11-25 10/8-22 11/4-19 12/1-16 12/29-1/12\n\
2008 1/25-2/9 2/22-3/6 3/20-4/3 4/16-30 5/13-28 6/10-24 7/6-21 8/3-17 9/1-14 9/27-10/12 10/24-11/8 11/21-12/5 12/18-31\n\
2009 1/14-29 2/10-25 3/10-24 4/6-21 5/4-18 5/30-6/14 6/27-7/12 7/25-8/8 8/21-9/4 9/17-30 10/14-28 11/11-25 12/8-22\n\
2010 1/4-18 2/1-15 2/28-3/14 3/27-4/10 4/24-5/8 5/21-6/4 6/17-7/1 7/15-29 8/11-25 9/8-21 10/5-18 11/2-15 11/29-12/12 12/27-1/9\n\
2011 1/22-2/5 2/19-3/4 3/18-31 4/15-28 5/12-25 6/8-21 7/6-19 8/2-15 8/25-9/11 9/26-10/9 10/23-11/5 11/29-12/2 12/17-29\n\
2012 1/2-15 2/9-22 3/8-21 4/4-17 5/1-14 5/29-6/10 6/24-7/8 7/22-8/4 8/9-22 9/15-28 10/12-25 11/9-21 12/6-19\n\
2013 1/2-15 1/2-15 1/29-2/11 3/25-4/7 4/22-5/4 5/29-6/1 6/15-28 7/12-25 8/9-22 9/4-18 10/2-15 10/29-11/12 11/26-12/9 12/23-1/5\n\
2014 1/20-2/2 2/16-3/1 3/15-29 4/11-24 5/9-22 6/5-18 7/2-15 7/29-8/12 8/25-9/8 9/22-10/6 10/19-11/1 11/16-29 12/13-27\n\
2015 1/9-23 2/5-19 3/4-18 4/1-14 4/28-5/13 5/25-6/9 6/22-7/6 7/19-8/2 8/15-29 9/12-26 10/9-23 11/6-20 12/3-17 12/30-1/15\n\
2016 1/26-2/9 2/22-3/8 3/21-4/5 4/17-5/2 5/15-29 6/11-26 7/8-23 8/5-19 9/1-15 9/28-10/13 1/26-11/9 11/27-12/7 12/19-1/2\n\
2017 1/16-30 2/12-26 3/11-26 4/8-22 5/5-20 6/1-16 6/29-7/13 7/26-8/9 8/22-9/6 9/18-10/3 10/15-30 11/12-26 12/9-24\n\
2018 1/23-2/6 2/2-16 3/1-15 3/29-4/12 4/25-5/10 5/22-6/5 6/19-7/3 7/16-30 8/13-27 9/9-23 1/6-20 11/2-16 11/29-12/14 12/27-1/10\n\
2019 1/23-2/6 2/20-3/5 3/19-4/2 4/15-29 5/13-26 6/9-23 7/7-20 8/3-17 8/30-9/13 9/26-10/10 10/24-11/6 11/21-12/3 12/18-31\n\
2020 1/14-27 2/10-23 3/9-22 4/5-18 5/2-15 5/29-6/11 6/26-7/9 7/23-8/6 8/20-9/2 9/16-29 10/13-26 11/10-22 12/7-20\n\
2021 1/2-16 1/31-2/13 2/27-3/12 3/26-4/8 4/23-5/5 5/20-6/1 6/16-29 7/13-26 8/10-23 9/6-19 10/3-16 11/1-13 11/27-12/10 12/24-1/6\n\
2022 1/22-2/2 2/17-3/2 3/16-29 4/13-25 5/10-23 6/6-19 7/4-17 7/31-8/13 8/27-9/9 9/23-10/7 10/21-11/3 11/17-30 12/14-27\n\
2023 1/11-24 2/7-20 3/6-20 4/3-16 4/30-5/14 5/28-6/10 6/24-7/7 7/21-8/3 8/17-31 9/13-27 10/11-24 11/7-21 12/5-18\n\
2024 1/1-14 1/28-2/11 2/24-3/10 3/22-4/5 4/19-5/3 5/16-31 6/13-27 7/410-24 8/6-20 9/2-17 9/30-10/14 10/27-11/11 11/23-12/8 12/21-1/4\n\
2025 1/17-31 2/14-28 3/12-27 4/9-24 5/6-21 6/2-17 6/29-7/14 7/27-8/11 8/24-9/7 9/20-10/4 10/17-31 11/13-28 12/11-26\n\
2026 1/7-21 2/3-18 3/3-17 3/30-4/13 4/27-5/11 5/24-6/8 6/20-7/5 7/18-8/1 8/14-28 9/10-25 10/8-22 11/4-18 12/1-16 12/28-1/12\n\
2027 1/25-2/8 2/21-3/7 3/20-4/3 4/17-5/1 5/14-28 6/11-24 7/8-22 8/4-18 9/1-15 9/28-10/11 10/25-11/8 11/22-12/5 12/18-1/2\n\
2028 1/15-29 2/12-25 3/10-24 4/7-20 5/3-17 5/31-6/13 6/27-7/10 7/25-8/7 8/21-9/3 9/18-10/1 10/15-28 11/11-24 12/8-22\n\
2029 1/5-18 2/1-14 3/1-14 3/28-4/10 4/24-5/17 5/22-6/3 6/18-7/1 7/15-28 8/11-24 9/8-21 10/5-18 11/2-14 11/29-12/12 12/26-1/8\n\
2030 1/22-2/4 2/19-3/4 3/18-31 4/15-27 5/12-24 6/8-21 7/5-18 8/2-15 8/29-9/11 9/25-10/8 10/23-11/4 11/19-12/12 12/16-29\n\
2031 1/13-26 2/9-22 3/8-21 4/4-17 5/2-15 5/29-6/11 6/26-7/8 7/23-8/5 8/19-9/1 9/15-29 10/12-25 11/9-22 12/6-19\n\
2032 1/3-16 1/30-2/12 2/26-3/11 3/24-4/7 4/21-5/4 5/18-6/1 6/14-28 7/12-25 8/8-22 9/4-18 10/2-16 10/29-11/12 11/25-12/9 12/22-1/5\n\
2033 1/19-2/2 2/15-3/1 3/14-29 4/11-25 5/8-22 6/4-18 7/2-16 7/29-8/12 8/25-9/8 9/21-10/6 10/19-11/2 11/15-30 12/12-26\n\
2034 1/9-23 2/5-19 3/4-19 4/1-15 4/28-5/13 5/25-6/9 6/22-7/6 7/19-8/2 8/15-30 9/12-26 10/9-24 11/5-20 12/2-17 12/30-1/13\n\
2035 1/26-2/10 2/23-3/9 3/22-4/5 4/18-5/2 5/15-30 6/12-25 7/9-24 8/6-20 9/1-16 9/29-10/14 10/27-11/9 11/22-12/7 12/30-1/3\n\
2036 1/16-30 2/13-27 3/11-25 4/7-21 5/4-19 6/1-15 6/28-7/12 7/26-8/8 8/22-9/5 9/19-10/2 10/16-29 11/12-26 12/10-23\n\
2037 1/6-20 2/2-15 3/2-15 3/29-4/11 4/26-5/9 5/23-6/5 6/18-7/2 7/16-30 8/13-26 9/9-22 10/7-20 11/3-16 11/30-12/13 12/27-1/10\n\
2038 1/24-2/6 2/20-3/5 3/20-4/1 4/16-29 5/13-26 6/10-22 7/7-20 8/3-16 8/30-9/12 9/27-10/10 10/24-11/6 11/21-12/3 12/18-31\n\
2039 1/14-27 2/10-23 3/1-23 4/6-19 5/3-16 5/31-6/12 6/27-7/10 7/24-8/6 8/12-9/3 9/17-30 10/14-27 11/11-23 12/8-21\n\
2040 1/4-17 2/1-14 2/28-3/12 3/26-4/8 4/23-5/5 5/19-6/1 6/16-29 7/13-26 8/10-23 9/6-19 10/3-17 10/31-11/13 11/27-12/10 12/24-1/6\n\
2041 1/21-2/3 2/17-3/2 3/16-29 4/12-26 5/10-23 6/6-20 7/3-17 7/31-8/13 8/27-9/10 9/23-10/7 10/20-11/3 11/17-30 12/14-28\n\
2042 1/10-24 2/7-21 3/6-20 4/2-17 4/29-5/14 5/27-6/10 6/23-7/7 7/20-8/4 8/16-31 9/13-28 10/10-25 11/7-21 12/4-18 12/31-1/15\n\
2043 1/28-2/11 2/24-3/10 3/23-4/7 4/20-5/4 5/17-6/1 6/13-28 7/10-25 8/7-21 9/3-17 9/30-10/15 10/7-11/11 11/24-12/9 12/21-1/5\n\
2044 1/18-2/1 2/14-29 3/13-27 4/29-23 5/6-21 6/2-17 6/30-7/14 7/27-8/10 8/24-9/7 9/20-10/4 10/17-31 11/13-28 12/11-25\n\
2045 1/7-21 2/3-18 3/3-17 3/30-4/13 4/27-5/11 5/24-6/11 6/20-7/4 7/17-31 8/14-28 9/10-24 10/8-21 11/4-18 12/1-15 12/28-1/11\n\
2046 1/25-2/8 2/21-3/6 3/21-4/3 4/17-30 5/14-27 6/10-24 7/8-21 8/4-18 9/1-13 9/28-10/11 10/26-11/7 11/22-12/5 12/19-1/1\n\
2047 1/15-28 2/12-25 3/11-24 4/8-20 5/5-18 6/1-14 6/28-7/11 7/26-8/8 8/22-9/4 9/18-10/1 10/16-28 11/12-25 12/9-22\n\
2048 1/6-18 2/2-15 2/29-3/11 3/28-4/9 4/24-5/7 5/21-6/2 6/17-30 7/15-28 8/11-24 9/8-21 10/5-18 11/1-14 11/28-12/11 12/25-1/8\n\
2049 1/22-2/4 2/18-3/4 3/18-30 4/14-27 5/11-24 6/8-21 7/5-18 8/1-14 8/29-9/11 9/25-10/8 10/22-11/4 11/18-12/2 12/16-29\n\
2050 1/12-25 2/18-22 3/8-21 4/4-18 5/1-14 5/29-6/11 6/25-7/8 7/22-8/5 8/18-9/1 9/15-29 10/12-26 11/8-22 12/6-20\n\
";
/// Table 3.1 (PDF p.16), Mercury Retrograde 2003-2050, verbatim.
const MERCURY: &str = "\
2003 1/2-22 4/26-5/19 8/28-9/19 12/17-1/5\n\
2004 4/6-29 8/10-9/1 12/2-19\n\
2005 3/20-4/11 7/23-8/15 11/14-12/3\n\
2006 3/2-24 7/4-28 10/28-11/17\n\
2007 2/14-3/7 6/15-7/9 10/12-11/1\n\
2008 1/28-2/18 5/26-6/18 9/24-10/14\n\
2009 1/11-2/1 5/7-30 9/7-28 12/26-1/14\n\
2010 4/18-5/10 8/20-9/11 12/10-29\n\
2011 3/30-4/22 8/3-25 11/24-12/13\n\
2012 3/12-4/3 7/15-8/8 11/6-25\n\
2013 2/23-3/16 6/26-7/19 10/21-11/9\n\
2014 2/6-27 6/7-7/1 10/4-24\n\
2015 1/21-2/10 5/18-6/10 9/17-10/8\n\
2016 1/5-24 4/28-5/21 8/30-9/21 12/19-1/17\n\
2017 4/9-5/3 8/13-9/5 12/3-22\n\
2018 3/23-4/14 7/26-8/18 11/17-12/5\n\
2019 3/5-27 7/7-8/1 11/2-19\n\
2020 2/17-3/9 6/18-7/11 10/14-11/2\n\
2021 1/30-2/20 5/29-6/21 9/27-10/17\n\
2022 1/14-2/3 5/10-6/3 9/10-10/1 12/29-1/16\n\
2023 4/21-5/14 8/23-9/14 12/13-1/1\n\
2024 4/2-24 8/5-27 11/26-12/14\n\
2025 3/15-4/6 7/18-8/10 11/9-28\n\
2026 2/26-3/19 6/29-7/22 10/24-11/12\n\
2027 2/9-3/2 6/10-7/3 10/7-27\n\
2028 1/24-2/13 5/21-6/13 9/19-10/10\n\
2029 1/7-26 5/2-24 9/2-24 12/22-1/10\n\
2030 4/13-5/5 8/16-9/7 12/6-24\n\
2031 3/26-4/17 7/29-8/21 11/19-12/8\n\
2032 3/7-29 7/10-8/2 11/2-21\n\
2033 2/18-3/12 6/21-7/14 10/25-11/5\n\
2034 2/2-22 6/2-25 9/30-10/20\n\
2035 1/17-2/6 5/13-6/5 9/13-10/4\n\
2036 1/2-20 4/23-5/16 8/25-9/16 12/15-1/2\n\
2037 4/5-27 8/8-30 11/28-12/17\n\
2038 3/18-4/9 7/21-8/13 11/12-12/1\n\
2039 3/2-22 7/2-27 10/27-11/15\n\
2040 2/12-3/4 6/13-7/6 10/9-29\n\
2041 1/26-2/15 5/24-6/16 9/22-10/13\n\
2042 1/10-29 5/5-28 9/5-25 12/24-1/13\n\
2043 4/15-5/9 8/19-9/10 12/7-27\n\
2044 3/28-4/18 8/2-22 11/21-12/10\n\
2045 3/9-4/1 3/13-8/5 11/5-23\n\
2046 2/21-3/15 6/24-7/16 10/19-11/7\n\
2047 2/5-25 6/5-28 10/3-24\n\
2048 1/20-2/8 5/16-6/7 9/15-10/6\n\
2049 1/2-22 4/26-5/20 8/28-9/19 12/17-1/5\n\
2050 4/8-30 8/11-9/2 12/2-20\n\
";
/// Table 6.3 (PDF p.41), Jupiter Line 2003-2050, verbatim.
const JUP_LINE: &str = "\
2003 4/4-24\n\
2004 None\n\
2005 6/5-11/21\n\
2006 6/27-7/14\n\
2007 8/7-12/18\n\
2008 10/29-12/31\n\
2009 10/13-12/31\n\
2010 None\n\
2011 1/1-25\n\
2012 6/24-10/3\n\
2013 7/8-11/6\n\
2014 None\n\
2015 8/2-12/31\n\
2016 None\n\
2017 1/1-2/5\n\
2018 9/6-12/31\n\
2019 9/18-12/31\n\
2020 1/1-5/10\n\
2021 10/18-12/31\n\
2022 11/23-12/31\n\
2023 1/1-4/20\n\
2024 1/1-4/20\n\
2025 2/4-7/12\n\
2026 None\n\
2027 12/23-31\n\
2028 6/25-8/20\n\
2029 None\n\
2030 1/19-3/12\n\
2031 2/2-4/13\n\
2032 2/16-5/19\n\
2033 3/2-4/22\n\
2034 3/15-4/16\n\
2035 3/30-9/9\n\
2036 4/12-7/17\n\
2037 4/27-9/4\n\
2038 5/11-12/17\n\
2039 7/3-12/31\n\
2040 9/19-12/31\n\
2041 6/12-12/31\n\
2042 1/1-2/13\n\
2043 1/1-2/25\n\
2044 9/22-12/31\n\
2045 1/1-12/31\n\
2046 12/4-31\n\
2047 1/13-4/13\n\
2048 1/10-4/23\n\
2049 2/14-5/6\n\
2050 10/6-12/20\n\
";
/// Table 6.4 (PDF p.42), The Saturn Line 2003-2050, verbatim.
const SAT_LINE: &str = "\
2003 5/2-6/28\n\
2004 5/5-11/6\n\
2005 None\n\
2006 9/8-11/2\n\
2007 None\n\
2008 9/7-10/27\n\
2009 None\n\
2010 5/29-7/22\n\
2011 6/12-8/29\n\
2012 1/1-2/6\n\
2013 None\n\
2014 7/20-12/7\n\
2015 None\n\
2016 8/13-12/31\n\
2017 8/24-12/31\n\
2018 1/1-3/8\n\
2019 1/1-4/7\n\
2020 9/29-12/31\n\
2021 1/1-5/22\n\
2022 1/1-6/3\n\
2023 4/21-6/16\n\
2024 4/21-6/28\n\
2025 None\n\
2026 3/11-7/24\n\
2027 4/13-8/8\n\
2028 5/13-6/24\n\
2029 6/13-9/5\n\
2030 7/14-9/19\n\
2031 8/16-10/4\n\
2032 9/17-10/18\n\
2033 10/23-12/31\n\
2034 4/17-8/2\n\
2035 None\n\
2036 7/18-10/14\n\
2037 9/5-10/14\n\
2038 None\n\
2039 5/25-7/2\n\
2040 6/6-9/18\n\
2041 1/1-2/1\n\
2042 7/19-12/31\n\
2043 8/20-12/31\n\
2044 1/1-3/8\n\
2045 None\n\
2046 1/1-4/2\n\
2047 None\n\
2048 9/12-10/18\n\
2049 9/24-11/19\n\
2050 3/20-5/18\n\
";
/// Tables 6.1 (PDF p.39, 1980-2002) and 6.2 (PDF p.40, 2003-2050), Saturn Retrograde start/end, verbatim (a half-row with one date, '2032 3/1/33', is left out).
const SAT_RETRO: &str = "\
1/9/80 5/24/80\n\
1/20/81 6/6/81\n\
2/2/82 6/20.82\n\
2/14/83 7/3/83\n\
2/26/84 7/15/84\n\
3/9/85 7/25/85\n\
3/21/86 8/9/86\n\
4/2/87 8/21.87\n\
4/12/88 9/2/88\n\
4/25/89 9/13/89\n\
5/6/90 9/25/90\n\
5/18/91 10/6/91\n\
5/30/92 10/19/92\n\
6/12/93 10/30/9\n\
6/25/94 11/10/94\n\
7/7/95 11/24/95\n\
6/26/96 12/5/96\n\
8/3/97 12/17/97\n\
8/7/98 12/31/98\n\
9/2/99 1/11/00\n\
9/29/00 1/24/01\n\
9/27/01 2/7/02\n\
10/11/02 2/21/03\n\
8/8/27 12/23/27\n\
10/25/03 3/5/04\n\
8/21/28 1/4/29\n\
11/8/04 3/21/05\n\
9/6/29 1/18/30\n\
11/22/05 4/4/06\n\
9/19/30 2/1/31\n\
12/5/06 4/18/07\n\
1/5/31 2/15/32\n\
12/19/07 5/3/08\n\
1/2/09 5/16/09\n\
11/2/33 3/15/34\n\
1/13/10 5/29/10\n\
11/16/34 3/29/35\n\
1/26/11 6/12/11\n\
11/30/35 4/11/36\n\
2/7/12 6/24/12\n\
12/13/36 4/26/37\n\
2/17/13 7/7/13\n\
12/26/37 5/11/38\n\
3/2/14 7/19/14\n\
3/14/15 8/1/15\n\
1/8/39 5/24/39\n\
3/25/16 8/12/16\n\
1/21/40 6/5/40\n\
4/6/17 8/24/17\n\
2/2/41 6/19/41\n\
4/17/18 9/5/18\n\
2/13/42 7/1/42\n\
5/2/19 9/17/19\n\
2/24/43 7/14/43\n\
5/11/20 9/28/20\n\
3/3/44 7/26/44\n\
5/23/21 10/10/21\n\
3/19/45 8/8/45\n\
6/4/22 10/22/22\n\
4/2/46 8/19/46\n\
6/17/23 11/3/23\n\
4/13/47 9/1/47\n\
6/28/24 11/14/24\n\
4/24/48 9/11/48\n\
7/2/25 11/27/25\n\
5/6/49 9/23/49\n\
7/25/26 12/9/26\n\
5/18/50 10/5/50\n\
";
/// Tables 7.1 (PDF p.45, 1979-2002) and 7.2 (PDF p.46, 2003-2050), Jupiter Retrograde start/finish, verbatim.
const JUP_RETRO: &str = "\
1/2/79 3/27/79\n\
12/28/79 4/27/80\n\
1/26/81 5/28/81\n\
2/25/82 6/28/82\n\
3/28/83 7/28/83\n\
5/2/84 8/31/84\n\
6/6/85 10/4/85\n\
7/14/86 11/10/86\n\
8/21/87 12/17/87\n\
9/26/88 1/21/89\n\
10/2/89 2/26/90\n\
12/2/90 4/2/91\n\
1/2/92 5/2/92\n\
1/30/93 6/2/93\n\
2/2/94 7/3/94\n\
4/3/95 8/4/95\n\
5/6/96 9/5/96\n\
6/11/97 10/9/97\n\
7/20/98 11/15/98\n\
8/26/99 12/22/99\n\
11/2/01 3/1/02\n\
9/29/00 1/25/01\n\
1/2/03 4/3/03\n\
1/5/04 5/4/04\n\
3/2/05 6/3/05\n\
3/3/06 7/5/06\n\
4/6/07 8/7/07\n\
5/9/08 9/8/08\n\
6/15/09 10/12/09\n\
7/23/10 11/17/10\n\
8/30/11 12/23/11\n\
10/4/12 1/29/13\n\
11/7/13 3/5/14\n\
12/8/14 4/7/15\n\
1/8/16 5/9/16\n\
2/6/17 6/18/17\n\
3/9/18 7/9/18\n\
4/10/19 8/9/19\n\
5/14/20 9/11/20\n\
6/21/21 10/15/21\n\
7/28/22 11/22/22\n\
9/2/23 12/29/23\n\
10/24/24 2/3/25\n\
11/11/25 3/10/26\n\
2/9/29 6/12/29\n\
3/13/30 7/12/30\n\
4/15/31 8/15/31\n\
5/19/32 9/16/32\n\
6/24/33 10/21/33\n\
8/3/34 11/28/34\n\
9/10/35 1/4/36\n\
10/14/36 2/6/37\n\
11/16/37 3/12/38\n\
12/17/38 4/15/39\n\
1/16/40 5/18/40\n\
2/14/41 6/17/41\n\
3/18/41 7/18/42\n\
4/20/43 8/19/43\n\
5/24/44 9/21/44\n\
7/3/45 10/27/45\n\
8/8/46 12/3/46\n\
9/13/47 1/9/48\n\
10/19/48 2/12/49\n\
1/2/50 3/19/50\n\
";
/// Tables 7.3/7.4 (PDF p.47): Jupiter/Saturn aspect dates 1984-1997 and the aspect (opp/trine = down, sextile/square = up), verbatim.
const ASPECT_DATES: &str = "\
9/9/89 opp\n\
11/14/89 opp\n\
7/14/90 opp\n\
5/24/87 trine\n\
11/19/87 trine\n\
3/13/88 trine\n\
9/9/93 trine\n\
8/27/94 trine\n\
5/5/84 sextile\n\
6/6/84 sextile\n\
1/21/85 sextile\n\
2/8/97 sextile\n\
7/13/97 sextile\n\
11/12/97 sextile\n\
5/11/91 square\n\
1/30/95 square\n\
3/3/95 square\n\
11/11/95 square\n\
";
/// Tables 7.5 (PDF p.47, sextile/square bull trend) and 7.6 (PDF p.48, trine/opposition bear trend) 2004-2050: 16B, aspect date, 16A, verbatim.
const ASPECT_TABLE: &str = "\
4/15/04 5/1/04 5/17/04 bull\n\
7/23/04 8/8/04 8/25/04 bull\n\
12/1/05 12/17/05 1/2/06 bull\n\
10/7/06 10/23/06 11/8/06 bull\n\
5/6/16 5/22/16 6/7/16 bull\n\
8/10/17 8/26/17 9/11/17 bull\n\
1/12/24 1/28/24 2/13/24 bull\n\
4/8/35 4/23/35 5/9/35 bull\n\
10/6/35 10/22/35 11/7/35 bull\n\
1/30/36 2/15/36 3/2/36 bull\n\
4/19/37 5/5/37 5/21/37 bull\n\
9/17/44 10/3/44 10/19/44 bull\n\
10/29/44 11/14/44 11/30/44 bull\n\
2/14/46 3/2/46 3/18/46 bull\n\
4/11/07 4/27/07 5/13/07 bear\n\
1/3/08 1/19/08 2/4/08 bear\n\
8/20/08 9/5/08 9/21/08 bear\n\
10/29/08 11/14/08 11/31/08 bear\n\
7/28/10 8/13/10 8/29/10 bear\n\
11/22/13 12/8/13 12/24/13 bear\n\
8/13/26 8/29/26 9/14/26 bear\n\
6/20/27 7/6/27 7/22/27 bear\n\
12/2/29 12/18/29 1/3/30 bear\n\
11/4/30 11/20/30 12/6/30 bear\n\
7/12/31 7/28/31 8/13/31 bear\n\
9/13/31 9/29/31 10/15/31 bear\n\
4/17/33 5/3/33 5/19/33 bear\n\
2/1/34 2/17/34 3/5/34 bear\n\
10/11/34 10/27/34 11/12/34 bear\n\
11/21/34 12/7/34 12/23/34 bear\n\
6/23/46 7/9/46 7/28/46 bear\n\
4/15/47 5/1/47 5/17/47 bear\n\
6/13/50 6/29/50 7/15/50 bear\n\
";

const SRC: &str = "Gillen, Astro Stats for the New York Stock Exchange (2002); printed pages (PDF page = printed + 7). ";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("jgasn_sun_jul", "Sun at 13-22 Cancer: long DJIA from the 7 July close to the 15 July close", "Ch.1 · Sun-stats", "Long", Status::Live, &[
            "E|\"the transit dates would be from July 7 - 15 each year. The price of the Dow Jones Industrial Average will be higher on the 15th then on the 7th\" (p.1)",
            "X|close of 15 July (p.1)"],
          concat!("Gillen, Astro Stats for the New York Stock Exchange (2002); printed pages (PDF page = printed + 7). ", "Every year: buy at the close of 7 July, sell at the close of 15 July. ",
            "Close-to-close on the DJIA in the book; here on every index, ETF and stock as a declared application with the same dates. ",
            "Dates that are not trading days use the close of the last trading day before them, at most 5 calendar days back (R1: Table 1.1 prints exactly those closes); a window that begins and ends on the same close is no trade, and one position at a time (R2). "),
          claim("15", "86.6%", "", "8 days", "1987-2001, DJIA (p.1; 72.8% overall)")),
        d("jgasn_sun_dec", "Venus opposition: long DJIA from the 24 December close to the 31 December close", "Ch.1 · Sun-stats", "Long", Status::Live, &[
            "E|\"dominant from December 24th - 31st. This is a long position, therefore the DJIA will be higher on the 31st than on the 24th\" (p.1)",
            "X|\"going long on December 24th ... on December 31st the market will be higher at the close\" (p.1)"],
          concat!("Gillen, Astro Stats for the New York Stock Exchange (2002); printed pages (PDF page = printed + 7). ", "Every year: buy at the close of 24 December, sell at the close of 31 December. ",
            "Close-to-close on the DJIA in the book; here on every index, ETF and stock as a declared application with the same dates. ",
            "Dates that are not trading days use the close of the last trading day before them, at most 5 calendar days back (R1: Table 1.1 prints exactly those closes); a window that begins and ends on the same close is no trade, and one position at a time (R2). "),
          claim("101", "80% (wrong 20 times)", "", "7 days", "1900-2000, DJIA (p.1; the losing years are listed)")),
        d("jgasn_sun_opp", "Opposition of the NYSE Sun: long DJIA from the 17 November close to the 17 May close", "Ch.1 · Sun-stats", "Long", Status::Live, &[
            "E|\"if you are trading on the long side at the closing price of November 17th to the closing price of May 17th, chances are that you are going to be correct\" (p.2)",
            "X|close of 17 May (p.2, Table 1.1)",
            "E|option short=1: \"If you are trading on the short side, you might want to trade from the date of the NYSE on May 17th, and short to the closing of November 17th\" (p.2)"],
          concat!("Gillen, Astro Stats for the New York Stock Exchange (2002); printed pages (PDF page = printed + 7). ", "Every year: buy at the close of 17 November, sell at the close of 17 May. Option p.short = 1 adds the book's short side, 17 May to 17 November (then always in the market). ",
            "Close-to-close on the DJIA in the book; here on every index, ETF and stock as a declared application with the same dates. ",
            "Dates that are not trading days use the close of the last trading day before them, at most 5 calendar days back (R1: Table 1.1 prints exactly those closes); a window that begins and ends on the same close is no trade, and one position at a time (R2). "),
          claim("22", "81.08% (4 of 22 years down)", "", "6 months", "Nov 1980 - May 2002, DJIA (Table 1.1, p.2)")),
        d("jgasn_moon", "Moon from Virgo to Pisces: long DJIA over each printed window (Tables 2.1-2.10)", "Ch.2 · Moon-stats", "Long", Status::Live, &[
            "E|\"the moon's transit from Virgo to Pisces. Therefore, if you are looking to go long ... at the closing price of January 21st would represent the moon in Virgo ... by going long during this period it is indicating to you there is a 70 percent chance that on February 3rd the Dow Jones Industrial Average will be higher\" (p.4)",
            "X|close of the window's end date (Tables 2.1-2.10, p.4-7, 2003-2050)",
            "E|option short=1: \"if you have a stock you want to short, your best chance would be from the sign of Pisces to Virgo ... your exit date going long ... would be the starting date for going short\" (p.4)"],
          concat!("Gillen, Astro Stats for the New York Stock Exchange (2002); printed pages (PDF page = printed + 7). ", "Each window of Tables 2.1-2.10 (2003-2050): buy at the close of its first date, sell at the close of its last date. Option p.short = 1 also sells short from each window's end to the next window's start. ",
            "Table misprints (R3, R4): 2003 '1/12-2/3' is read '1/21-2/3' as the text quotes it (p.4); entries that are not real dates or that start before the previous entry ends (2003 '1/21-11/3', '11/18-31'; 2013 the repeated '1/2-15'; 2016 '1/26-11/9'; 2018 '2/2-16', '1/6-20'; 2024 '7/410-24') are skipped. Kept as printed although it looks misprinted: 2011 '11/29-12/2' (3 days; the other windows run 12-15 days). ",
            "Close-to-close on the DJIA in the book; here on every index, ETF and stock as a declared application with the same dates. ",
            "Dates that are not trading days use the close of the last trading day before them, at most 5 calendar days back (R1: Table 1.1 prints exactly those closes); a window that begins and ends on the same close is no trade, and one position at a time (R2). "),
          claim("", "about 70%", "", "about 2 weeks", "no period stated (p.4)")),
        d("jgasn_mercury", "Mercury retrograde: long DJIA from the retrograde date to the direct date (Table 3.1)", "Ch.3 · Mercury-stats", "Long", Status::Live, &[
            "E|\"in those years when Mercury is retrograding there is an 85 percent chance that the DJIA price at the time it goes direct will be higher than when it was retrograding\" (p.9)",
            "X|close of the direct date (Table 3.1, p.9, 2003-2050)"],
          concat!("Gillen, Astro Stats for the New York Stock Exchange (2002); printed pages (PDF page = printed + 7). ", "Each period of Table 3.1 (2003-2050): buy at the close of the retrograde date, sell at the close of the direct date. The 'dates in bold print' (70-80%, p.9) cannot be recovered: the source copy's text layer carries no bold marking (decision #297), so every period is traded. ",
            "Close-to-close on the DJIA in the book; here on every index, ETF and stock as a declared application with the same dates. ",
            "Dates that are not trading days use the close of the last trading day before them, at most 5 calendar days back (R1: Table 1.1 prints exactly those closes); a window that begins and ends on the same close is no trade, and one position at a time (R2). "),
          claim("", "85%", "", "about 3 weeks", "no period stated (p.9)")),
        d("jgasn_jupiter_line", "Jupiter-line: long DJIA over each printed Jupiter-line period (Table 6.3)", "Ch.6-7 · Jupiter-line", "Long", Status::Live, &[
            "E|\"a Jupiter-line is always positive with the exit date being higher than the entry date\" (p.37)",
            "X|close of the period's end date (Table 6.3, p.34, 2003-2050)",
            "E|option decade=1: \"every ten years from May 21st - December 21st, is considered a Jupiter-line ... the price on December 21st would be higher than on May 21st\" (years 1895, 1905 ... 2025, p.8)"],
          concat!("Gillen, Astro Stats for the New York Stock Exchange (2002); printed pages (PDF page = printed + 7). ", "Each period of Table 6.3 (2003-2050; 'None' = no trade that year): buy at the close of the start date, sell at the close of the end date. Option p.decade = 1 uses the chapter 3 rule instead: in every year ending in 5, buy the close of 21 May and sell the close of 21 December. ",
            "Close-to-close on the DJIA in the book; here on every index, ETF and stock as a declared application with the same dates. ",
            "Dates that are not trading days use the close of the last trading day before them, at most 5 calendar days back (R1: Table 1.1 prints exactly those closes); a window that begins and ends on the same close is no trade, and one position at a time (R2). "),
          claim("", "98% (failed twice since 1900)", "", "months", "1900-2003 (p.36)")),
        d("jgasn_saturn_line", "Saturn-line: short DJIA over each printed Saturn-line period (Table 6.4)", "Ch.6 · Saturn-stats", "Short", Status::Live, &[
            "E|\"The Saturn-line falls into the category of the 70 - 100 percent accuracy ... the end will always be lower than the beginning, and you will find the tables from the year 2003 - 2050\" (p.31)",
            "X|close of the period's end date (Table 6.4, p.35)"],
          concat!("Gillen, Astro Stats for the New York Stock Exchange (2002); printed pages (PDF page = printed + 7). ", "Each period of Table 6.4 (2003-2050; 'None' = no trade): sell short at the close of the start date, cover at the close of the end date. ",
            "Close-to-close on the DJIA in the book; here on every index, ETF and stock as a declared application with the same dates. ",
            "Dates that are not trading days use the close of the last trading day before them, at most 5 calendar days back (R1: Table 1.1 prints exactly those closes); a window that begins and ends on the same close is no trade, and one position at a time (R2). "),
          claim("", "100% since 1980", "", "months", "1980-2003 (p.31)")),
        d("jgasn_saturn_retro", "Saturn retrograde: long DJIA from the retrograde start to its end (Tables 6.1-6.2)", "Ch.6 · Saturn-stats", "Long", Status::Live, &[
            "E|\"with a Saturn retrograde the key word is 'retreat' so that it does the opposite, instead of being lower at the exit date it will be higher\"; \"since 1980 the retrograded cycle had a 72.72 percent accuracy to the up side\" (p.31)",
            "X|close of the end date (Table 6.1, p.32, 1980-2002 with DJIA prices; Table 6.2, p.33, 2003-2050)"],
          concat!("Gillen, Astro Stats for the New York Stock Exchange (2002); printed pages (PDF page = printed + 7). ", "Each period of Tables 6.1 and 6.2: buy at the close of the start date, sell at the close of the end date. Misprints (R5): '6/20.82' and '8/21.87' read with a slash; '10/30/9' = 30 Oct 1993 (its printed 3,680.59 is the DJIA close of 29 Oct 1993). Kept as printed although it looks misprinted: Table 6.1 '6/26/96' (starts before the 1995 period's yearly step, 7/7/95 -> 6/26/96 -> 8/3/97). ",
            "Close-to-close on the DJIA in the book; here on every index, ETF and stock as a declared application with the same dates. ",
            "Dates that are not trading days use the close of the last trading day before them, at most 5 calendar days back (R1: Table 1.1 prints exactly those closes); a window that begins and ends on the same close is no trade, and one position at a time (R2). "),
          claim("22", "72.72%", "", "about 4.5 months", "1980-2002, DJIA (Table 6.1, p.32)")),
        d("jgasn_jupiter_retro", "Jupiter retrograde: long DJIA from the retrograde start to its finish (Tables 7.1-7.2)", "Ch.7 · Jupiter-stats", "Long", Status::Live, &[
            "E|\"I will usually go long at the start date or either exit or put a stop on the finish date\" (p.36)",
            "X|close of the finish date (Table 7.1, p.38, 1979-2002 with DJIA prices; Table 7.2, p.39, 2003-2050)"],
          concat!("Gillen, Astro Stats for the New York Stock Exchange (2002); printed pages (PDF page = printed + 7). ", "Each period of Tables 7.1 and 7.2 (Table 7.1 prints its 2000 row after the 2001 row; full-date tables are sorted by start, R3): buy at the close of the start date, sell at the close of the finish date. The 'stop on the finish date' alternative has no price and is not applied. ",
            "Close-to-close on the DJIA in the book; here on every index, ETF and stock as a declared application with the same dates. ",
            "Dates that are not trading days use the close of the last trading day before them, at most 5 calendar days back (R1: Table 1.1 prints exactly those closes); a window that begins and ends on the same close is no trade, and one position at a time (R2). "),
          claim("22", "77.2% (17 of 22 rows up)", "", "about 4 months", "1979-2002, DJIA (Table 7.1, p.38)")),
        d("jgasn_js_aspects", "Jupiter/Saturn aspects: DJIA 16 days before and after each aspect, long sextile/square, short trine/opposition", "Ch.7 · Jupiter-stats", "Long/Short", Status::Live, &[
            "E|\"The sextile and square aspect of Jupiter and Saturn ... the market will move up 16 days before and 16 days after the aspected date. With the trine and opposition aspect of Jupiter and Saturn the DJIA is subject to be down 16 days before and 16 days after the aspect\" (p.37)",
            "E|\"Sixteen B is the starting date 16 days before the aspect is due, and the next column will give you the aspected date. Sixteen A, is the date for the 16 days after the aspect\" (p.37; Tables 7.5-7.6, p.40-41, 2004-2050)",
            "X|close of the aspect date, then close of 16A (two predictions per aspect, Tables 7.3-7.4, p.40)"],
          concat!("Gillen, Astro Stats for the New York Stock Exchange (2002); printed pages (PDF page = printed + 7). ", "For each aspect two trades (R6): from the close of 16B to the close of the aspect date, and from there to the close of 16A; long for sextile/square (Tables 7.4, 7.5), short for trine/opposition (Tables 7.3, 7.6). Tables 7.3/7.4 (1984-1997) print only the aspect date: 16B/16A = 16 calendar days before/after (their printed closes are those dates' closes). The impossible date '11/31/08' (Table 7.6) drops that half. Kept as printed although they look misprinted (not 16 days): Table 7.5 '4/8/35 -> 4/23/35' (15 days) and Table 7.6 '7/9/46 -> 7/28/46' (19 days). ",
            "Close-to-close on the DJIA in the book; here on every index, ETF and stock as a declared application with the same dates. ",
            "Dates that are not trading days use the close of the last trading day before them, at most 5 calendar days back (R1: Table 1.1 prints exactly those closes); a window that begins and ends on the same close is no trade, and one position at a time (R2). "),
          claim("36", "75% (27 of 36)", "", "16 days", "1984-1997, DJIA (Tables 7.3-7.4, p.37, 40)")),
        d("jgasn_venus_line", "Venus-line: four or more RED (GREEN) weeks in a row = DJIA up (down)", "Ch.4 · Venus-stats", "Long/Short",
          Status::NotTestable("Held: the weekly R/G grid (Tables 4.1-4.5, p.12-15) gives no week numbering or entry price: the worked 'week 10 to week 18' trades start and end on different weekdays (5 Mar - 4 May 1979, 4 Mar - 6 May 1987, p.11), and the text ties the 100% pattern to Venus at 0 Aquarius - 14 Aries without dates, so which runs of 'four or more consecutive weekly patterns' are traded, and from which close, is not stated."), &[
            "E|\"The Venus-line means having four or more consecutive weekly patterns, and if the pattern is RED we know the trend is up, and if the pattern is GREEN we know the trend is down\" (p.11)"],
          SRC, None),
        d("jgasn_mars_cardinal", "Mars to the cardinal full moon: DJIA from one cardinal point to the next", "Ch.5 · Mars-stats", "Long/Short",
          Status::NotTestable("Held: the forward tables 5.6-5.7 (p.20-21) mark each cardinal full moon with a code A-D that the book never defines, and the 'segment' stop ('if it goes below that point it changes into a new direction', p.17) has no level; Tables 5.1-5.3 (p.17-18) list past trine/sextile/square periods only (1980-2002)."), &[
            "E|\"all you do is look at the cycle of (B) (Start Date) to be higher than (A) (Finish Date)\" (p.16)"],
          SRC, None),
        d("jgasn_top20", "Top 20 stocks from company birthdays (Jupiter, Saturn, Uranus houses)", "Ch.8-15 · Stock selection", "Long/Short",
          Status::NotTestable("Held: the stocks are picked from each company's incorporation chart and the houses Jupiter, Saturn and Uranus transit, read by judgement (earnings, advertising, volume 'in the wrong areas', p.42-44); no list of incorporation dates or objective selection rule is given."), &[
            "E|\"If I have a Jupiter-line and Saturn-line for the year, I select ten stocks to go long with and ten stocks to short\" (p.45)"],
          SRC, None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    // decision #302 (1): six methods. Ch.1 presents its three windows as the Sun-stats (24-31 Dec is the second of the
    // 'two statistics related to the sun', worked from Venus in the US chart, p.1), so they stay one method, named for both;
    // the Mercury, Saturn and Jupiter retrograde periods are one method (long over a retrograde period, ch.3, 6, 7).
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some(match id {
        "jgasn_sun_jul" | "jgasn_sun_dec" | "jgasn_sun_opp" => ("jgasn_sun", "Sun-stats: fixed DJIA calendar windows (Sun transit, Venus opposition, NYSE Sun opposition)", d.name),
        "jgasn_mercury" | "jgasn_saturn_retro" | "jgasn_jupiter_retro" => ("jgasn_retro", "Retrograde periods: long DJIA from retrograde to direct (Mercury, Saturn, Jupiter)", d.name),
        _ => (d.id, d.name, "Astro Stats for the NYSE (Gillen)"),
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label| ParamDef { key, label, def: 0., min: 0., max: 1., step: 1. };
    match id {
        "jgasn_sun_opp" => Some(vec![p("short", "Also short 17 May - 17 November (book's short side, p.2)")]),
        "jgasn_moon" => Some(vec![p("short", "Also short from each window's end to the next start (Pisces to Virgo, p.4)")]),
        "jgasn_jupiter_line" => Some(vec![p("decade", "Use the ch.3 rule instead: years ending in 5, 21 May - 21 December (p.8)")]),
        _ if id.starts_with("jgasn_") && catalogue().iter().any(|d| d.id == id) => Some(vec![]),
        _ => None,
    }
}

fn dim(y: i64, m: i64) -> i64 { if m == 12 { civil(y + 1, 1, 1) - civil(y, 12, 1) } else { civil(y, m + 1, 1) - civil(y, m, 1) } }
fn date(y: i64, m: i64, d: i64) -> Option<i64> { ((1..=12).contains(&m) && d >= 1 && d <= dim(y, m)).then(|| civil(y, m, d)) }
fn num(s: &str) -> Option<i64> { (!s.is_empty() && s.len() <= 2 && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse().ok()).flatten() }
/// "m/d-m/d" or "m/d-d" under year y (an end month before the start month is next year).
fn md(tok: &str, y: i64) -> Option<(i64, i64)> {
    let (a, b) = tok.split_once('-')?;
    let (m1, d1) = a.split_once('/')?;
    let (m1, d1) = (num(m1)?, num(d1)?);
    let (m2, d2) = match b.split_once('/') { Some((m, d)) => (num(m)?, num(d)?), None => (m1, num(b)?) };
    let (s, e) = (date(y, m1, d1)?, date(if m2 < m1 { y + 1 } else { y }, m2, d2)?);
    (e >= s).then_some((s, e))
}
/// "m/d/yy" (or "m/d.yy", R5); 70-99 = 19xx.
fn full(tok: &str) -> Option<i64> {
    let tok = if tok == "10/30/9" { "10/30/93" } else { tok };
    let (m, rest) = tok.split_once('/')?;
    let (d, y) = rest.split_once(['/', '.'])?;
    let y = num(y).filter(|_| y.len() == 2)?;
    date(if y >= 70 { 1900 + y } else { 2000 + y }, num(m)?, num(d)?)
}
/// R3, year-row tables: printed order; bad or overlapping entries skipped.
fn year_rows(t: &str, fix: &[(i64, &str, &str)]) -> Vec<(i64, i64)> {
    let mut out: Vec<(i64, i64)> = vec![];
    for line in t.lines() {
        let mut w = line.split_whitespace();
        let Some(y) = w.next().and_then(|y| y.parse::<i64>().ok()) else { continue };
        for tok in w.filter(|t| *t != "None") {
            let tok = fix.iter().find(|f| f.0 == y && f.1 == tok).map_or(tok, |f| f.2);
            if let Some(s) = md(tok, y) { if out.last().map_or(true, |l| s.0 >= l.1) { out.push(s); } }
        }
    }
    out
}
fn no_overlap(mut v: Vec<(i64, i64, f64)>) -> Vec<(i64, i64, f64)> {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut out: Vec<(i64, i64, f64)> = vec![];
    for w in v { if out.last().map_or(true, |l| w.0 >= l.1) { out.push(w); } }
    out
}
fn full_pairs(t: &str, dir: f64) -> Vec<(i64, i64, f64)> {
    no_overlap(t.lines().filter_map(|l| { let mut w = l.split_whitespace(); let (a, b) = (full(w.next()?)?, full(w.next()?)?); (b >= a).then_some((a, b, dir)) }).collect())
}

/// The card's windows (start day, end day, direction) from the dates and tables above; years = the span to generate.
fn windows(id: &str, p: &Params, y0: i64, y1: i64) -> Option<Vec<(i64, i64, f64)>> {
    let on = |k: &str| p.get(k).copied().unwrap_or(0.) >= 0.5;
    let yearly = |m1, d1, m2, d2, add: i64, dir| (y0..=y1).map(move |y| (civil(y, m1, d1), civil(y + add, m2, d2), dir));
    let long = |v: Vec<(i64, i64)>| v.into_iter().map(|(a, b)| (a, b, 1.)).collect::<Vec<_>>();
    Some(match id {
        "jgasn_sun_jul" => yearly(7, 7, 7, 15, 0, 1.).collect(),
        "jgasn_sun_dec" => yearly(12, 24, 12, 31, 0, 1.).collect(),
        "jgasn_sun_opp" => {
            let mut v: Vec<_> = yearly(11, 17, 5, 17, 1, 1.).collect();
            if on("short") { v.extend(yearly(5, 17, 11, 17, 0, -1.)); }
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v
        }
        "jgasn_moon" => {
            let l = year_rows(MOON, &[(2003, "1/12-2/3", "1/21-2/3")]);
            let mut v = long(l.clone());
            if on("short") { v.extend(l.windows(2).map(|w| (w[0].1, w[1].0, -1.))); }
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v
        }
        "jgasn_mercury" => long(year_rows(MERCURY, &[])),
        "jgasn_jupiter_line" if on("decade") => (y0..=y1).filter(|y| y.rem_euclid(10) == 5).map(|y| (civil(y, 5, 21), civil(y, 12, 21), 1.)).collect(),
        "jgasn_jupiter_line" => long(year_rows(JUP_LINE, &[])),
        "jgasn_saturn_line" => year_rows(SAT_LINE, &[]).into_iter().map(|(a, b)| (a, b, -1.)).collect(),
        "jgasn_saturn_retro" => full_pairs(SAT_RETRO, 1.),
        "jgasn_jupiter_retro" => full_pairs(JUP_RETRO, 1.),
        "jgasn_js_aspects" => {
            let mut v = vec![];
            for l in ASPECT_DATES.lines() {
                let mut w = l.split_whitespace();
                let (Some(a), Some(k)) = (w.next().and_then(full), w.next()) else { continue };
                let dir = if k == "opp" || k == "trine" { -1. } else { 1. };
                v.extend([(a - 16, a, dir), (a, a + 16, dir)]);
            }
            for l in ASPECT_TABLE.lines() {
                let w: Vec<&str> = l.split_whitespace().collect();
                if w.len() != 4 { continue; }
                let dir = if w[3] == "bear" { -1. } else { 1. };
                let (b, a, f) = (full(w[0]), full(w[1]), full(w[2]));
                if let (Some(b), Some(a)) = (b, a) { if a >= b { v.push((b, a, dir)); } }
                if let (Some(a), Some(f)) = (a, f) { if f >= a { v.push((a, f, dir)); } }
            }
            no_overlap(v)
        }
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !REVIEWED.contains(&id) { return None; }
    let n = x.r.len();
    if n == 0 || from > to || to >= n { return Some(vec![]); }
    let dn: Vec<i64> = x.r.iter().map(|r| day_num(&r.date)).collect();
    let (y0, y1) = (x.r[0].date[0..4].parse::<i64>().ok()? - 1, x.r[n - 1].date[0..4].parse::<i64>().ok()? + 1);
    // R1: the last bar on or before the day; unknown when the day is after the last bar
    // R1 (#302 (3)): at most 5 calendar days back (a longer gap in a stock's file is no trade, not an old close)
    let idx = |d: i64| { let k = dn.partition_point(|&b| b <= d); (k > 0 && (dn[k - 1] == d || k < n) && d - dn[k - 1] <= 5).then(|| k - 1) };
    let mut out = vec![];
    let mut busy = 0usize;
    for (s, e, dir) in windows(id, p, y0, y1)? {
        let Some(ei) = idx(s) else { continue };
        if ei < from || ei > to || ei < busy { continue; }
        let xi = idx(e);
        if xi.is_some_and(|k| k <= ei) { continue; }
        let mut t = Trade { dir, legs: vec![Leg { i: ei, px: x.c[ei], w: 1. }], exit_i: to, exit_px: x.c[to], reason: "open".into(), entry_kind: "close",
            signal_i: to, open: true, exits: vec![] };
        match xi {
            Some(k) if k <= to => { (t.exit_i, t.exit_px, t.signal_i, t.open, t.reason) = (k, x.c[k], k, false, "close of the end date".into()); busy = k; }
            _ => busy = n,
        }
        out.push(t);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(v: &[(&str, f64)]) -> Vec<Row> {
        v.iter().map(|&(d, c)| Row { ticker: "T".into(), date: d.into(), open: c, high: c, low: c, close: c, volume: None }).collect()
    }
    #[test]
    fn dates_use_the_last_close_on_or_before_and_tables_skip_misprints() {
        // Table 1.1, 1980/81: 17 Nov 1980 (Mon) 986.27 -> 17 May 1981 (Sun) = Fri 15 May 985.95
        let r = rows(&[("1980-11-14", 986.34), ("1980-11-17", 986.27), ("1981-05-15", 985.95), ("1981-05-18", 985.77), ("1981-11-16", 845.03), ("1981-11-17", 850.17)]);
        // 17 Nov 1981 is the last bar: entered there and open; a date after the last bar would be unknown (R1)
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jgasn_sun_opp", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.iter().map(|t| (t.legs[0].i, t.exit_i, t.open)).collect::<Vec<_>>(), vec![(1, 2, false), (5, 5, true)]);
        let s = run("jgasn_sun_opp", &x, 0, r.len() - 1, &[("short".to_string(), 1.)].into_iter().collect()).unwrap();
        assert_eq!(s.iter().map(|t| (t.dir, t.legs[0].i, t.exit_i)).collect::<Vec<_>>(), vec![(1., 1, 2), (-1., 2, 5), (1., 5, 5)]);
        // #302 (3): a gap longer than 5 calendar days is no trade (7 Jul 2000 would fall back 17 days to 20 Jun)
        let g = rows(&[("2000-06-20", 1.), ("2000-07-14", 2.), ("2000-07-17", 3.)]);
        assert!(run("jgasn_sun_jul", &Ctx::new(&g, &g, &[]), 0, 2, &Params::new()).unwrap().is_empty());
        // R3/R4 on the 2003 Moon row: '1/12-2/3' read as 1/21-2/3; '1/21-11/3' (out of order) and '11/18-31' (no such day) skipped
        let m = year_rows(MOON, &[(2003, "1/12-2/3", "1/21-2/3")]);
        let y03: Vec<_> = m.iter().filter(|w| w.0 < civil(2004, 1, 1)).collect();
        assert_eq!(y03.len(), 11);
        assert_eq!((y03[0].0, y03[0].1), (civil(2003, 1, 21), civil(2003, 2, 3)));
        assert_eq!(y03[10].0, civil(2003, 12, 15));
        assert_eq!(full("6/20.82"), Some(civil(1982, 6, 20)));
        assert_eq!(full("10/30/9"), Some(civil(1993, 10, 30)));
        assert_eq!(full("11/31/08"), None);
        assert_eq!(md("7/410-24", 2024), None);
        assert_eq!(md("12/22-1/4", 2005), Some((civil(2005, 12, 22), civil(2006, 1, 4))));
        // aspects: 18 dated aspects 1984-1997 = the book's 36 predictions
        let a = windows("jgasn_js_aspects", &Params::new(), 1970, 2030).unwrap();
        assert_eq!(a.iter().filter(|w| w.0 < civil(2003, 1, 1)).count(), 36);
        assert!(catalogue().iter().all(|d| d.id.starts_with("jgasn_") && family_of(d.id).is_some() && params(d.id).is_some()));
        assert!(REVIEWED.iter().all(|id| crate::gate::get(id).is_some()));
    }
}
