//! Strict request-line admission for the new owner only. The caller must invoke
//! this before legacy query collection, and must never fall back on failure.
use crate::{pristine_request::Failure, pristine_transport};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Endpoint {
    Strategies,
    Params,
    Readiness,
    Summary,
    Run,
}
#[derive(Debug)]
pub(crate) struct Request {
    pub endpoint: Endpoint,
    pub query: BTreeMap<String, String>,
}
fn failure(code: &'static str) -> Failure {
    Failure {
        stage: "request",
        code,
        at: None,
        signal_i: None,
        source_i: None,
    }
}

/// `head` is the original HTTP head bytes, not a lossy UTF-8 conversion.
/// The gateway retains its existing authentication/authorization contract.
pub(crate) fn parse(head: &[u8]) -> Result<Request, Failure> {
    let line_end = head
        .windows(2)
        .position(|w| w == b"\r\n")
        .unwrap_or(head.len());
    let line = &head[..line_end];
    if line.len() > 8192 {
        return Err(failure("pristine_request_target_limit"));
    }
    // Visible ASCII only in the origin-form request line. UTF-8 query values
    // must be percent encoded and are then decoded strictly by the pure decoder.
    if !line.iter().all(|&b| (0x20..=0x7e).contains(&b)) {
        return Err(failure("pristine_invalid_request_line"));
    }
    let text = std::str::from_utf8(line).map_err(|_| failure("pristine_invalid_request_line"))?;
    let mut fields = text.split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return Err(failure("pristine_invalid_request_line"));
    };
    if method != "GET" {
        return Err(failure("pristine_method_not_allowed"));
    }
    if !matches!(version, "HTTP/1.0" | "HTTP/1.1") || target.contains('#') {
        return Err(failure("pristine_invalid_request_line"));
    }
    let (path, raw) = target.split_once('?').unwrap_or((target, ""));
    let endpoint = match path {
        "/api/guru/pristine/strategies" => Endpoint::Strategies,
        "/api/guru/pristine/params" => Endpoint::Params,
        "/api/guru/pristine/readiness" => Endpoint::Readiness,
        "/api/guru/pristine/summary" => Endpoint::Summary,
        "/api/guru/pristine/run" => Endpoint::Run,
        _ => return Err(failure("pristine_unknown_endpoint")),
    };
    let query = pristine_transport::decode_query(raw)?;
    // Metadata is not an execution endpoint. Do not silently accept ignored
    // ticker, mode, cost, parameter, capital or other user-supplied fields here.
    if matches!(endpoint, Endpoint::Strategies | Endpoint::Params) && !query.is_empty() {
        return Err(failure("pristine_metadata_query_unsupported"));
    }
    Ok(Request { endpoint, query })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_only_declared_origin_paths_and_preserves_query() {
        for (suffix, expected) in [
            ("strategies", Endpoint::Strategies),
            ("params", Endpoint::Params),
            ("readiness", Endpoint::Readiness),
            ("summary", Endpoint::Summary),
            ("run", Endpoint::Run),
        ] {
            let input = format!("GET /api/guru/pristine/{suffix} HTTP/1.1\r\nHost: localhost");
            assert_eq!(parse(input.as_bytes()).unwrap().endpoint, expected);
        }
        let r=parse(b"GET /api/guru/pristine/run?id=ptpm1_daily_cbs_cog&ticker=basket%3Aus_sp500%3AAMZN&research_mode=supplied_usd_series_units HTTP/1.0\r\nHost: x").unwrap();
        assert_eq!(r.query["ticker"], "basket:us_sp500:AMZN");
        assert_eq!(r.query["research_mode"], "supplied_usd_series_units");
        assert_eq!(r.query.len(), 3);
    }
    #[test]
    fn rejects_malformed_request_lines_without_legacy_fallback() {
        for input in [
            "GET  /api/guru/pristine/run HTTP/1.1",
            "GET /api/guru/pristine/run  HTTP/1.1",
            "GET\t/api/guru/pristine/run HTTP/1.1",
            "GET /api/guru/pristine/run HTTP/2.0",
            "GET /api/guru/pristine/run HTTP/1.1 extra",
            "GET /api/guru/pristine/run#fragment HTTP/1.1",
            "GET /api/guru/pristine/run?x=two words HTTP/1.1",
            "GET /api/guru/pristine/run?x=é HTTP/1.1",
        ] {
            assert_eq!(
                parse(input.as_bytes()).unwrap_err().code,
                "pristine_invalid_request_line",
                "{input}"
            );
        }
        let mut bytes = b"GET /api/guru/pristine/run?x=".to_vec();
        bytes.push(0xff);
        bytes.extend_from_slice(b" HTTP/1.1");
        assert_eq!(
            parse(&bytes).unwrap_err().code,
            "pristine_invalid_request_line"
        );
    }
    #[test]
    fn rejects_methods_unknown_paths_and_ignored_metadata_queries() {
        for method in ["POST", "PUT", "DELETE", "HEAD", "OPTIONS", ""] {
            let input = format!("{method} /api/guru/pristine/run HTTP/1.1");
            assert_eq!(
                parse(input.as_bytes()).unwrap_err().code,
                "pristine_method_not_allowed"
            );
        }
        for path in [
            "https://x/api/guru/pristine/run",
            "/api/guru/pristine/optimize",
            "/api/guru/pristine/basket-run",
            "/api/guru/pristine/run/",
            "/api/guru/pristine/%72un",
        ] {
            let input = format!("GET {path} HTTP/1.1");
            assert_eq!(
                parse(input.as_bytes()).unwrap_err().code,
                "pristine_unknown_endpoint"
            );
        }
        for path in ["strategies", "params"] {
            let input = format!("GET /api/guru/pristine/{path}?id=x HTTP/1.1");
            assert_eq!(
                parse(input.as_bytes()).unwrap_err().code,
                "pristine_metadata_query_unsupported"
            );
        }
    }
    #[test]
    fn decoder_checks_original_query_before_collection() {
        for query in ["id=a&id=b", "id=a&%69d=b"] {
            let input = format!("GET /api/guru/pristine/run?{query} HTTP/1.1");
            assert_eq!(
                parse(input.as_bytes()).unwrap_err().code,
                "pristine_duplicate_query_field"
            );
        }
        for query in ["x=%GG", "x=%FF", "x=%C0%AF", "x=%"] {
            let input = format!("GET /api/guru/pristine/run?{query} HTTP/1.1");
            assert_eq!(
                parse(input.as_bytes()).unwrap_err().code,
                "pristine_invalid_query_encoding"
            );
        }
        let input = format!("GET /api/guru/pristine/run?x={} HTTP/1.1", "a".repeat(8192));
        assert_eq!(
            parse(input.as_bytes()).unwrap_err().code,
            "pristine_request_target_limit"
        );
    }
}
