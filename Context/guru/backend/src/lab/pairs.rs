//! Phase 8 checked pairs semantics; hand-computed fixtures, not platform-export parity.
use super::bars::Bars;
use super::catalogue::{int,n,IndicatorDef,Needs,Output,ParamSpec,Params,Placement};
use super::ta::NA;

const SECOND: Needs = Needs {second_symbol:true,..Needs::NONE};
const L20: &[ParamSpec] = &[int("length","Length",20.,2.,2000.)];
const fn def(id: &'static str,name: &'static str,params: &'static [ParamSpec],definition: &'static str,first: &'static [usize],f: fn(&Bars,&Params)->Output) -> IndicatorDef {
    IndicatorDef {id,name,category:"Second instrument",params,outputs:&["value"],placement:Placement::Pane,needs:SECOND,definition,first,sparse:false,compute:Some(f),note:""}
}
// Declared formulas; fixtures below are hand calculations/public worked examples, not TradingView exports.
// https://www.tradingview.com/support/solutions/43000502022-correlation-coefficient-cc/
// https://www.amibroker.com/guide/afl/correlation.html
pub const DEFS: &[IndicatorDef] = &[
    def("correlation","Correlation Coefficient (CC)",L20,"Pearson correlation of close and the aligned vs close over length bars; a missing pair or zero variance gives no value",&[19],correlation),
    def("correlation_log","Correlation - Log",L20,"Declared log-return correlation: Pearson of ln(close/previous close) for both instruments over length bars; platform-export parity pending",&[20],correlation_log),
    def("ratio","Ratio",&[],"close divided by aligned vs close; zero denominator gives no value",&[0],ratio),
    def("spread","Spread",&[],"close minus aligned vs close",&[0],spread),
];
fn second(b: &Bars) -> Result<&[f64],String> {
    let s = b.second.as_deref().ok_or_else(||"needs a second instrument".to_string())?;
    if b.date.len() != b.close.len() || s.len() != b.close.len() {return Err("pair dates, closes and second closes must have equal lengths".into());}
    super::foreign::validate_dates(&b.date,"pair calendar").map_err(|e|e.message().to_string())?;
    if b.close.iter().chain(s).any(|v|v.is_infinite()) {return Err("pair inputs cannot contain infinity".into());}
    Ok(s)
}
fn finite_result(v: f64) -> Result<f64,String> {if v.is_infinite(){Err("pair arithmetic overflow".into())}else{Ok(v)}}
fn ratio(b: &Bars,_: &Params) -> Output {let s=second(b)?;Ok(vec![b.close.iter().zip(s).map(|(&a,&c)|if c==0. {Ok(NA)}else{finite_result(a/c)}).collect::<Result<Vec<_>,_>>()?])}
fn spread(b: &Bars,_: &Params) -> Output {let s=second(b)?;Ok(vec![b.close.iter().zip(s).map(|(&a,&c)|finite_result(a-c)).collect::<Result<Vec<_>,_>>()?])}

#[derive(Default)]
struct Moments {count:usize,mx:f64,my:f64,xx:f64,yy:f64,xy:f64}
impl Moments {
    fn finite(&self) -> Result<(),String> {if [self.mx,self.my,self.xx,self.yy,self.xy].iter().all(|v|v.is_finite()){Ok(())}else{Err("pair correlation arithmetic overflow".into())}}
    fn add(&mut self,x:f64,y:f64) -> Result<(),String> {
        if x.is_nan() || y.is_nan(){return Ok(());}
        self.count+=1;let (dx,dy)=(x-self.mx,y-self.my);self.mx+=dx/self.count as f64;self.my+=dy/self.count as f64;
        self.xx+=dx*(x-self.mx);self.yy+=dy*(y-self.my);self.xy+=dx*(y-self.my);self.finite()
    }
    fn remove(&mut self,x:f64,y:f64) -> Result<(),String> {
        if x.is_nan() || y.is_nan(){return Ok(());}
        if self.count<=1 {*self=Self::default();return Ok(());}
        self.count-=1;let (dx,dy)=(x-self.mx,y-self.my);self.mx-=dx/self.count as f64;self.my-=dy/self.count as f64;
        self.xx-=dx*(x-self.mx);self.yy-=dy*(y-self.my);self.xy-=dx*(y-self.my);self.finite()
    }
}
/// Bar-counted rolling Pearson, O(bars) amortized: stable centered moments, rebased once per window.
/// NaN pairs blank their whole window; infinities, vector mismatches and arithmetic overflow are errors.
pub fn pearson(x:&[f64],y:&[f64],n:usize) -> Result<Vec<f64>,String> {
    if x.len()!=y.len(){return Err("correlation inputs must have equal lengths".into());}
    if n<2 {return Err("correlation length must be at least 2".into());}
    if x.iter().chain(y).any(|v|v.is_infinite()){return Err("correlation inputs cannot contain infinity".into());}
    let mut out=vec![NA;x.len()];let mut m=Moments::default();
    for i in 0..x.len() {
        if n==2 {
            if i==0 || [x[i],x[i-1],y[i],y[i-1]].iter().any(|v|v.is_nan()){continue;}
            let (dx,dy)=(x[i]-x[i-1],y[i]-y[i-1]);finite_result(dx)?;finite_result(dy)?;
            if dx!=0. && dy!=0. {out[i]=dx.signum()*dy.signum();}continue;
        }
        if i>=n {m.remove(x[i-n],y[i-n])?;}m.add(x[i],y[i])?;
        if i+1<n {continue;}
        if (i+1)%n==0 || m.xx<0. || m.yy<0. {
            m=Moments::default();for k in i+1-n..=i {m.add(x[k],y[k])?;}
        }
        if m.count==n && m.xx>0. && m.yy>0. {out[i]=finite_result(m.xy/m.xx.sqrt()/m.yy.sqrt())?.clamp(-1.,1.);}
    }
    Ok(out)
}
fn log_returns(x:&[f64]) -> Vec<f64> {
    (0..x.len()).map(|i|{
        if i==0 || !x[i].is_finite() || !x[i-1].is_finite() || x[i]<=0. || x[i-1]<=0. {return NA;}
        let ratio=x[i]/x[i-1];if ratio.is_finite() && ratio>0. {ratio.ln()}else{x[i].ln()-x[i-1].ln()}
    }).collect()
}
fn correlation(b:&Bars,p:&Params)->Output {Ok(vec![pearson(&b.close,second(b)?,n(p,"length"))?])}
fn correlation_log(b:&Bars,p:&Params)->Output {Ok(vec![pearson(&log_returns(&b.close),&log_returns(second(b)?),n(p,"length"))?])}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::catalogue::{get, run};
    use crate::lab::testdata::{assert_series, close_eq, closes, go, walk};
    const N: f64 = f64::NAN;
    fn pair(a: &[f64], b: &[f64]) -> Bars { let mut x = closes(a); x.second = Some(b.to_vec()); x }

    #[test] fn ratio_and_spread_follow_the_second_close() {
        let b = pair(&[10., 12., 9.], &[5., 0., N]);
        assert_series(&go("ratio", &b, &[])[0], &[2., N, N]);
        assert_series(&go("spread", &b, &[])[0], &[5., 12., N]);
    }
    #[test] fn correlation_is_pearson_over_a_bar_counted_window() {
        let b = pair(&[1., 2., 3., 5., 4.], &[2., 4., 6., 7., 9.]);
        assert_series(&go("correlation", &b, &[("length", 3.)])[0], &[N, N, 1., 13. / 14., (3f64 / 28.).sqrt()]);
        assert_series(&go("correlation", &pair(&[1., 2., 3., 4.], &[1., N, 3., 4.]), &[("length", 2.)])[0], &[N, N, N, 1.]);
        assert_series(&go("correlation", &pair(&[1., 1., 1.], &[1., 2., 3.]), &[("length", 3.)])[0], &[N, N, N]); // zero variance
        // log returns: identical returns correlate 1; a constant window has no value
        assert_series(&go("correlation_log", &pair(&[1., 2., 4., 6.], &[10., 20., 40., 60.]), &[("length", 2.)])[0], &[N, N, N, 1.]);
    }
    #[test] fn pairs_need_a_second_instrument_and_never_use_later_bars() {
        for id in ["correlation", "correlation_log", "ratio", "spread"] {
            let d = get(id).unwrap();
            assert!(d.compute.is_some() && d.needs.second_symbol && d.note.is_empty(), "{id}");
            assert_eq!(run(&d, &walk(30, 1, false), &Default::default()).unwrap_err(), "needs a second instrument");
            let mut full = walk(300, 2, false);
            full.second = Some(walk(300, 3, false).close);
            let want = run(&d, &full, &Default::default()).unwrap();
            for cut in [37usize, 150, 299] {
                let mut pre = crate::lab::testdata::cut(&full, cut);
                pre.second = Some(full.second.as_ref().unwrap()[..cut].to_vec());
                let got = run(&d, &pre, &Default::default()).unwrap();
                for i in 0..cut { assert!(close_eq(got[0][i], want[0][i]), "{id} bar {i} changes when later bars exist"); }
            }
            assert!(want[0][..d.first[0]].iter().all(|v| v.is_nan()) && !want[0][d.first[0]].is_nan(), "{id} first value at {}", d.first[0]);
        }
    }

    #[test] fn malformed_pair_vectors_dates_and_infinite_values_fail_instead_of_truncating() {
        for id in ["ratio","spread","correlation","correlation_log"] {
            let def=get(id).unwrap();
            for b in [pair(&[1.,2.,3.],&[1.,2.]),pair(&[1.,2.,3.],&[1.,2.,3.,4.]),pair(&[1.,f64::INFINITY,3.],&[1.,2.,3.]),pair(&[1.,2.,3.],&[1.,f64::NEG_INFINITY,3.])] {
                assert!(run(&def,&b,&Default::default()).is_err(),"{id}");
            }
            let mut b=pair(&[1.,2.,3.],&[1.,2.,3.]);b.date[1]=b.date[0].clone();assert!(run(&def,&b,&Default::default()).is_err());
        }
        assert!(pearson(&[1.,2.],&[1.],2).is_err());assert!(pearson(&[1.,2.],&[1.,2.],1).is_err());
    }
    #[test] fn finite_arithmetic_overflow_is_an_error_and_log_return_division_is_stable() {
        for (id,a,b) in [("ratio",1e308,1e-308),("spread",1e308,-1e308)] {
            assert!(run(&get(id).unwrap(),&pair(&[a],&[b]),&Default::default()).is_err(),"{id}");
        }
        assert!(pearson(&[1e308,-1e308],&[1.,2.],2).is_err());
        let b=pair(&[1e-300,1e300,1.],&[1e-200,1e200,1.]);
        let v=go("correlation_log",&b,&[("length",2.)]);assert!(v[0][2].is_finite());
    }
    #[test] fn rolling_correlation_matches_an_independent_two_pass_reference() {
        let b=walk(750,73,false);let y=walk(750,91,false).close;
        for n in [2,3,20,200,2000] {
            let got=pearson(&b.close,&y,n).unwrap();
            for i in 0..b.len() {if i+1<n {assert!(got[i].is_nan());continue;}
                let (x,y)=(&b.close[i+1-n..=i],&y[i+1-n..=i]);let (mx,my)=(x.iter().sum::<f64>()/n as f64,y.iter().sum::<f64>()/n as f64);
                let cov=x.iter().zip(y).map(|(x,y)|(x-mx)*(y-my)).sum::<f64>();
                let sx=x.iter().map(|x|(x-mx).powi(2)).sum::<f64>().sqrt();let sy=y.iter().map(|y|(y-my).powi(2)).sum::<f64>().sqrt();
                assert!(close_eq(got[i],cov/sx/sy),"n{n} i{i}: {}",got[i]);
            }
        }
    }
    #[test] fn pearson_handles_large_offsets_without_raw_sum_cancellation() {
        let x:Vec<_>=(0..5000).map(|i|1e12+i as f64).collect();let y:Vec<_>=x.iter().map(|x|3.*x+7.).collect();
        let v=pearson(&x,&y,2000).unwrap();assert!(v[..1999].iter().all(|x|x.is_nan()));assert!(v[1999..].iter().all(|x|(*x-1.).abs()<1e-10));
    }
    #[test] fn published_cc_worked_example_is_a_formula_fixture_not_a_platform_export() {
        // TradingView's public CC help-page raw table, not an exported TV parity fixture.
        // Independently recomputing centered Pearson from those prices gives0.9271156377916231;
        // the page's displayed0.9432 uses inconsistent intermediate arithmetic and is not our expected value.
        // https://www.tradingview.com/support/solutions/43000502022-correlation-coefficient-cc/
        let x=[170.66,170.95,170.70,169.73,169.18,169.80,169.31,169.11,169.61,168.74,166.38,165.83];
        let y=[56.54,56.40,56.10,55.49,55.30,54.83,54.52,54.09,54.29,54.15,53.29,51.83];
        assert!((pearson(&x,&y,12).unwrap()[11]-0.9271156377916231).abs()<1e-12);
    }
}
