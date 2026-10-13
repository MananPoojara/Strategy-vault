//! Source arithmetic only. None of these functions is a complete strategy or publishes a backtest.
//! Printed CCI 1.5 and standard 0.015 are explicit caller choices; neither is silently substituted.
#![allow(dead_code)]

pub fn bond_thirty_seconds(whole: u32, fraction: u32) -> Option<u32> {
    if fraction >= 32 { return None; }
    whole.checked_mul(32)?.checked_add(fraction)
}
pub fn average_range(high_low: &[(f64,f64)]) -> Option<f64> {
    if high_low.is_empty() || high_low.iter().any(|(h,l)| !h.is_finite() || !l.is_finite() || h<l) { return None; }
    let value=high_low.iter().map(|(h,l)| h-l).sum::<f64>()/high_low.len() as f64;
    value.is_finite().then_some(value)
}
/// Manual p. 137 example. These are projections, never observed or executable prices.
pub fn congestion_projection(open:f64,high:f64,low:f64,close:f64) -> Option<(f64,f64,f64)> {
    if ![open,high,low,close].iter().all(|n| n.is_finite() && *n>0.0) || open<low || open>high || close<low || close>high {return None;}
    let x=(open+high+low+2.0*close)/5.0;
    let (projected_low,projected_high)=(2.0*x-high,2.0*x-low);
    let typical=(projected_low+projected_high)/2.0;
    [projected_low,projected_high,typical].iter().all(|v| v.is_finite()).then_some((projected_low,projected_high,typical))
}
/// Mean absolute deviation of this exact completed window, not standard deviation or Wilder smoothing.
pub fn cci_from_typical_prices(window:&[f64], divisor:f64) -> Option<f64> {
    if window.is_empty() || !divisor.is_finite() || divisor<=0.0 || window.iter().any(|v| !v.is_finite()) {return None;}
    let mean=window.iter().sum::<f64>()/window.len() as f64;
    let deviation=window.iter().map(|v| (v-mean).abs()).sum::<f64>()/window.len() as f64;
    if deviation==0.0 {return None;}
    let value=(window.last()?-mean)/(divisor*deviation);
    value.is_finite().then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn printed_bond_example_is_exact_without_guessing_general_rounding() {
        // Ross Hook p.138, visually checked. Published prices, not manufactured market data.
        let pairs=[((103,20),(103,8)),((103,16),(103,2)),((103,14),(102,19)),((102,27),(101,29)),((101,29),(100,28))];
        let hl:Vec<_>=pairs.iter().map(|(h,l)| (bond_thirty_seconds(h.0,h.1).unwrap() as f64,bond_thirty_seconds(l.0,l.1).unwrap() as f64)).collect();
        assert_eq!(average_range(&hl),Some(23.2));
        assert_eq!(bond_thirty_seconds(103,14).unwrap()-23,bond_thirty_seconds(102,23).unwrap());
        assert_eq!(bond_thirty_seconds(103,32),None);
        assert_eq!(bond_thirty_seconds(u32::MAX,0),None);
    }
    #[test]
    fn manual_congestion_example_preserves_projected_price_identity() {
        let (lo,hi,tp)=congestion_projection(24.0,25.0,23.0,23.5).unwrap();
        assert!((lo-22.6).abs()<1e-12 && (hi-24.6).abs()<1e-12 && (tp-23.6).abs()<1e-12);
        assert!(congestion_projection(30.0,25.0,23.0,23.5).is_none());
        assert!(congestion_projection(f64::NAN,25.0,23.0,23.5).is_none());
    }
    #[test]
    fn cci_source_conflict_is_explicit_and_missing_inputs_are_not_zero() {
        // Private arithmetic fixture, no trading result. Mean=2, mean absolute deviation=2/3.
        assert!((cci_from_typical_prices(&[1.0,2.0,3.0],0.015).unwrap()-100.0).abs()<1e-12);
        assert!((cci_from_typical_prices(&[1.0,2.0,3.0],1.5).unwrap()-1.0).abs()<1e-12);
        for window in [&[][..],&[1.0,1.0][..],&[1.0,f64::NAN][..]] {assert!(cci_from_typical_prices(window,0.015).is_none());}
        assert!(cci_from_typical_prices(&[1.0,2.0],0.0).is_none());
        assert!(average_range(&[]).is_none());
        assert!(average_range(&[(1.0,2.0)]).is_none());
        assert!(average_range(&[(f64::NAN,1.0)]).is_none());
    }
}
