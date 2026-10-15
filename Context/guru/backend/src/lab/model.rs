//! Execution model per instrument kind (Phase 7 spec §7): what a lab result means for an index, a stock, an ETF, crypto or a
//! CFD, and which combinations are refused. The kinds come from the server's own instrument identity (api.rs instrument()).
use super::json::esc;
use super::rules::LabError;

#[derive(Debug)]
pub struct Model { pub kind: String, pub mode: &'static str, pub executable: bool, pub notes: Vec<String> }

pub const INDEX: &str = "An index cannot be traded; returns are a study of index levels, not of an ETF or futures contract.";
pub const INDIA_SHORT: &str = "Hypothetical — overnight short positions are not executable in the Indian cash segment (they need F&O or SLB borrowing).";
pub const US_SHORT_UNIT: &str = "Short positions: borrow cost is not modelled in unit mode.";
pub const CRYPTO: &str = "CRYPTO:* is a TradingView index composite, not an exchange order book.";
pub const CRYPTO_SHORT: &str = "Hypothetical — shorting spot needs a margin borrow or perpetual data with funding, which is not staged.";
pub const CFD: &str = "Provisional CFD quotes; CFD contract terms (size, financing) are not admitted.";
pub const MAX_EVENTS: usize = 20;
pub const FUTURES: &str = "futures contract data (a contract chain with expiries, lot sizes and tick size) is not admitted; no futures result is published, and the index or spot series is never used as a stand-in";

/// `kind`: index | etf | individual_stock | cfd | crypto; `market`: "in" / "us" (None for gl instruments);
/// `shorts`: the spec can open short positions; `money`: execution.capital was given.
pub fn model(kind: &str, market: Option<&str>, shorts: bool, money: bool) -> Result<Model, LabError> {
    let mut m = Model { kind: kind.into(), mode: if money { "money" } else { "unit" }, executable: true, notes: vec![] };
    let refuse = |why: &str| Err(LabError::NeedsData(format!("{why}; no money result is published")));
    match kind {
        "index" => {
            if money { return refuse("an index has no tradable quantity: money sizing needs an ETF or a futures contract with its own data (the index is never priced as a stand-in)"); }
            (m.executable, m.notes) = (false, vec![INDEX.into()]);
        }
        "crypto" => {
            if money { return refuse("the crypto series is a TradingView index composite, not an exchange order book"); }
            m.executable = false;
            m.notes.push(CRYPTO.into());
            if shorts { m.notes.push(CRYPTO_SHORT.into()); }
        }
        "cfd" => {
            if money { return refuse("CFD contract terms are not admitted"); }
            (m.executable, m.notes) = (false, vec![CFD.into()]);
        }
        _ if shorts && market == Some("in") => {
            if money { return refuse("India cash shorts held overnight need an F&O contract or an SLB borrow model, and neither is admitted"); }
            (m.executable, m.notes) = (false, vec![INDIA_SHORT.into()]);
        }
        _ if shorts && !money => m.notes.push(US_SHORT_UNIT.into()),
        _ => {}
    }
    Ok(m)
}
impl Model {
    /// `events`: the run's equity-level events (first MAX_EVENTS, then a count); None (optimize) → null: events are reported by run only.
    pub fn json(&self, events: Option<&[String]>) -> String {
        let list = |v: &[String]| v.iter().map(|s| format!("\"{}\"", esc(s))).collect::<Vec<_>>().join(",");
        let events = events.map_or("null".into(), |v| {
            let more = (v.len() > MAX_EVENTS).then(|| format!(",\"… and {} more\"", v.len() - MAX_EVENTS)).unwrap_or_default();
            format!("[{}{more}]", list(&v[..v.len().min(MAX_EVENTS)]))
        });
        format!("{{\"kind\":\"{}\",\"mode\":\"{}\",\"executable\":{},\"notes\":[{}],\"events\":{events}}}", esc(&self.kind), self.mode, self.executable, list(&self.notes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn each_kind_says_what_its_result_means() {
        let m = model("index", Some("in"), false, false).unwrap();
        assert_eq!((m.executable, m.notes[0].as_str()), (false, INDEX));
        assert_eq!(model("index", Some("us"), false, true).unwrap_err().status(), "needs_data");
        let m = model("individual_stock", Some("in"), true, false).unwrap();
        assert_eq!((m.executable, m.notes[0].as_str()), (false, INDIA_SHORT));
        assert!(model("individual_stock", Some("in"), true, true).unwrap_err().message().contains("F&O contract or an SLB"));
        assert!(model("individual_stock", Some("in"), false, true).unwrap().executable);
        assert_eq!(model("etf", Some("us"), true, false).unwrap().notes, vec![US_SHORT_UNIT.to_string()]);
        assert!(model("etf", Some("us"), true, true).unwrap().notes.is_empty());
        let m = model("crypto", None, true, false).unwrap();
        assert_eq!((m.executable, m.notes.len()), (false, 2));
        assert_eq!(model("crypto", None, false, true).unwrap_err().status(), "needs_data");
        assert_eq!(model("cfd", None, false, true).unwrap_err().status(), "needs_data");
        let j = model("index", Some("in"), false, false).unwrap().json(Some(&["e \"1\"".to_string()]));
        let many: Vec<String> = (0..23).map(|k| format!("e{k}")).collect();
        let m = model("etf", Some("us"), false, false).unwrap();
        assert!(m.json(Some(&many)).ends_with("\"e19\",\"… and 3 more\"]}") && m.json(None).ends_with("\"events\":null}"));
        assert!(j.starts_with("{\"kind\":\"index\",\"mode\":\"unit\",\"executable\":false,\"notes\":[\"An index") && j.ends_with("\"events\":[\"e \\\"1\\\"\"]}"), "{j}");
    }
}
