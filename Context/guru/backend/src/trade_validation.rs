//! Structural admission for the single-symbol return engine. Never fixes a trade to pass.
use crate::connors::Trade;
pub fn check(trades:&[Trade],from:usize,to:usize,cost:f64)->Result<(),String>{check_inner(trades,from,to,Some(cost))}
/// All existing structural/event checks, with economic admission owned by the
/// explicitly scoped canonical unit report rather than legacy ratio arithmetic.
pub fn check_structure(trades:&[Trade],from:usize,to:usize)->Result<(),String>{check_inner(trades,from,to,None)}
fn check_inner(trades:&[Trade],from:usize,to:usize,cost:Option<f64>)->Result<(),String>{
    let bad=|i:usize,why:&str|format!("calculation_unavailable: Trade{}: {why}. No performance is published.",i+1);
    for (i,t) in trades.iter().enumerate(){
        let Some(first)=t.legs.first() else{return Err(bad(i,"no observed entry leg"))};
        if first.i<from || first.i>t.exit_i || t.exit_i>to || t.signal_i>t.exit_i {
            return Err(bad(i,"entry/exit indices are outside the selected chronological window"));
        }
        if t.legs.iter().any(|l|l.i<first.i||l.i>t.exit_i||!l.px.is_finite()||l.px<=0.||!l.w.is_finite()||l.w<=0.)
            || t.legs.windows(2).any(|w|w[0].i>w[1].i) || !t.exit_px.is_finite() || t.exit_px<=0.
            || ![-1.,1.].contains(&t.dir){return Err(bad(i,"invalid prices, leg weights, direction or leg ordering"));}
        if t.deployed()>1.+1e-9 {return Err(bad(i,"exposure exceeds the supported unit allocation; source sizing, leverage, funding and margin accounting are not supplied"));}
        if i>0 && (trades[i-1].open || first.i<trades[i-1].exit_i){return Err(bad(i,"positions overlap or an earlier open position was not closed; portfolio allocation/accounting is unsupported"));}
        if t.open && i+1!=trades.len(){return Err(bad(i,"only the terminal unresolved position can remain open"));}
        if let Some(cost)=cost {if !t.ret(cost).is_finite() || t.ret(cost)<=-1. {return Err(bad(i,"return implies nonpositive normalized equity without a supported bankruptcy/margin model"));}}
        if !t.exits.is_empty(){
            if t.exits.iter().any(|&(k,p,w)|k<first.i||k>t.exit_i||!p.is_finite()||p<=0.||!w.is_finite()||w<=0.)
                || t.exits.windows(2).any(|w|w[0].0>w[1].0)
                || (t.exits.iter().map(|e|e.2).sum::<f64>()-1.).abs()>1e-8 {
                return Err(bad(i,"scale-out prices, chronology or fractions do not describe one complete position"));
            }
            let average=t.exits.iter().map(|e|e.1*e.2).sum::<f64>();
            if (average-t.exit_px).abs()>1e-8*average.abs().max(1.){return Err(bad(i,"exit price disagrees with the scale-out ledger"));}
        }
    }
    Ok(())
}
#[cfg(test)]mod tests{
    use super::*;use crate::connors::Leg;
    fn t(a:usize,b:usize)->Trade{Trade{dir:1.,legs:vec![Leg{i:a,px:100.,w:1.}],exit_i:b,exit_px:110.,reason:"close".into(),entry_kind:"close",signal_i:b,open:false,exits:vec![]}}
    #[test]fn sequential_and_known_shared_boundary_are_supported(){assert!(check(&[t(0,2),t(2,5)],0,5,0.).is_ok());}
    #[test]fn overlapping_observations_cannot_be_compounded_as_one_position(){assert!(check(&[t(0,4),t(2,5)],0,5,0.).unwrap_err().contains("overlap"));}
    #[test]fn unmodeled_pyramiding_is_withheld_not_capped(){let mut x=t(0,5);x.legs.push(Leg{i:1,px:102.,w:1.});assert!(check(&[x],0,5,0.).unwrap_err().contains("exposure"));}
    #[test]fn source_prices_and_scale_outs_are_never_repaired(){let mut x=t(0,5);x.exit_px=f64::NAN;assert!(check(&[x],0,5,0.).is_err());let mut x=t(0,5);x.exits=vec![(2,110.,0.5),(5,110.,0.3)];assert!(check(&[x],0,5,0.).is_err());}
}
