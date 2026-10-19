//! Frozen independent expectations only. Not compiled/executed; arithmetic implementation pending root GO.
//! Reference SHA256 f3c5fef4b2913a1643ff1d69c99b5aa49aafe27e7bc37cd970d4c506afca18e5
use super::*;
#[derive(Clone, Copy, Debug)]
enum Expected { Bits(u64), Error(&'static str) }
fn check(actual: Result<f64, Error>, expected: Expected, label: &str) {
    match expected { Expected::Bits(bits) => assert_eq!(actual.unwrap().to_bits(), bits, "{label}"),
        Expected::Error(code) => assert_eq!(actual.unwrap_err().code(), code, "{label}"), }
}
#[test]
fn frozen_pinned_rust_display_tokens() {
    for (bits, token) in [
        (0x4058c00000000000, "99"),
        (0x4058c65604189375, "99.099"),
        (0x0000000000000000, "0"),
        (0x4014000000000000, "5"),
        (0x4058c65604189376, "99.09900000000002"),
        (0x4058c65604189374, "99.09899999999999"),
        (0x4058c061522a6f3f, "99.00594"),
        (0x3fb999999999999a, "0.1"),
        (0x3fc999999999999a, "0.2"),
        (0x3fd3333333333333, "0.3"),
        (0x3fd3333333333334, "0.30000000000000004"),
        (0x8000000000000000, "-0"),
        (0x4059000000000000, "100"),
        (0x3d719799812dea11, "0.000000000001"),
        (0x3ff0000000000000, "1"),
        (0x0000000000000001, "0.000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005"),
        (0x00000000000013c4, "0.000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000025"),
        (0x0000000000000002, "0.00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001"),
        (0x4010000000000000, "4"),
        (0x40b3880000000000, "5000"),
        (0x40b7700000000000, "6000"),
        (0x4024000000000000, "10"),
        (0x40d3880000000000, "20000"),
        (0x7fe1ccf385ebc8a0, "100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
        (0x7fefffffffffffff, "179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
        (0x8000000000000001, "-0.000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005"),
    ] { assert_eq!(f64::from_bits(bits).to_string(), token); }
}
#[test]
fn frozen_twenty_eight_exact_model_cases() {
    // Scalar conversion errors are independently visible instead of concealing other fields.
    let mut count=0;
    { // economic_break_even_99_99_099
        let result=evaluate_for_test(f64::from_bits(0x4058c00000000000), f64::from_bits(0x4058c65604189375), f64::from_bits(0x0000000000000000), f64::from_bits(0x4014000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x3f50624dd2f1a9fc), "economic_break_even_99_99_099:gross");
        check(value.fee, Expected::Bits(0x0000000000000000), "economic_break_even_99_99_099:fee");
        check(value.slippage, Expected::Bits(0x3f50624dd2f1a9fc), "economic_break_even_99_99_099:slippage");
        check(value.net, Expected::Bits(0x0000000000000000), "economic_break_even_99_99_099:net");
        check(value.gross_factor, Expected::Bits(0x3ff004189374bc6a), "economic_break_even_99_99_099:gross_factor");
        check(value.net_factor, Expected::Bits(0x3ff0000000000000), "economic_break_even_99_99_099:net_factor");
        count+=1;
    }
    { // break_even_adjacent_above
        let result=evaluate_for_test(f64::from_bits(0x4058c00000000000), f64::from_bits(0x4058c65604189376), f64::from_bits(0x0000000000000000), f64::from_bits(0x4014000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x3f50624dd2f1ada0), "break_even_adjacent_above:gross");
        check(value.fee, Expected::Bits(0x0000000000000000), "break_even_adjacent_above:fee");
        check(value.slippage, Expected::Bits(0x3f50624dd2f1a9fc), "break_even_adjacent_above:slippage");
        check(value.net, Expected::Bits(0x3cad1d3adc9caa28), "break_even_adjacent_above:net");
        check(value.gross_factor, Expected::Bits(0x3ff004189374bc6b), "break_even_adjacent_above:gross_factor");
        check(value.net_factor, Expected::Bits(0x3ff0000000000001), "break_even_adjacent_above:net_factor");
        count+=1;
    }
    { // break_even_adjacent_below
        let result=evaluate_for_test(f64::from_bits(0x4058c00000000000), f64::from_bits(0x4058c65604189374), f64::from_bits(0x0000000000000000), f64::from_bits(0x4014000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x3f50624dd2f1a82a), "break_even_adjacent_below:gross");
        check(value.fee, Expected::Bits(0x0000000000000000), "break_even_adjacent_below:fee");
        check(value.slippage, Expected::Bits(0x3f50624dd2f1a9fc), "break_even_adjacent_below:slippage");
        check(value.net, Expected::Bits(0xbc9d1d3adc9caa28), "break_even_adjacent_below:net");
        check(value.gross_factor, Expected::Bits(0x3ff004189374bc6a), "break_even_adjacent_below:gross_factor");
        check(value.net_factor, Expected::Bits(0x3fefffffffffffff), "break_even_adjacent_below:net_factor");
        count+=1;
    }
    { // component_split_point1_point2_exact_zero
        let result=evaluate_for_test(f64::from_bits(0x4058c00000000000), f64::from_bits(0x4058c061522a6f3f), f64::from_bits(0x3fb999999999999a), f64::from_bits(0x3fc999999999999a), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x3f0f75104d551d69), "component_split_point1_point2_exact_zero:gross");
        check(value.fee, Expected::Bits(0x3ef4f8b588e368f1), "component_split_point1_point2_exact_zero:fee");
        check(value.slippage, Expected::Bits(0x3f04f8b588e368f1), "component_split_point1_point2_exact_zero:slippage");
        check(value.net, Expected::Bits(0x0000000000000000), "component_split_point1_point2_exact_zero:net");
        check(value.gross_factor, Expected::Bits(0x3ff0003eea209aaa), "component_split_point1_point2_exact_zero:gross_factor");
        check(value.net_factor, Expected::Bits(0x3ff0000000000000), "component_split_point1_point2_exact_zero:net_factor");
        count+=1;
    }
    { // component_point3_exact_zero
        let result=evaluate_for_test(f64::from_bits(0x4058c00000000000), f64::from_bits(0x4058c061522a6f3f), f64::from_bits(0x3fd3333333333333), f64::from_bits(0x0000000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x3f0f75104d551d69), "component_point3_exact_zero:gross");
        check(value.fee, Expected::Bits(0x3f0f75104d551d69), "component_point3_exact_zero:fee");
        check(value.slippage, Expected::Bits(0x0000000000000000), "component_point3_exact_zero:slippage");
        check(value.net, Expected::Bits(0x0000000000000000), "component_point3_exact_zero:net");
        check(value.gross_factor, Expected::Bits(0x3ff0003eea209aaa), "component_point3_exact_zero:gross_factor");
        check(value.net_factor, Expected::Bits(0x3ff0000000000000), "component_point3_exact_zero:net_factor");
        count+=1;
    }
    { // forbidden_preadded_point30000000000000004_differs
        let result=evaluate_for_test(f64::from_bits(0x4058c00000000000), f64::from_bits(0x4058c061522a6f3f), f64::from_bits(0x3fd3333333333334), f64::from_bits(0x0000000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x3f0f75104d551d69), "forbidden_preadded_point30000000000000004_differs:gross");
        check(value.fee, Expected::Bits(0x3f0f75104d551d6a), "forbidden_preadded_point30000000000000004_differs:fee");
        check(value.slippage, Expected::Bits(0x0000000000000000), "forbidden_preadded_point30000000000000004_differs:slippage");
        check(value.net, Expected::Bits(0xbbc2e3b40a0e9b4f), "forbidden_preadded_point30000000000000004_differs:net");
        check(value.gross_factor, Expected::Bits(0x3ff0003eea209aaa), "forbidden_preadded_point30000000000000004_differs:gross_factor");
        check(value.net_factor, Expected::Bits(0x3ff0000000000000), "forbidden_preadded_point30000000000000004_differs:net_factor");
        count+=1;
    }
    { // positive_zero_cost
        let result=evaluate_for_test(f64::from_bits(0x4058c00000000000), f64::from_bits(0x4058c00000000000), f64::from_bits(0x0000000000000000), f64::from_bits(0x0000000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x0000000000000000), "positive_zero_cost:gross");
        check(value.fee, Expected::Bits(0x0000000000000000), "positive_zero_cost:fee");
        check(value.slippage, Expected::Bits(0x0000000000000000), "positive_zero_cost:slippage");
        check(value.net, Expected::Bits(0x0000000000000000), "positive_zero_cost:net");
        check(value.gross_factor, Expected::Bits(0x3ff0000000000000), "positive_zero_cost:gross_factor");
        check(value.net_factor, Expected::Bits(0x3ff0000000000000), "positive_zero_cost:net_factor");
        count+=1;
    }
    { // negative_zero_cost
        let result=evaluate_for_test(f64::from_bits(0x4058c00000000000), f64::from_bits(0x4058c00000000000), f64::from_bits(0x8000000000000000), f64::from_bits(0x8000000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x0000000000000000), "negative_zero_cost:gross");
        check(value.fee, Expected::Bits(0x0000000000000000), "negative_zero_cost:fee");
        check(value.slippage, Expected::Bits(0x0000000000000000), "negative_zero_cost:slippage");
        check(value.net, Expected::Bits(0x0000000000000000), "negative_zero_cost:net");
        check(value.gross_factor, Expected::Bits(0x3ff0000000000000), "negative_zero_cost:gross_factor");
        check(value.net_factor, Expected::Bits(0x3ff0000000000000), "negative_zero_cost:net_factor");
        count+=1;
    }
    { // intermediate_one_side
        let result=evaluate_for_test(f64::from_bits(0x4058c00000000000), f64::from_bits(0x4058c00000000000), f64::from_bits(0x0000000000000000), f64::from_bits(0x4014000000000000), 1);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x0000000000000000), "intermediate_one_side:gross");
        check(value.fee, Expected::Bits(0x0000000000000000), "intermediate_one_side:fee");
        check(value.slippage, Expected::Bits(0x3f40624dd2f1a9fc), "intermediate_one_side:slippage");
        check(value.net, Expected::Bits(0xbf40624dd2f1a9fc), "intermediate_one_side:net");
        check(value.gross_factor, Expected::Bits(0x3ff0000000000000), "intermediate_one_side:gross_factor");
        check(value.net_factor, Expected::Bits(0x3feffbe76c8b4396), "intermediate_one_side:net_factor");
        count+=1;
    }
    { // closed_two_sides
        let result=evaluate_for_test(f64::from_bits(0x4058c00000000000), f64::from_bits(0x4058c00000000000), f64::from_bits(0x0000000000000000), f64::from_bits(0x4014000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x0000000000000000), "closed_two_sides:gross");
        check(value.fee, Expected::Bits(0x0000000000000000), "closed_two_sides:fee");
        check(value.slippage, Expected::Bits(0x3f50624dd2f1a9fc), "closed_two_sides:slippage");
        check(value.net, Expected::Bits(0xbf50624dd2f1a9fc), "closed_two_sides:net");
        check(value.gross_factor, Expected::Bits(0x3ff0000000000000), "closed_two_sides:gross_factor");
        check(value.net_factor, Expected::Bits(0x3feff7ced916872b), "closed_two_sides:net_factor");
        count+=1;
    }
    { // zero_cost_benchmark
        let result=evaluate_for_test(f64::from_bits(0x4058c00000000000), f64::from_bits(0x4059000000000000), f64::from_bits(0x0000000000000000), f64::from_bits(0x0000000000000000), 0);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x3f84afd6a052bf5b), "zero_cost_benchmark:gross");
        check(value.fee, Expected::Bits(0x0000000000000000), "zero_cost_benchmark:fee");
        check(value.slippage, Expected::Bits(0x0000000000000000), "zero_cost_benchmark:slippage");
        check(value.net, Expected::Bits(0x3f84afd6a052bf5b), "zero_cost_benchmark:net");
        check(value.gross_factor, Expected::Bits(0x3ff0295fad40a57f), "zero_cost_benchmark:gross_factor");
        check(value.net_factor, Expected::Bits(0x3ff0295fad40a57f), "zero_cost_benchmark:net_factor");
        count+=1;
    }
    { // tiny_nonzero_cost_not_hidden
        let result=evaluate_for_test(f64::from_bits(0x4058c00000000000), f64::from_bits(0x4058c00000000000), f64::from_bits(0x0000000000000000), f64::from_bits(0x3d719799812dea11), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x0000000000000000), "tiny_nonzero_cost_not_hidden:gross");
        check(value.fee, Expected::Bits(0x0000000000000000), "tiny_nonzero_cost_not_hidden:fee");
        check(value.slippage, Expected::Bits(0x3cacd2b297d889bc), "tiny_nonzero_cost_not_hidden:slippage");
        check(value.net, Expected::Bits(0xbcacd2b297d889bc), "tiny_nonzero_cost_not_hidden:net");
        check(value.gross_factor, Expected::Bits(0x3ff0000000000000), "tiny_nonzero_cost_not_hidden:gross_factor");
        check(value.net_factor, Expected::Bits(0x3feffffffffffffe), "tiny_nonzero_cost_not_hidden:net_factor");
        count+=1;
    }
    { // tiny_fee_nonzero_to_zero_rejected
        let result=evaluate_for_test(f64::from_bits(0x3ff0000000000000), f64::from_bits(0x3ff0000000000000), f64::from_bits(0x0000000000000001), f64::from_bits(0x0000000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x0000000000000000), "tiny_fee_nonzero_to_zero_rejected:gross");
        check(value.fee, Expected::Error("reporting_arithmetic_underflow"), "tiny_fee_nonzero_to_zero_rejected:fee");
        check(value.slippage, Expected::Bits(0x0000000000000000), "tiny_fee_nonzero_to_zero_rejected:slippage");
        check(value.net, Expected::Error("reporting_arithmetic_underflow"), "tiny_fee_nonzero_to_zero_rejected:net");
        check(value.gross_factor, Expected::Bits(0x3ff0000000000000), "tiny_fee_nonzero_to_zero_rejected:gross_factor");
        check(value.net_factor, Expected::Bits(0x3ff0000000000000), "tiny_fee_nonzero_to_zero_rejected:net_factor");
        count+=1;
    }
    { // representable_subnormal_fee_and_net
        let result=evaluate_for_test(f64::from_bits(0x3ff0000000000000), f64::from_bits(0x3ff0000000000000), f64::from_bits(0x00000000000013c4), f64::from_bits(0x0000000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x0000000000000000), "representable_subnormal_fee_and_net:gross");
        check(value.fee, Expected::Bits(0x0000000000000001), "representable_subnormal_fee_and_net:fee");
        check(value.slippage, Expected::Bits(0x0000000000000000), "representable_subnormal_fee_and_net:slippage");
        check(value.net, Expected::Bits(0x8000000000000001), "representable_subnormal_fee_and_net:net");
        check(value.gross_factor, Expected::Bits(0x3ff0000000000000), "representable_subnormal_fee_and_net:gross_factor");
        check(value.net_factor, Expected::Bits(0x3ff0000000000000), "representable_subnormal_fee_and_net:net_factor");
        count+=1;
    }
    { // representable_subnormal_price
        let result=evaluate_for_test(f64::from_bits(0x0000000000000001), f64::from_bits(0x0000000000000002), f64::from_bits(0x0000000000000000), f64::from_bits(0x0000000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x3ff0000000000000), "representable_subnormal_price:gross");
        check(value.fee, Expected::Bits(0x0000000000000000), "representable_subnormal_price:fee");
        check(value.slippage, Expected::Bits(0x0000000000000000), "representable_subnormal_price:slippage");
        check(value.net, Expected::Bits(0x3ff0000000000000), "representable_subnormal_price:net");
        check(value.gross_factor, Expected::Bits(0x4000000000000000), "representable_subnormal_price:gross_factor");
        check(value.net_factor, Expected::Bits(0x4000000000000000), "representable_subnormal_price:net_factor");
        count+=1;
    }
    { // positive_subnormal_factor_despite_rounded_net_minus1
        let result=evaluate_for_test(f64::from_bits(0x3ff0000000000000), f64::from_bits(0x0000000000000001), f64::from_bits(0x0000000000000000), f64::from_bits(0x0000000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0xbff0000000000000), "positive_subnormal_factor_despite_rounded_net_minus1:gross");
        check(value.fee, Expected::Bits(0x0000000000000000), "positive_subnormal_factor_despite_rounded_net_minus1:fee");
        check(value.slippage, Expected::Bits(0x0000000000000000), "positive_subnormal_factor_despite_rounded_net_minus1:slippage");
        check(value.net, Expected::Bits(0xbff0000000000000), "positive_subnormal_factor_despite_rounded_net_minus1:net");
        check(value.gross_factor, Expected::Bits(0x0000000000000001), "positive_subnormal_factor_despite_rounded_net_minus1:gross_factor");
        check(value.net_factor, Expected::Bits(0x0000000000000001), "positive_subnormal_factor_despite_rounded_net_minus1:net_factor");
        count+=1;
    }
    { // positive_factor_nonzero_to_zero_rejected
        let result=evaluate_for_test(f64::from_bits(0x4010000000000000), f64::from_bits(0x0000000000000001), f64::from_bits(0x0000000000000000), f64::from_bits(0x0000000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0xbff0000000000000), "positive_factor_nonzero_to_zero_rejected:gross");
        check(value.fee, Expected::Bits(0x0000000000000000), "positive_factor_nonzero_to_zero_rejected:fee");
        check(value.slippage, Expected::Bits(0x0000000000000000), "positive_factor_nonzero_to_zero_rejected:slippage");
        check(value.net, Expected::Bits(0xbff0000000000000), "positive_factor_nonzero_to_zero_rejected:net");
        check(value.gross_factor, Expected::Error("reporting_arithmetic_underflow"), "positive_factor_nonzero_to_zero_rejected:gross_factor");
        check(value.net_factor, Expected::Error("reporting_arithmetic_underflow"), "positive_factor_nonzero_to_zero_rejected:net_factor");
        count+=1;
    }
    { // exact_nonpositive_factor_zero
        let result=evaluate_for_test(f64::from_bits(0x3ff0000000000000), f64::from_bits(0x3ff0000000000000), f64::from_bits(0x40b3880000000000), f64::from_bits(0x0000000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x0000000000000000), "exact_nonpositive_factor_zero:gross");
        check(value.fee, Expected::Bits(0x3ff0000000000000), "exact_nonpositive_factor_zero:fee");
        check(value.slippage, Expected::Bits(0x0000000000000000), "exact_nonpositive_factor_zero:slippage");
        check(value.net, Expected::Bits(0xbff0000000000000), "exact_nonpositive_factor_zero:net");
        check(value.gross_factor, Expected::Bits(0x3ff0000000000000), "exact_nonpositive_factor_zero:gross_factor");
        check(value.net_factor, Expected::Error("unit_exposure_equity_domain"), "exact_nonpositive_factor_zero:net_factor");
        count+=1;
    }
    { // exact_negative_factor
        let result=evaluate_for_test(f64::from_bits(0x3ff0000000000000), f64::from_bits(0x3ff0000000000000), f64::from_bits(0x40b7700000000000), f64::from_bits(0x0000000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x0000000000000000), "exact_negative_factor:gross");
        check(value.fee, Expected::Bits(0x3ff3333333333333), "exact_negative_factor:fee");
        check(value.slippage, Expected::Bits(0x0000000000000000), "exact_negative_factor:slippage");
        check(value.net, Expected::Bits(0xbff3333333333333), "exact_negative_factor:net");
        check(value.gross_factor, Expected::Bits(0x3ff0000000000000), "exact_negative_factor:gross_factor");
        check(value.net_factor, Expected::Error("unit_exposure_equity_domain"), "exact_negative_factor:net_factor");
        count+=1;
    }
    { // helper_no_invented_http_fee_cap
        let result=evaluate_for_test(f64::from_bits(0x3ff0000000000000), f64::from_bits(0x4024000000000000), f64::from_bits(0x40d3880000000000), f64::from_bits(0x0000000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x4022000000000000), "helper_no_invented_http_fee_cap:gross");
        check(value.fee, Expected::Bits(0x4010000000000000), "helper_no_invented_http_fee_cap:fee");
        check(value.slippage, Expected::Bits(0x0000000000000000), "helper_no_invented_http_fee_cap:slippage");
        check(value.net, Expected::Bits(0x4014000000000000), "helper_no_invented_http_fee_cap:net");
        check(value.gross_factor, Expected::Bits(0x4024000000000000), "helper_no_invented_http_fee_cap:gross_factor");
        check(value.net_factor, Expected::Bits(0x4018000000000000), "helper_no_invented_http_fee_cap:net_factor");
        count+=1;
    }
    { // split_sum_overflows_f64_before_scaling_but_exact_finite
        let result=evaluate_for_test(f64::from_bits(0x3ff0000000000000), f64::from_bits(0x7fe1ccf385ebc8a0), f64::from_bits(0x7fe1ccf385ebc8a0), f64::from_bits(0x7fe1ccf385ebc8a0), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x7fe1ccf385ebc8a0), "split_sum_overflows_f64_before_scaling_but_exact_finite:gross");
        check(value.fee, Expected::Bits(0x7f1d2a1be4048f90), "split_sum_overflows_f64_before_scaling_but_exact_finite:fee");
        check(value.slippage, Expected::Bits(0x7f1d2a1be4048f90), "split_sum_overflows_f64_before_scaling_but_exact_finite:slippage");
        check(value.net, Expected::Bits(0x7fe1cb20e42d8857), "split_sum_overflows_f64_before_scaling_but_exact_finite:net");
        check(value.gross_factor, Expected::Bits(0x7fe1ccf385ebc8a0), "split_sum_overflows_f64_before_scaling_but_exact_finite:gross_factor");
        check(value.net_factor, Expected::Bits(0x7fe1cb20e42d8857), "split_sum_overflows_f64_before_scaling_but_exact_finite:net_factor");
        count+=1;
    }
    { // finite_maximum_return
        let result=evaluate_for_test(f64::from_bits(0x3ff0000000000000), f64::from_bits(0x7fefffffffffffff), f64::from_bits(0x0000000000000000), f64::from_bits(0x0000000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Bits(0x7fefffffffffffff), "finite_maximum_return:gross");
        check(value.fee, Expected::Bits(0x0000000000000000), "finite_maximum_return:fee");
        check(value.slippage, Expected::Bits(0x0000000000000000), "finite_maximum_return:slippage");
        check(value.net, Expected::Bits(0x7fefffffffffffff), "finite_maximum_return:net");
        check(value.gross_factor, Expected::Bits(0x7fefffffffffffff), "finite_maximum_return:gross_factor");
        check(value.net_factor, Expected::Bits(0x7fefffffffffffff), "finite_maximum_return:net_factor");
        count+=1;
    }
    { // true_ratio_overflow
        let result=evaluate_for_test(f64::from_bits(0x0000000000000001), f64::from_bits(0x7fefffffffffffff), f64::from_bits(0x0000000000000000), f64::from_bits(0x0000000000000000), 2);
        let value=result.unwrap();
        check(value.gross, Expected::Error("reporting_arithmetic_overflow"), "true_ratio_overflow:gross");
        check(value.fee, Expected::Bits(0x0000000000000000), "true_ratio_overflow:fee");
        check(value.slippage, Expected::Bits(0x0000000000000000), "true_ratio_overflow:slippage");
        check(value.net, Expected::Error("reporting_arithmetic_overflow"), "true_ratio_overflow:net");
        check(value.gross_factor, Expected::Error("reporting_arithmetic_overflow"), "true_ratio_overflow:gross_factor");
        check(value.net_factor, Expected::Error("reporting_arithmetic_overflow"), "true_ratio_overflow:net_factor");
        count+=1;
    }
    { // invalid_negative_zero_entry
        let result=evaluate_for_test(f64::from_bits(0x8000000000000000), f64::from_bits(0x3ff0000000000000), f64::from_bits(0x0000000000000000), f64::from_bits(0x0000000000000000), 2);
        assert_eq!(result.unwrap_err().code(), "invalid_domain", "invalid_negative_zero_entry");
        count+=1;
    }
    { // invalid_zero_exit
        let result=evaluate_for_test(f64::from_bits(0x3ff0000000000000), f64::from_bits(0x0000000000000000), f64::from_bits(0x0000000000000000), f64::from_bits(0x0000000000000000), 2);
        assert_eq!(result.unwrap_err().code(), "invalid_domain", "invalid_zero_exit");
        count+=1;
    }
    { // invalid_negative_fee
        let result=evaluate_for_test(f64::from_bits(0x3ff0000000000000), f64::from_bits(0x3ff0000000000000), f64::from_bits(0x8000000000000001), f64::from_bits(0x0000000000000000), 2);
        assert_eq!(result.unwrap_err().code(), "invalid_domain", "invalid_negative_fee");
        count+=1;
    }
    { // invalid_nonfinite_price
        let result=evaluate_for_test(f64::from_bits(0x3ff0000000000000), f64::from_bits(0x7ff0000000000000), f64::from_bits(0x0000000000000000), f64::from_bits(0x0000000000000000), 2);
        assert_eq!(result.unwrap_err().code(), "invalid_domain", "invalid_nonfinite_price");
        count+=1;
    }
    { // invalid_nonfinite_fee
        let result=evaluate_for_test(f64::from_bits(0x3ff0000000000000), f64::from_bits(0x3ff0000000000000), f64::from_bits(0x7ff8000000000000), f64::from_bits(0x0000000000000000), 2);
        assert_eq!(result.unwrap_err().code(), "invalid_domain", "invalid_nonfinite_fee");
        count+=1;
    }
    assert_eq!(count,28);
}
#[test]
fn frozen_fifteen_nearest_even_rational_cases() {
    let mut count=0;
    check(round_ratio_for_test("0", "1"), Expected::Bits(0x0000000000000000), "positive_zero"); count+=1;
    check(round_ratio_for_test("9007199254740993", "9007199254740992"), Expected::Bits(0x3ff0000000000000), "half_ulp_even_one"); count+=1;
    check(round_ratio_for_test("9007199254740995", "9007199254740992"), Expected::Bits(0x3ff0000000000002), "half_ulp_even_nextnext"); count+=1;
    check(round_ratio_for_test("-9007199254740993", "9007199254740992"), Expected::Bits(0xbff0000000000000), "negative_half_ulp_even_one"); count+=1;
    check(round_ratio_for_test("1", "202402253307310618352495346718917307049556649764142118356901358027430339567995346891960383701437124495187077864316811911389808737385793476867013399940738509921517424276566361364466907742093216341239767678472745068562007483424692698618103355649159556340810056512358769552333414615230502532186327508646006263307707741093494784"), Expected::Bits(0x0000000000000001), "smallest_subnormal"); count+=1;
    check(round_ratio_for_test("-1", "202402253307310618352495346718917307049556649764142118356901358027430339567995346891960383701437124495187077864316811911389808737385793476867013399940738509921517424276566361364466907742093216341239767678472745068562007483424692698618103355649159556340810056512358769552333414615230502532186327508646006263307707741093494784"), Expected::Bits(0x8000000000000001), "negative_smallest_subnormal"); count+=1;
    check(round_ratio_for_test("1", "404804506614621236704990693437834614099113299528284236713802716054860679135990693783920767402874248990374155728633623822779617474771586953734026799881477019843034848553132722728933815484186432682479535356945490137124014966849385397236206711298319112681620113024717539104666829230461005064372655017292012526615415482186989568"), Expected::Error("reporting_arithmetic_underflow"), "half_smallest_subnormal_rejected"); count+=1;
    check(round_ratio_for_test("-1", "404804506614621236704990693437834614099113299528284236713802716054860679135990693783920767402874248990374155728633623822779617474771586953734026799881477019843034848553132722728933815484186432682479535356945490137124014966849385397236206711298319112681620113024717539104666829230461005064372655017292012526615415482186989568"), Expected::Error("reporting_arithmetic_underflow"), "negative_half_smallest_rejected"); count+=1;
    check(round_ratio_for_test("3", "404804506614621236704990693437834614099113299528284236713802716054860679135990693783920767402874248990374155728633623822779617474771586953734026799881477019843034848553132722728933815484186432682479535356945490137124014966849385397236206711298319112681620113024717539104666829230461005064372655017292012526615415482186989568"), Expected::Bits(0x0000000000000002), "subnormal_tie_even_two"); count+=1;
    check(round_ratio_for_test("4503599627370495", "202402253307310618352495346718917307049556649764142118356901358027430339567995346891960383701437124495187077864316811911389808737385793476867013399940738509921517424276566361364466907742093216341239767678472745068562007483424692698618103355649159556340810056512358769552333414615230502532186327508646006263307707741093494784"), Expected::Bits(0x000fffffffffffff), "largest_subnormal"); count+=1;
    check(round_ratio_for_test("1", "44942328371557897693232629769725618340449424473557664318357520289433168951375240783177119330601884005280028469967848339414697442203604155623211857659868531094441973356216371319075554900311523529863270738021251442209537670585615720368478277635206809290837627671146574559986811484619929076208839082406056034304"), Expected::Bits(0x0010000000000000), "smallest_normal"); count+=1;
    check(round_ratio_for_test("9007199254740991", "404804506614621236704990693437834614099113299528284236713802716054860679135990693783920767402874248990374155728633623822779617474771586953734026799881477019843034848553132722728933815484186432682479535356945490137124014966849385397236206711298319112681620113024717539104666829230461005064372655017292012526615415482186989568"), Expected::Bits(0x0010000000000000), "normal_boundary_tie_even"); count+=1;
    check(round_ratio_for_test("179769313486231570814527423731704356798070567525844996598917476803157260780028538760589558632766878171540458953514382464234321326889464182768467546703537516986049910576551282076245490090389328944075868508455133942304583236903222948165808559332123348274797826204144723168738177180919299881250404026184124858368", "1"), Expected::Bits(0x7fefffffffffffff), "max_finite"); count+=1;
    check(round_ratio_for_test("179769313486231580793728971405303415079934132710037826936173778980444968292764750946649017977587207096330286416692887910946555547851940402630657488671505820681908902000708383676273854845817711531764475730270069855571366959622842914819860834936475292719074168444365510704342711559699508093042880177904174497792", "1"), Expected::Error("reporting_arithmetic_overflow"), "positive_overflow_tie"); count+=1;
    check(round_ratio_for_test("-179769313486231580793728971405303415079934132710037826936173778980444968292764750946649017977587207096330286416692887910946555547851940402630657488671505820681908902000708383676273854845817711531764475730270069855571366959622842914819860834936475292719074168444365510704342711559699508093042880177904174497792", "1"), Expected::Error("reporting_arithmetic_overflow"), "negative_overflow_tie"); count+=1;
    assert_eq!(count,15);
}
#[test]
fn frozen_five_checked_binary64_products() {
    let mut count=0;
    check(checked_product(f64::from_bits(0x0000000000000001), f64::from_bits(0x3ff0000000000000)), Expected::Bits(0x0000000000000001), "positive_subnormal_kept"); count+=1;
    check(checked_product(f64::from_bits(0x0000000000000001), f64::from_bits(0x3fe0000000000000)), Expected::Error("reporting_arithmetic_underflow"), "product_nonzero_to_zero"); count+=1;
    check(checked_product(f64::from_bits(0x0000000000000001), f64::from_bits(0x3fe3333333333333)), Expected::Bits(0x0000000000000001), "product_subnormal_rounds_up"); count+=1;
    check(checked_product(f64::from_bits(0x7fefffffffffffff), f64::from_bits(0x4000000000000000)), Expected::Error("reporting_arithmetic_overflow"), "product_overflow"); count+=1;
    check(checked_product(f64::from_bits(0x0000000000000000), f64::from_bits(0x7fefffffffffffff)), Expected::Bits(0x0000000000000000), "exact_zero_operand"); count+=1;
    assert_eq!(count,5);
}
