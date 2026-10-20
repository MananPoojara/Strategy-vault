//! Private bounded query decoder for the new Pristine research adapter.
//!
//! Integration must call this on the raw query before any legacy collection
//! discards duplicate keys. This pure decoder registers no HTTP route and makes
//! no claim about raw HTTP-target, authentication or strategy admission rules.
use crate::pristine_request::Failure;
use std::collections::BTreeMap;

const MAX_RAW_BYTES: usize = 4096;
const MAX_FIELDS: usize = 16;
const MAX_NAME_BYTES: usize = 64;
const MAX_VALUE_BYTES: usize = 512;

fn failure(code: &'static str) -> Failure {
    // Never echo input fragments, paths, credentials or a partial map.
    Failure {
        stage: "request",
        code,
        at: None,
        signal_i: None,
        source_i: None,
    }
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn decode_part(raw: &str) -> Result<String, Failure> {
    // Every part is bounded by the already checked raw-query byte limit.
    let bytes = raw.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let high = bytes.get(i + 1).copied().and_then(hex);
                let low = bytes.get(i + 2).copied().and_then(hex);
                let (Some(high), Some(low)) = (high, low) else {
                    return Err(failure("pristine_invalid_query_encoding"));
                };
                decoded.push((high << 4) | low);
                i += 3;
            }
            b'+' => {
                decoded.push(b' ');
                i += 1;
            }
            byte => {
                decoded.push(byte);
                i += 1;
            }
        }
    }
    let value =
        String::from_utf8(decoded).map_err(|_| failure("pristine_invalid_query_encoding"))?;
    if value.chars().any(char::is_control) {
        return Err(failure("pristine_invalid_query_field"));
    }
    Ok(value)
}

/// Decode query text without a leading question mark. Names and values are
/// decoded independently, with no case, numeric, ticker or mode normalization.
pub(crate) fn decode_query(raw: &str) -> Result<BTreeMap<String, String>, Failure> {
    if raw.len() > MAX_RAW_BYTES {
        return Err(failure("pristine_query_limit"));
    }
    if raw.split('&').filter(|field| !field.is_empty()).count() > MAX_FIELDS {
        return Err(failure("pristine_query_limit"));
    }
    let mut query = BTreeMap::new();
    for field in raw.split('&').filter(|field| !field.is_empty()) {
        let Some((raw_name, raw_value)) = field.split_once('=') else {
            return Err(failure("pristine_invalid_query_field"));
        };
        let name = decode_part(raw_name)?;
        let value = decode_part(raw_value)?;
        if name.is_empty() {
            return Err(failure("pristine_invalid_query_field"));
        }
        if name.len() > MAX_NAME_BYTES || value.len() > MAX_VALUE_BYTES {
            return Err(failure("pristine_query_limit"));
        }
        if !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
        {
            return Err(failure("pristine_invalid_query_field"));
        }
        if query.contains_key(&name) {
            return Err(failure("pristine_duplicate_query_field"));
        }
        query.insert(name, value);
    }
    Ok(query)
}
