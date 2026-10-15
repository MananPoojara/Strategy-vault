//! Portfolio universes: actual admitted stocks/ETFs on the market reference calendar.
//! Basket expansion respects the server's current-member inventory; explicit lists retain their
//! hindsight-selection disclosure. Source rows are read only through the shared admitted loader.
use super::{api::frames_of, markets, rules::{LabError, Universe, MAX_UNIVERSE, MAX_UNIVERSE_LIST}, series::Frames};
use crate::State;
use std::time::Instant;

pub struct Member { pub ticker: String, pub kind: &'static str, pub frames: Frames, pub at: Vec<Option<usize>> }
pub struct Loaded {
    pub reference: String, pub dates: Vec<String>, pub members: Vec<Member>,
    pub refused: Vec<(String, String)>, pub basket: Option<String>, pub membership: (&'static str, &'static str),
}

/// Quotes retained for individual research are not automatically current basket constituents.
pub fn basket_members(st: &State, basket: &str) -> Vec<(String, String)> {
    let mut members: Vec<_> = st.baskets.members(basket).into_iter().map(|m| (m.key, m.membership_kind)).collect();
    members.sort();
    members
}

/// Exact-date alignment only. Callers validate both calendars first; no price is forward-filled.
pub fn calendar_map(reference: &[String], member: &[String]) -> Vec<Option<usize>> {
    calendar_map_with_deadline(reference, member, None).expect("an unbounded calendar map cannot time out")
}
fn check_deadline(deadline: Option<Instant>) -> Result<(), LabError> {
    if deadline.is_some_and(|end| Instant::now() >= end) {
        return Err(LabError::Invalid("portfolio run stopped after 90 s; use fewer members or a shorter window".into()));
    }
    Ok(())
}
fn calendar_map_with_deadline(reference: &[String], member: &[String], deadline: Option<Instant>) -> Result<Vec<Option<usize>>, LabError> {
    let (mut k, mut out) = (0, Vec::with_capacity(reference.len()));
    for (i, date) in reference.iter().enumerate() {
        if i % 1024 == 0 { check_deadline(deadline)?; }
        while k < member.len() && member[k] < *date {
            if k % 1024 == 0 { check_deadline(deadline)?; }
            k += 1;
        }
        out.push((k < member.len() && member[k] == *date).then_some(k));
    }
    check_deadline(deadline)?;
    Ok(out)
}

pub fn membership_note(kind: &str) -> &'static str {
    match kind {
        "current_plus_past_snapshot" => "Members include past constituents but without the dates they were in the index, so a stock can be traded years before it joined.",
        "user_list" => "A list chosen today carries hindsight selection bias.",
        _ => "Members are today's constituents tested over past dates: stocks that left the index (often after falling) are missing, which biases results upward.",
    }
}

fn calendar_ok(dates: &[String], label: &str, deadline: Option<Instant>) -> Result<(), LabError> {
    if dates.is_empty() { return Err(LabError::NeedsData(format!("{label}: calendar has no dates"))); }
    for (i, date) in dates.iter().enumerate() {
        if i % 1024 == 0 { check_deadline(deadline)?; }
        if !crate::baskets::valid_date(date) || !date.bytes().enumerate().all(|(k, b)| if k == 4 || k == 7 { b == b'-' } else { b.is_ascii_digit() }) || (i > 0 && dates[i-1] >= *date) {
            return Err(LabError::NeedsData(format!("{label}: calendar must contain real, sorted, unique YYYY-MM-DD dates")));
        }
    }
    check_deadline(deadline)
}

/// Bound retained member Frames before accumulation; loading one source remains cooperative.
fn source_budget(used: usize, next: usize) -> Result<usize, LabError> {
    use super::api::{PORTFOLIO_MAX_MEMBER_BARS as MEMBER, PORTFOLIO_MAX_SOURCE_BARS as TOTAL};
    let refuse = |why: String| LabError::Invalid(format!("portfolio resource limit: {why}; use fewer members or shorter admitted histories (from/to does not truncate indicator warm-up)"));
    if next > MEMBER { return Err(refuse(format!("at most {MEMBER} source bars per member"))); }
    let total = used.checked_add(next).ok_or_else(|| refuse("source-bar count overflow".into()))?;
    if total > TOTAL { return Err(refuse(format!("at most {TOTAL} total source bars"))); }
    Ok(total)
}

pub fn load(st: &State, universe: &Universe) -> Result<Loaded, LabError> { load_inner(st, universe, None) }
/// API callers start one deadline before universe loading and share it with compilation/simulation.
pub fn load_with_deadline(st: &State, universe: &Universe, deadline: Instant) -> Result<Loaded, LabError> { load_inner(st, universe, Some(deadline)) }
fn load_inner(st: &State, universe: &Universe, deadline: Option<Instant>) -> Result<Loaded, LabError> {
    check_deadline(deadline)?;
    let (tickers, basket, membership_kind) = match universe {
        Universe::Basket(basket) => {
            let members = basket_members(st, basket);
            check_deadline(deadline)?;
            let expected = st.baskets.expected_member_count(basket);
            let size = expected.unwrap_or(members.len()).max(members.len());
            if size > MAX_UNIVERSE {
                return Err(LabError::Invalid(format!("{basket} has {size} members; the portfolio limit is {MAX_UNIVERSE} — choose a smaller basket or a list of at most {MAX_UNIVERSE_LIST} members")));
            }
            if expected.is_some_and(|n| n != members.len()) {
                return Err(LabError::NeedsData(format!("{basket}: expected {} basket members but only {} are admitted; the incomplete basket is not silently narrowed", expected.unwrap(), members.len())));
            }
            if members.is_empty() {
                return Err(LabError::Invalid(format!("portfolio.universe.basket: no basket '{basket}' on this engine")));
            }
            let kind = if members.iter().any(|m| m.1 == "current_plus_past_snapshot") { "current_plus_past_snapshot" } else { "current_snapshot" };
            (members.into_iter().map(|m| m.0).collect::<Vec<_>>(), Some(basket.clone()), kind)
        }
        Universe::Tickers(tickers) => {
            if !(2..=MAX_UNIVERSE_LIST).contains(&tickers.len()) {
                return Err(LabError::Invalid(format!("portfolio.universe.tickers needs 2 to {MAX_UNIVERSE_LIST} distinct instruments")));
            }
            let mut tickers = tickers.clone(); tickers.sort();
            if tickers.windows(2).any(|w| w[0] == w[1]) {
                return Err(LabError::Invalid("portfolio.universe.tickers must be distinct".into()));
            }
            (tickers, None, "user_list")
        }
    };
    // Validate the whole requested universe's identity before loading or refusing individual data.
    let mut market = None;
    for ticker in &tickers {
        check_deadline(deadline)?;
        if markets::gl(ticker).is_some() {
            return Err(LabError::NeedsData(format!("{ticker} is provisional gl data served by /api/gl; portfolios run on India/US stocks and ETFs")));
        }
        let evidence = st.baskets.native_evidence(ticker);
        let kind = crate::instrument_policy::selected_type(ticker, evidence.is_some());
        if !matches!(kind, "individual_stock" | "etf") {
            return Err(LabError::Invalid(format!("{ticker} ({kind}) cannot be a portfolio member; members are actual stocks or ETFs with their own prices (an index can be a signal through symbol)")));
        }
        let member_market = evidence.map(|e| e.market()).or_else(|| crate::preloaded_market(ticker))
            .ok_or_else(|| LabError::Invalid(format!("{ticker}: portfolio member has no established market")))?;
        if market.is_some_and(|m| m != member_market) {
            return Err(LabError::NeedsData(format!("{ticker}: cross-market universes are not available")));
        }
        market = Some(member_market);
    }
    let market = market.ok_or_else(|| LabError::Invalid("portfolio.universe: no member of this engine's market".into()))?;
    let reference = crate::native_benchmark(market).to_string();
    let context = st.ctxs.get(&reference).ok_or_else(|| LabError::NeedsData(format!("{reference} (the portfolio's reference calendar) is not loaded on this engine")))?;
    if context.r.iter().any(|r| r.ticker != reference) {
        return Err(LabError::NeedsData(format!("{reference}: reference calendar identity disagrees with its admitted source")));
    }
    let dates: Vec<_> = context.r.iter().map(|r| r.date.clone()).collect();
    calendar_ok(&dates, &format!("{reference} reference calendar"), deadline)?;
    let (mut members, mut refused) = (Vec::new(), Vec::new());
    let mut source_bars = dates.len();
    for ticker in tickers {
        check_deadline(deadline)?;
        let loaded = frames_of(st, &ticker);
        check_deadline(deadline)?;
        let (frames, kind, _, _) = match loaded {
            Ok(loaded) => loaded,
            Err(LabError::NeedsData(reason)) => { refused.push((ticker, reason)); continue; }
            Err(error) => return Err(error),
        };
        source_bars = source_budget(source_bars, frames.d.len())?;
        match calendar_ok(&frames.d.date, &ticker, deadline) {
            Ok(()) => {},
            Err(LabError::NeedsData(reason)) => { refused.push((ticker, reason)); continue; }
            Err(error) => return Err(error),
        }
        let at = calendar_map_with_deadline(&dates, &frames.d.date, deadline)?;
        members.push(Member { ticker, kind, frames, at });
    }
    if members.is_empty() {
        return Err(LabError::NeedsData(format!("portfolio.universe: no admitted member data; {}", refused.iter().map(|(ticker, reason)| format!("{ticker}: {reason}")).collect::<Vec<_>>().join("; "))));
    }
    check_deadline(deadline)?;
    // Window overlap, OHLC and indicator/volume admission are request-specific (API Task12).
    Ok(Loaded { reference, dates, members, refused, basket, membership: (membership_kind, membership_note(membership_kind)) })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{basket_members, calendar_map, calendar_ok, load, load_with_deadline};
    use crate::lab::rules::Universe;
    use std::{collections::BTreeMap, io::Write, path::PathBuf, process::{Command, Stdio}, sync::atomic::{AtomicUsize, Ordering}};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    // Only generated local fixtures are written. Keep them for inspection; no source/data files are removed.
    pub(crate) struct Dir(pub PathBuf);
    fn sha(bytes: &[u8]) -> String {
        let mut child = Command::new("sha256sum").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        child.stdin.take().unwrap().write_all(bytes).unwrap();
        String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap()[..64].into()
    }
    pub(crate) fn india(name: &str) -> (Dir, crate::State) {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/lab-universe-fixtures")
            .join(format!("{}-{}-{name}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&dir).unwrap();
        let mut index = String::from("basket,ticker,path,first,last,bars,source,source_sha256,csv_sha256,membership,status,has_volume,basket_member,basket_expected_members\n");
        let csv = |skip: usize| {
            let mut text = String::from("date,open,high,low,close,volume\n");
            for i in 0..60usize { if i == skip { continue; }
                let close = 100. + i as f64;
                text.push_str(&format!("2024-{:02}-{:02},{close},{},{},{close},1000\n", 1 + i/28, 1 + i%28, close+2., close-2.));
            }
            text
        };
        let mut add = |basket: &str, symbol: &str, skip: usize, member: bool| {
            let text = csv(skip); let file = format!("{basket}_{symbol}.csv");
            std::fs::write(dir.join(&file), &text).unwrap();
            index.push_str(&format!("{basket},{symbol},{file},2024-01-01,2024-03-04,{},Test,{},{},Current snapshot,ready,true,{member},{}\n", if skip<60 {59} else {60}, "a".repeat(64), sha(text.as_bytes()), if basket=="nifty50" {2} else {130}));
        };
        add("nifty50", "A", 99, true); add("nifty50", "B", 10, true);
        add("nifty50", "QUOTE_ONLY", 99, false);
        for k in 0..130 { add("nse500", &format!("S{k:03}"), 99, true); }
        std::fs::write(dir.join("index.csv"), index).unwrap();
        let rows: &'static [crate::Row] = Box::leak((0..60usize).map(|i| crate::Row { ticker:"NIFTY 50".into(), date:format!("2024-{:02}-{:02}",1+i/28,1+i%28),open:1., high:2.,low:0.5, close:1.5,volume:None }).collect::<Vec<_>>().into_boxed_slice());
        let st = crate::State {ctxs:[("NIFTY 50".into(),crate::connors::Ctx::new(rows,rows,&[]))].into(), baskets:crate::baskets::Catalogue::load(&dir).unwrap(),vix:vec![],market_extra:BTreeMap::new()};
        (Dir(dir),st)
    }
    #[test] fn a_basket_expands_only_current_members_on_the_reference_calendar() {
        let (_dir, st) = india("expand");
        assert_eq!(basket_members(&st,"nifty50").iter().map(|m|m.0.as_str()).collect::<Vec<_>>(), ["basket:nifty50:A","basket:nifty50:B"]);
        let loaded=load(&st,&Universe::Basket("nifty50".into())).unwrap();
        assert_eq!((loaded.reference.as_str(),loaded.dates.len(),loaded.members.len(),loaded.basket.as_deref()),("NIFTY 50",60,2,Some("nifty50")));
        assert_eq!((loaded.members[1].at[9],loaded.members[1].at[10],loaded.members[1].at[11]),(Some(9),None,Some(10)));
        assert_eq!(loaded.membership.0,"current_snapshot");
        assert!(loaded.membership.1.contains("biases results upward"));
    }
    #[test] fn explicit_lists_are_distinct_bounded_and_sorted_independent_of_input_order() {
        let (_dir,st)=india("list");
        for tickers in [vec![],vec!["basket:nifty50:A".into()],vec!["basket:nifty50:A".into();2],(0..51).map(|i|format!("basket:nse500:S{i:03}")).collect()] {
            assert_eq!(load(&st,&Universe::Tickers(tickers)).err().unwrap().status(),"invalid");
        }
        let loaded=load(&st,&Universe::Tickers(vec!["basket:nifty50:B".into(),"basket:nifty50:A".into()])).unwrap();
        assert_eq!(loaded.members.iter().map(|m|m.ticker.as_str()).collect::<Vec<_>>(),["basket:nifty50:A","basket:nifty50:B"]);
        assert_eq!(loaded.membership.0,"user_list");
        assert!(loaded.membership.1.contains("hindsight"));
        let loaded=load(&st,&Universe::Tickers(vec!["basket:nifty50:A".into(),"basket:nifty50:QUOTE_ONLY".into()])).unwrap();
        assert_eq!(loaded.members.len(),2,"an intentional user list may select an admitted stock outside current basket membership");
    }
    #[test] fn universes_never_substitute_an_index_or_provisional_prices_for_stocks() {
        let (_dir,st)=india("types");
        let list=|t:&str| Universe::Tickers(vec!["basket:nifty50:A".into(),t.into()]);
        let error=load(&st,&list("NIFTY 50")).err().unwrap();
        assert_eq!(error.status(),"invalid"); assert!(error.message().contains("NIFTY 50 (index) cannot be a portfolio member"));
        assert_eq!(load(&st,&list("IG:DAX")).err().unwrap().status(),"needs_data");
        assert_eq!(load(&st,&list("UNKNOWN")).err().unwrap().status(),"invalid");
        assert_eq!(load(&st,&list("SPY")).err().unwrap().status(),"needs_data","cross-market list refused before any missing US source can be guessed");
        assert_eq!(load(&st,&Universe::Basket("nope".into())).err().unwrap().message(),"portfolio.universe.basket: no basket 'nope' on this engine");
        let error=load(&st,&Universe::Basket("nse500".into())).err().unwrap();
        assert_eq!(error.status(),"invalid"); assert!(error.message().contains("nse500 has 130 members; the portfolio limit is 120"));
    }
    #[test] fn missing_member_files_are_listed_without_discarding_valid_peers() {
        let (dir,mut st)=india("missing");
        // Alter this generated whitelist to point at a missing generated path; preserve the original data files.
        let index=std::fs::read_to_string(dir.0.join("index.csv")).unwrap().replace("nifty50_B.csv","missing_B.csv");
        std::fs::write(dir.0.join("index.csv"),index).unwrap();
        st.baskets=crate::baskets::Catalogue::load(&dir.0).unwrap();
        let loaded=load(&st,&Universe::Basket("nifty50".into())).unwrap();
        assert_eq!(loaded.members.iter().map(|m|m.ticker.as_str()).collect::<Vec<_>>(),["basket:nifty50:A"]);
        assert_eq!(loaded.refused.len(),1); assert_eq!(loaded.refused[0].0,"basket:nifty50:B");
        assert!(loaded.refused[0].1.contains("unavailable"));
    }
    #[test] fn a_missing_reference_calendar_never_falls_back_to_member_or_index_substitute() {
        let (_dir,mut st)=india("reference"); st.ctxs.clear();
        let error=load(&st,&Universe::Basket("nifty50".into())).err().unwrap();
        assert_eq!(error.status(),"needs_data");assert!(error.message().contains("NIFTY 50"));
    }
    #[test] fn an_incomplete_declared_basket_is_not_silently_backtested_as_the_whole_universe() {
        let (dir,mut st)=india("incomplete");
        let index=std::fs::read_to_string(dir.0.join("index.csv")).unwrap().lines().map(|line| {
            if line.starts_with("nifty50,") { format!("{},3",line.rsplit_once(',').unwrap().0) } else { line.to_string() }
        }).collect::<Vec<_>>().join("\n");
        std::fs::write(dir.0.join("index.csv"),index).unwrap();
        st.baskets=crate::baskets::Catalogue::load(&dir.0).unwrap();
        let error=load(&st,&Universe::Basket("nifty50".into())).err().unwrap();
        assert_eq!(error.status(),"needs_data");assert!(error.message().contains("nifty50"));
        assert!(error.message().contains("3") && error.message().contains("2"),"{}",error.message());
    }
    #[test] fn zero_loaded_members_returns_the_member_data_reasons() {
        let (dir,mut st)=india("empty");
        let index=std::fs::read_to_string(dir.0.join("index.csv")).unwrap().replace("nifty50_A.csv","missing_A.csv").replace("nifty50_B.csv","missing_B.csv");
        std::fs::write(dir.0.join("index.csv"),index).unwrap();st.baskets=crate::baskets::Catalogue::load(&dir.0).unwrap();
        let error=load(&st,&Universe::Basket("nifty50".into())).err().unwrap();
        assert_eq!(error.status(),"needs_data");assert!(error.message().contains("basket:nifty50:A"));assert!(error.message().contains("basket:nifty50:B"));
    }
    #[test] fn reference_calendars_are_valid_sorted_unique_dates() {
        let (_dir,mut st)=india("invalid-calendar");
        for bad in ["2024-01-01","2024-02-30","2023-12-31"] {
            let mut rows=st.ctxs["NIFTY 50"].r.to_vec();rows[1].date=bad.into();
            let rows: &'static [crate::Row]=Box::leak(rows.into_boxed_slice());
            st.ctxs.insert("NIFTY 50".into(),crate::connors::Ctx::new(rows,rows,&[]));
            let error=load(&st,&Universe::Basket("nifty50".into())).err().unwrap();
            assert_eq!(error.status(),"needs_data");assert!(error.message().contains("calendar"));
        }
    }
    #[test] fn actual_us_etfs_use_spx_reference_and_keep_missing_quotes_as_holes() {
        let rows = |ticker: &str, skip: usize| -> &'static [crate::Row] {
            Box::leak((0..5).filter(|i|*i!=skip).map(|i| crate::Row {ticker:ticker.into(),date:format!("2024-01-0{}",i+1),open:100.,high:102.,low:99.,close:101.,volume:Some(1000.)}).collect::<Vec<_>>().into_boxed_slice())
        };
        let reference=rows("SPX",99);let spy=rows("SPY",2);let qqq=rows("QQQ",99);
        let mut st=crate::State {ctxs:BTreeMap::new(),baskets:Default::default(),vix:vec![],market_extra:BTreeMap::new()};
        for (name,data) in [("SPX",reference),("SPY",spy),("QQQ",qqq)] {st.ctxs.insert(name.into(),crate::connors::Ctx::new(data,reference,&[]));}
        let loaded=load(&st,&Universe::Tickers(vec!["SPY".into(),"QQQ".into()])).unwrap();
        assert_eq!(loaded.reference,"SPX");assert_eq!(loaded.members.iter().map(|m|(m.ticker.as_str(),m.kind)).collect::<Vec<_>>(),[("QQQ","etf"),("SPY","etf")]);
        assert_eq!(loaded.members[1].at,vec![Some(0),Some(1),None,Some(2),Some(3)]);
    }
    #[test] fn nonfinite_native_quotes_are_refused_with_their_source_reason() {
        let (dir,mut st)=india("nonfinite");let file=dir.0.join("nifty50_B.csv");
        let before=std::fs::read_to_string(&file).unwrap();let after=before.replace("2024-01-02,101,", "2024-01-02,NaN,");
        let index=std::fs::read_to_string(dir.0.join("index.csv")).unwrap().replace(&sha(before.as_bytes()),&sha(after.as_bytes()));
        std::fs::write(&file,after).unwrap();std::fs::write(dir.0.join("index.csv"),index).unwrap();
        st.baskets=crate::baskets::Catalogue::load(&dir.0).unwrap();
        let loaded=load(&st,&Universe::Basket("nifty50".into())).unwrap();
        assert_eq!(loaded.members.len(),1);assert_eq!(loaded.refused[0].0,"basket:nifty50:B");assert!(loaded.refused[0].1.contains("numeric field invalid"));
    }
    #[test] fn an_expired_deadline_refuses_before_loading_or_publishing_a_partial_universe() {
        let st=crate::State {ctxs:BTreeMap::new(),baskets:Default::default(),vix:vec![],market_extra:BTreeMap::new()};
        let deadline=std::time::Instant::now()-std::time::Duration::from_secs(1);
        let error=load_with_deadline(&st,&Universe::Basket("nifty50".into()),deadline).err().unwrap();
        assert_eq!(error.status(),"invalid");assert!(error.message().contains("stopped after 90 s"));
    }
    #[test] fn reference_dates_require_ascii_iso_digits_not_signed_numeric_parts() {
        for dates in [vec!["2024-+1-01".into(),"2024-01-02".into()], vec!["+202-01-01".into()]] {
            assert_eq!(calendar_ok(&dates,"reference",None).err().unwrap().status(),"needs_data");
        }
    }
    #[test] fn source_budget_accepts_exact_limits_and_refuses_whole_requests_without_allocating_history() {
        use super::source_budget;
        use crate::lab::api::{PORTFOLIO_MAX_MEMBER_BARS as MEMBER, PORTFOLIO_MAX_SOURCE_BARS as TOTAL};
        assert_eq!(source_budget(0, MEMBER).unwrap(), MEMBER);
        assert_eq!(source_budget(TOTAL - MEMBER, MEMBER).unwrap(), TOTAL);
        for (used, next, text) in [(0, MEMBER + 1, "per member"), (TOTAL - MEMBER + 1, MEMBER, "total source bars"), (usize::MAX, 1, "overflow")] {
            let error = source_budget(used, next).unwrap_err();
            assert!(matches!(error, crate::lab::rules::LabError::Invalid(_)), "resource limits are request errors, never missing-member data");
            assert!(error.message().contains(text), "{}", error.message());
            assert!(error.message().contains("fewer members or shorter admitted histories"));
        }
    }
    #[test] fn calendar_map_has_only_exact_date_matches_including_leading_and_trailing_holes() {
        let reference=["2024-01-01","2024-01-02","2024-01-03","2024-01-04","2024-01-05"].map(String::from);
        let member=["2024-01-02","2024-01-04","2024-01-06"].map(String::from);
        assert_eq!(calendar_map(&reference,&member),vec![None,Some(0),None,Some(1),None]);
        assert_eq!(calendar_map(&reference,&[]),vec![None;5]);assert!(calendar_map(&[],&member).is_empty());
    }
}
