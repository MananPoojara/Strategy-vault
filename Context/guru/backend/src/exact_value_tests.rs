//! Additive std-only harness successor; SAME immutable48 expectations from
//! EXACT-API-CHALLENGES-v1-FROZEN.json SHA0160c7bbdf40fcde4433a99dc05b1fe0c5f4cdb7e1f5394c8436ef4a0792653f.
//! Frozenv1 serde-dependent compilefailure is preserved. No candidate numeric
//! output informed this module; it changes only fixture transport to Rust literals.
use super::{ExactValue,Error,evaluate};
use std::cmp::Ordering;
// Independent base16 schoolbook multiplication checks lossless audit fractions.
// It intentionally does not call candidate Natural/Rational operations, parse a
// candidate audit through f64, or require its numerator/denominator be reduced.
fn digits(text: &str) -> Vec<u32> {
    let text = text.strip_prefix("0x").unwrap_or(text);
    assert!(!text.is_empty() && text.len() <= 32768, "bounded hex audit");
    text.bytes().rev().map(|b| (b as char).to_digit(16).expect("hex audit digit")).collect()
}
fn multiply_hex(a: &str, b: &str) -> String {
    let (a, b) = (digits(a), digits(b));
    let mut out = vec![0u32; a.len() + b.len() + 1];
    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() { out[i+j] += x*y; }
    }
    for i in 0..out.len()-1 {
        let carry = out[i] / 16; out[i] %= 16; out[i+1] += carry;
    }
    while out.len()>1 && out.last()==Some(&0) {out.pop();}
    out.iter().rev().map(|&n| char::from_digit(n,16).expect("base16 product")).collect()
}
fn normalized_hex(s: &str) -> String { multiply_hex(s,"1") }
enum Rounded {Bits(u64),Error(&'static str)}
fn assert_rounded(id:&str,label:&str,got:Result<f64,Error>,want:Rounded) {
    match want {
        Rounded::Error(error)=>match got {
            Err(e)=>assert_eq!(e.code(),error,"{id}:{label}"),
            Ok(v)=>panic!("{id}:{label}: expected {error}, got {v:?}"),
        },
        Rounded::Bits(want)=> {
            let got=got.unwrap_or_else(|e|panic!("{id}:{label}: unexpected {e}"));
            assert_eq!(got.to_bits(),want,"{id}:{label}");
        }
    }
}
fn check_value(id:&str,got:Result<ExactValue,Error>,negative:bool,n:&str,d:&str,positive:bool,rounded:Rounded) {
    let got=got.unwrap_or_else(|e|panic!("{id}: unexpected construction error {e}"));
    let audit=got.audit();let an=normalized_hex(&audit.numerator_hex);let ad=normalized_hex(&audit.denominator_hex);
    assert_ne!(ad,"0","{id}: denominator must be positive");
    assert_eq!(audit.negative,negative,"{id}: sign");
    if an=="0" {assert!(!audit.negative,"{id}: zero canonical positive");}
    assert_eq!(multiply_hex(&an,d),multiply_hex(n,&ad),"{id}: lossless rational audit");
    assert_eq!(got.positive(),positive,"{id}: positive");
    assert_rounded(id,"rounded",got.rounded(),rounded);
}
fn check_invalid(id:&str,got:Result<ExactValue,Error>,error:&str) {
    match got {
        Err(e)=>assert_eq!(e.code(),error,"{id}:construction"),
        Ok(_)=>panic!("{id}: expected construction rejection {error}"),
    }
}
#[test]
fn frozen_thirty_exact_values_errors_and_means() {
    check_value("positive_zero",(|| -> Result<ExactValue,Error> {Ok(ExactValue::canonical(f64::from_bits(0x0000000000000000u64))?)})(),false,"0","1",false,Rounded::Bits(0x0000000000000000u64));
    check_value("negative_zero_canonicalized",(|| -> Result<ExactValue,Error> {Ok(ExactValue::canonical(f64::from_bits(0x8000000000000000u64))?)})(),false,"0","1",false,Rounded::Bits(0x0000000000000000u64));
    check_value("signed_negative_decimal",(|| -> Result<ExactValue,Error> {Ok(ExactValue::canonical(f64::from_bits(0xbfb999999999999au64))?)})(),true,"1","a",false,Rounded::Bits(0xbfb999999999999au64));
    check_value("smallest_subnormal_canonical",(|| -> Result<ExactValue,Error> {Ok(ExactValue::canonical(f64::from_bits(0x0000000000000001u64))?)})(),false,"1","3f3d8b077b8e0b10919ca780f767b5ed5081f0c878538c4ddf9835e86939ebea24f676b5b58628127b7075889de5f1f6b9dc1c344ff3752979ee02e133a048782c79b5eda33fd9010b60776bb0c887d3b3f6761d8edd96c0eed45c77af7d000000000000000000000000000000000000000000000000000000000000000000000000000000000",true,Rounded::Bits(0x0000000000000001u64));
    check_value("maximum_finite_canonical",(|| -> Result<ExactValue,Error> {Ok(ExactValue::canonical(f64::from_bits(0x7fefffffffffffffu64))?)})(),false,"fffffffffffff7ac6b26715bb52611f3557ab9dff9b8b22ed97fd6ccd362d748b926b935ae4cef9ce976c482510fe77592128540fd165024271d198d0f4dc7903c685ce0d8f622cfbd38498d87a8565633a38b99f3a6aa7406629e50000000000000000000000000000000000000000000000000000000000000000000000000","1",true,Rounded::Bits(0x7fefffffffffffffu64));
    check_value("third_ratio",(|| -> Result<ExactValue,Error> {Ok(ExactValue::ratio(1u32,3u32)?)})(),false,"1","3",true,Rounded::Bits(0x3fd5555555555555u64));
    check_value("signed_cancellation",(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0xc029000000000000u64))?).add(&(ExactValue::canonical(f64::from_bits(0x4029000000000000u64))?)))})(),false,"0","1",false,Rounded::Bits(0x0000000000000000u64));
    check_value("negative_subtraction",(|| -> Result<ExactValue,Error> {Ok((ExactValue::ratio(1u32,20u32)?).sub(&(ExactValue::ratio(1u32,2u32)?)))})(),true,"9","14",false,Rounded::Bits(0xbfdccccccccccccdu64));
    check_value("zero_scale",(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0xc05ed00000000000u64))?).scale(0u32))})(),false,"0","1",false,Rounded::Bits(0x0000000000000000u64));
    check_value("divide_nonzero",(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x3fc3333333333333u64))?).divide(3u32)?)})(),false,"1","14",true,Rounded::Bits(0x3fa999999999999au64));
    check_value("nearest_even_lower",(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x3ff0000000000000u64))?).add(&((ExactValue::ratio(1u32,2147483648u32)?).divide(4194304u32)?)))})(),false,"20000000000001","20000000000000",true,Rounded::Bits(0x3ff0000000000000u64));
    check_value("nearest_even_upper",(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x3ff0000000000000u64))?).add(&(((ExactValue::ratio(1u32,2147483648u32)?).divide(4194304u32)?).scale(3u32))))})(),false,"20000000000003","20000000000000",true,Rounded::Bits(0x3ff0000000000002u64));
    check_value("nearest_even_negative",(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0xbff0000000000000u64))?).sub(&((ExactValue::ratio(1u32,2147483648u32)?).divide(4194304u32)?)))})(),true,"20000000000001","20000000000000",false,Rounded::Bits(0xbff0000000000000u64));
    check_value("rounding_underflow",(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x0000000000000001u64))?).divide(4u32)?)})(),false,"1","fcf62c1dee382c4246729e03dd9ed7b54207c321e14e31377e60d7a1a4e7afa893d9dad6d618a049edc1d6227797c7dae77070d13fcdd4a5e7b80b84ce8121e0b1e6d7b68cff64042d81ddaec3221f4ecfd9d8763b765b03bb5171debdf4000000000000000000000000000000000000000000000000000000000000000000000000000000000",true,Rounded::Error("reporting_arithmetic_underflow"));
    check_value("exact_intermediate_overflow_only_final_rounded",(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x7fefffffffffffffu64))?).add(&(ExactValue::canonical(f64::from_bits(0x7fefffffffffffffu64))?)))})(),false,"1ffffffffffffef58d64ce2b76a4c23e6aaf573bff371645db2ffad99a6c5ae91724d726b5c99df39d2ed8904a21fceeb24250a81fa2ca0484e3a331a1e9b8f2078d0b9c1b1ec459f7a70931b0f50acac67471733e74d54e80cc53ca0000000000000000000000000000000000000000000000000000000000000000000000000","1",true,Rounded::Error("reporting_arithmetic_overflow"));
    check_value("mean20_normal",(|| -> Result<ExactValue,Error> {Ok(ExactValue::mean20(&[f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64)])?)})(),false,"64","1",true,Rounded::Bits(0x4059000000000000u64));
    check_value("mean20_cancellation_wide",(|| -> Result<ExactValue,Error> {Ok(ExactValue::mean20(&[f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0xffefffffffffffffu64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64)])?)})(),false,"1","14",true,Rounded::Bits(0x3fa999999999999au64));
    check_value("mean20_all_maximum",(|| -> Result<ExactValue,Error> {Ok(ExactValue::mean20(&[f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64),f64::from_bits(0x7fefffffffffffffu64)])?)})(),false,"fffffffffffff7ac6b26715bb52611f3557ab9dff9b8b22ed97fd6ccd362d748b926b935ae4cef9ce976c482510fe77592128540fd165024271d198d0f4dc7903c685ce0d8f622cfbd38498d87a8565633a38b99f3a6aa7406629e50000000000000000000000000000000000000000000000000000000000000000000000000","1",true,Rounded::Bits(0x7fefffffffffffffu64));
    check_value("mean20_all_subnormal",(|| -> Result<ExactValue,Error> {Ok(ExactValue::mean20(&[f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000001u64)])?)})(),false,"1","3f3d8b077b8e0b10919ca780f767b5ed5081f0c878538c4ddf9835e86939ebea24f676b5b58628127b7075889de5f1f6b9dc1c344ff3752979ee02e133a048782c79b5eda33fd9010b60776bb0c887d3b3f6761d8edd96c0eed45c77af7d000000000000000000000000000000000000000000000000000000000000000000000000000000000",true,Rounded::Bits(0x0000000000000001u64));
    check_value("mean20_subnormal_underflow",(|| -> Result<ExactValue,Error> {Ok(ExactValue::mean20(&[f64::from_bits(0x0000000000000001u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64)])?)})(),false,"1","4f0cedc95a718dd4b603d1613541a368a4a26cfa96686f61577e4362838866e4ae34146322e7b2171a4c92eac55f6e746853234163f05273d869839980885a96379823690c0fcf414e3895469cfaa9c8a0f413a4f294fc712a8973959b5c4000000000000000000000000000000000000000000000000000000000000000000000000000000000",true,Rounded::Error("reporting_arithmetic_underflow"));
    check_invalid("canonical_nan",(|| -> Result<ExactValue,Error> {Ok(ExactValue::canonical(f64::from_bits(0x7ff8000000000000u64))?)})(),"invalid_domain");
    check_invalid("canonical_infinity",(|| -> Result<ExactValue,Error> {Ok(ExactValue::canonical(f64::from_bits(0x7ff0000000000000u64))?)})(),"invalid_domain");
    check_invalid("canonical_negative_infinity",(|| -> Result<ExactValue,Error> {Ok(ExactValue::canonical(f64::from_bits(0xfff0000000000000u64))?)})(),"invalid_domain");
    check_invalid("ratio_zero_denominator",(|| -> Result<ExactValue,Error> {Ok(ExactValue::ratio(1u32,0u32)?)})(),"invalid_domain");
    check_invalid("divide_zero",(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x3ff0000000000000u64))?).divide(0u32)?)})(),"invalid_domain");
    check_invalid("mean19_rejected",(|| -> Result<ExactValue,Error> {Ok(ExactValue::mean20(&[f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64)])?)})(),"invalid_domain");
    check_invalid("mean21_rejected",(|| -> Result<ExactValue,Error> {Ok(ExactValue::mean20(&[f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64)])?)})(),"invalid_domain");
    check_invalid("mean20_nonfinite",(|| -> Result<ExactValue,Error> {Ok(ExactValue::mean20(&[f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x7ff8000000000000u64)])?)})(),"invalid_domain");
    check_value("caller_level_upper_collapse",(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x4340000000000000u64))?).add(&(ExactValue::ratio(1u32,20u32)?)))})(),false,"280000000000001","14",true,Rounded::Bits(0x4340000000000000u64));
    check_value("caller_level_lower_collapse",(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x4340000000000000u64))?).sub(&(ExactValue::ratio(1u32,20u32)?)))})(),false,"27fffffffffffff","14",true,Rounded::Bits(0x4340000000000000u64));
}

#[test]
fn frozen_fourteen_exact_boundaries_and_signed_orders() {
    {
        let left=(|| -> Result<ExactValue,Error> {Ok(((ExactValue::canonical(f64::from_bits(0x4059000000000000u64))?).sub(&(ExactValue::canonical(f64::from_bits(0x4057000000000000u64))?))).scale(100u32))})().unwrap();
        let right=(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x4059000000000000u64))?).scale(8u32))})().unwrap();
        assert_eq!(left.signed_cmp(&right),Ordering::Equal,"{}: exact order","CBS8pct_equal");
        assert_eq!(right.signed_cmp(&left),Ordering::Equal.reverse(),"{}: reverse order","CBS8pct_equal");
    }
    {
        let left=(|| -> Result<ExactValue,Error> {Ok(((ExactValue::canonical(f64::from_bits(0x4059000000000000u64))?).sub(&(ExactValue::canonical(f64::from_bits(0x4056ffffffffffffu64))?))).scale(100u32))})().unwrap();
        let right=(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x4059000000000000u64))?).scale(8u32))})().unwrap();
        assert_eq!(left.signed_cmp(&right),Ordering::Greater,"{}: exact order","CBS8pct_more_extended");
        assert_eq!(right.signed_cmp(&left),Ordering::Greater.reverse(),"{}: reverse order","CBS8pct_more_extended");
    }
    {
        let left=(|| -> Result<ExactValue,Error> {Ok(((ExactValue::canonical(f64::from_bits(0x4059000000000000u64))?).sub(&(ExactValue::canonical(f64::from_bits(0x4057000000000001u64))?))).scale(100u32))})().unwrap();
        let right=(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x4059000000000000u64))?).scale(8u32))})().unwrap();
        assert_eq!(left.signed_cmp(&right),Ordering::Less,"{}: exact order","CBS8pct_less_extended");
        assert_eq!(right.signed_cmp(&left),Ordering::Less.reverse(),"{}: reverse order","CBS8pct_less_extended");
    }
    {
        let left=(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x4059700000000000u64))?).sub(&(ExactValue::canonical(f64::from_bits(0x4059000000000000u64))?)))})().unwrap();
        let right=(|| -> Result<ExactValue,Error> {Ok(ExactValue::ratio(7u32,4u32)?)})().unwrap();
        assert_eq!(left.signed_cmp(&right),Ordering::Equal,"{}: exact order","BGS_range175_equal");
        assert_eq!(right.signed_cmp(&left),Ordering::Equal.reverse(),"{}: reverse order","BGS_range175_equal");
    }
    {
        let left=(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x4059200000000000u64))?).sub(&(ExactValue::canonical(f64::from_bits(0x4059000000000000u64))?)))})().unwrap();
        let right=(|| -> Result<ExactValue,Error> {Ok(ExactValue::ratio(1u32,2u32)?)})().unwrap();
        assert_eq!(left.signed_cmp(&right),Ordering::Equal,"{}: exact order","BGS_gap050_equal");
        assert_eq!(right.signed_cmp(&left),Ordering::Equal.reverse(),"{}: reverse order","BGS_gap050_equal");
    }
    {
        let left=(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x4059000000000000u64))?).add(&(ExactValue::ratio(1u32,20u32)?)))})().unwrap();
        let right=(|| -> Result<ExactValue,Error> {Ok(ExactValue::canonical(f64::from_bits(0x4059033333333333u64))?)})().unwrap();
        assert_eq!(left.signed_cmp(&right),Ordering::Equal,"{}: exact order","CBS_offset005_equal");
        assert_eq!(right.signed_cmp(&left),Ordering::Equal.reverse(),"{}: reverse order","CBS_offset005_equal");
    }
    {
        let left=(|| -> Result<ExactValue,Error> {Ok(ExactValue::canonical(f64::from_bits(0x405959999999999au64))?)})().unwrap();
        let right=(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x4059700000000000u64))?).sub(&(((ExactValue::canonical(f64::from_bits(0x4059700000000000u64))?).sub(&(ExactValue::canonical(f64::from_bits(0x4059000000000000u64))?))).divide(5u32)?)))})().unwrap();
        assert_eq!(left.signed_cmp(&right),Ordering::Equal,"{}: exact order","BGS_open_top_fifth_equal");
        assert_eq!(right.signed_cmp(&left),Ordering::Equal.reverse(),"{}: reverse order","BGS_open_top_fifth_equal");
    }
    {
        let left=(|| -> Result<ExactValue,Error> {Ok(ExactValue::canonical(f64::from_bits(0x4059166666666666u64))?)})().unwrap();
        let right=(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x4059000000000000u64))?).add(&(((ExactValue::canonical(f64::from_bits(0x4059700000000000u64))?).sub(&(ExactValue::canonical(f64::from_bits(0x4059000000000000u64))?))).divide(5u32)?)))})().unwrap();
        assert_eq!(left.signed_cmp(&right),Ordering::Equal,"{}: exact order","BGS_close_bottom_fifth_equal");
        assert_eq!(right.signed_cmp(&left),Ordering::Equal.reverse(),"{}: reverse order","BGS_close_bottom_fifth_equal");
    }
    {
        let left=(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x4057c00000000000u64))?).scale(20u32))})().unwrap();
        let right=(|| -> Result<ExactValue,Error> {Ok((ExactValue::mean20(&[f64::from_bits(0x0000000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64)])?).scale(20u32))})().unwrap();
        assert_eq!(left.signed_cmp(&right),Ordering::Equal,"{}: exact order","volume20_equal_is_not_above");
        assert_eq!(right.signed_cmp(&left),Ordering::Equal.reverse(),"{}: reverse order","volume20_equal_is_not_above");
    }
    {
        let left=(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x4057c00000000001u64))?).scale(20u32))})().unwrap();
        let right=(|| -> Result<ExactValue,Error> {Ok((ExactValue::mean20(&[f64::from_bits(0x0000000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64)])?).scale(20u32))})().unwrap();
        assert_eq!(left.signed_cmp(&right),Ordering::Greater,"{}: exact order","volume20_just_above");
        assert_eq!(right.signed_cmp(&left),Ordering::Greater.reverse(),"{}: reverse order","volume20_just_above");
    }
    {
        let left=(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x0000000000000000u64))?).scale(20u32))})().unwrap();
        let right=(|| -> Result<ExactValue,Error> {Ok((ExactValue::mean20(&[f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64)])?).scale(20u32))})().unwrap();
        assert_eq!(left.signed_cmp(&right),Ordering::Equal,"{}: exact order","all_zero_volume_equal_is_not_above");
        assert_eq!(right.signed_cmp(&left),Ordering::Equal.reverse(),"{}: reverse order","all_zero_volume_equal_is_not_above");
    }
    {
        let left=(|| -> Result<ExactValue,Error> {Ok(ExactValue::canonical(f64::from_bits(0xbfb999999999999au64))?)})().unwrap();
        let right=(|| -> Result<ExactValue,Error> {Ok(ExactValue::canonical(f64::from_bits(0x0000000000000000u64))?)})().unwrap();
        assert_eq!(left.signed_cmp(&right),Ordering::Less,"{}: exact order","signed_ordering");
        assert_eq!(right.signed_cmp(&left),Ordering::Less.reverse(),"{}: reverse order","signed_ordering");
    }
    {
        let left=(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x4340000000000000u64))?).add(&(ExactValue::ratio(1u32,20u32)?)))})().unwrap();
        let right=(|| -> Result<ExactValue,Error> {Ok(ExactValue::canonical(f64::from_bits(0x4340000000000000u64))?)})().unwrap();
        assert_eq!(left.signed_cmp(&right),Ordering::Greater,"{}: exact order","collapse_still_exactly_above");
        assert_eq!(right.signed_cmp(&left),Ordering::Greater.reverse(),"{}: reverse order","collapse_still_exactly_above");
    }
    {
        let left=(|| -> Result<ExactValue,Error> {Ok((ExactValue::canonical(f64::from_bits(0x4340000000000000u64))?).sub(&(ExactValue::ratio(1u32,20u32)?)))})().unwrap();
        let right=(|| -> Result<ExactValue,Error> {Ok(ExactValue::canonical(f64::from_bits(0x4340000000000000u64))?)})().unwrap();
        assert_eq!(left.signed_cmp(&right),Ordering::Less,"{}: exact order","collapse_still_exactly_below");
        assert_eq!(right.signed_cmp(&left),Ordering::Less.reverse(),"{}: reverse order","collapse_still_exactly_below");
    }
}

#[test]
fn frozen_four_legacy_scalar_cases_unchanged() {
    {
        let got=evaluate(f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4059000000000000u64),f64::from_bits(0x4014000000000000u64),f64::from_bits(0x0000000000000000u64),1u8).unwrap();
        assert_rounded("legacy_flat_one_side","gross",got.gross,Rounded::Bits(0x0000000000000000u64));
        assert_rounded("legacy_flat_one_side","fee",got.fee,Rounded::Bits(0x3f40624dd2f1a9fcu64));
        assert_rounded("legacy_flat_one_side","slippage",got.slippage,Rounded::Bits(0x0000000000000000u64));
        assert_rounded("legacy_flat_one_side","net",got.net,Rounded::Bits(0xbf40624dd2f1a9fcu64));
        assert_rounded("legacy_flat_one_side","gross_factor",got.gross_factor,Rounded::Bits(0x3ff0000000000000u64));
        assert_rounded("legacy_flat_one_side","net_factor",got.net_factor,Rounded::Bits(0x3feffbe76c8b4396u64));
    }
    {
        let got=evaluate(f64::from_bits(0x4024000000000000u64),f64::from_bits(0x4024051eb851eb85u64),f64::from_bits(0x4000000000000000u64),f64::from_bits(0x4008000000000000u64),2u8).unwrap();
        assert_rounded("legacy_exact_zero_net","gross",got.gross,Rounded::Bits(0x3f50624dd2f1a9fcu64));
        assert_rounded("legacy_exact_zero_net","fee",got.fee,Rounded::Bits(0x3f3a36e2eb1c432du64));
        assert_rounded("legacy_exact_zero_net","slippage",got.slippage,Rounded::Bits(0x3f43a92a30553261u64));
        assert_rounded("legacy_exact_zero_net","net",got.net,Rounded::Bits(0x0000000000000000u64));
        assert_rounded("legacy_exact_zero_net","gross_factor",got.gross_factor,Rounded::Bits(0x3ff004189374bc6au64));
        assert_rounded("legacy_exact_zero_net","net_factor",got.net_factor,Rounded::Bits(0x3ff0000000000000u64));
    }
    {
        let got=evaluate(f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000001u64),f64::from_bits(0x0000000000000000u64),f64::from_bits(0x0000000000000000u64),2u8).unwrap();
        assert_rounded("legacy_tiny_gross","gross",got.gross,Rounded::Bits(0x3cacd2b297d889bcu64));
        assert_rounded("legacy_tiny_gross","fee",got.fee,Rounded::Bits(0x0000000000000000u64));
        assert_rounded("legacy_tiny_gross","slippage",got.slippage,Rounded::Bits(0x0000000000000000u64));
        assert_rounded("legacy_tiny_gross","net",got.net,Rounded::Bits(0x3cacd2b297d889bcu64));
        assert_rounded("legacy_tiny_gross","gross_factor",got.gross_factor,Rounded::Bits(0x3ff0000000000001u64));
        assert_rounded("legacy_tiny_gross","net_factor",got.net_factor,Rounded::Bits(0x3ff0000000000001u64));
    }
    {
        let got=evaluate(f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x3ff0000000000000u64),f64::from_bits(0x40b3880000000000u64),f64::from_bits(0x0000000000000000u64),2u8).unwrap();
        assert_rounded("legacy_nonpositive_factor","gross",got.gross,Rounded::Bits(0x0000000000000000u64));
        assert_rounded("legacy_nonpositive_factor","fee",got.fee,Rounded::Bits(0x3ff0000000000000u64));
        assert_rounded("legacy_nonpositive_factor","slippage",got.slippage,Rounded::Bits(0x0000000000000000u64));
        assert_rounded("legacy_nonpositive_factor","net",got.net,Rounded::Bits(0xbff0000000000000u64));
        assert_rounded("legacy_nonpositive_factor","gross_factor",got.gross_factor,Rounded::Bits(0x3ff0000000000000u64));
        assert_rounded("legacy_nonpositive_factor","net_factor",got.net_factor,Rounded::Error("unit_exposure_equity_domain"));
    }
}
