//! Dated membership inputs for the team's explicitly mapped cash databases.
//! AUX is an entry eligibility rule, not an automatic exit on leaving a basket.
#![allow(dead_code)]
use std::collections::BTreeMap;
use crate::Row;

#[derive(Clone,Copy,Debug,PartialEq)]
pub(crate) struct Aux { pub aux1:f32, pub aux2:f32, pub open_interest:f32 }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum Rule { All, Aux1, Aux2, Aux1AndAux2 }
impl Rule {
    pub(crate) fn allows(self,a:Aux)->bool {
        match self {Self::All=>true,Self::Aux1=>a.aux1==1.,Self::Aux2=>a.aux2==1.,Self::Aux1AndAux2=>a.aux1==1. && a.aux2==1.}
    }
}

/// `Ctx::extra` key of a stock's dated AUX2 from the team's C&P cash database (AUX2 = 1: an F&O stock on that date).
/// Strategies that read AUX2 in their own AFL list it in `needs_series`, so readiness reports needs_data without it.
pub(crate) const AUX2_SERIES:&str="AUX2 (dated F&O membership, native C&P AmiBroker database)";

/// The C&P cash databases whose AUX2 is F&O membership (Database Directory: "Use to Run on FO Stocks but Cash Prices").
pub(crate) const AUX2_FO_DATABASES:[&str;2]=["cp_updated_2026","cp_old_2022"];

/// Strategies whose own AFL reads AUX2; a single-stock context loads AUX2 only for these (or for a summary of all).
pub(crate) fn reads_aux2(id:&str)->bool {crate::books::yttr::FO_IDS.contains(&id)}

/// Single-stock hook: the dated AUX2 of a native C&P cash stock, read from `<native_root>/<its C&P database>` through
/// `Catalogue::yt_aux` (original-source SHA-256 and exact date/open/close alignment). Err when unavailable.
pub(crate) fn native_aux2(catalogue:&crate::baskets::Catalogue,key:&str,native_root:Option<&std::path::Path>,rows:&[crate::Row])->Result<Vec<f64>,String>{
    let source=catalogue.native_source(key).ok_or("Unknown qualified basket symbol")?;
    let db=AUX2_FO_DATABASES.into_iter().find(|db|source==format!("Team AmiBroker database {db}")).ok_or("AUX2 = F&O membership is established only for the team's C&P cash databases")?;
    let root=native_root.ok_or("AMIBROKER_DATA_DIR is not set")?;
    Ok(catalogue.yt_aux(key,&root.join(db),rows)?.into_iter().map(|a|a.aux2 as f64).collect())
}

/// The source-specific entry decision must be gated before its internal state
/// advances. Other strategies remain unavailable until individually adapted.
pub(crate) fn strategy_trades(catalogue:&crate::baskets::Catalogue,key:&str,native_root:&std::path::Path,id:&str,x:&crate::connors::Ctx<'_>,from:usize,to:usize,params:&crate::connors::Params,rule:Rule)->Result<Vec<crate::connors::Trade>,String>{
    if !crate::books::ytnr7::IDS.contains(&id) && !crate::books::ytcr::IDS.contains(&id) {return Err("AUX entry-state support has not been reviewed for this strategy".into());}
    let aux=catalogue.yt_aux(key,native_root,x.r)?;
    let eligible:Vec<_>=aux.iter().map(|&a|rule.allows(a)).collect();
    if crate::books::ytnr7::IDS.contains(&id) {
        let s=crate::books::ytnr7::source_signals_with_eligibility(id,x,params,Some(&eligible)).ok_or("NR7 source signals unavailable")?;
        Ok(crate::books::ytnr7::fills(x,&s,from,to,"close"))
    } else {
        let turnover_on=params.get("turnover").is_some_and(|v|v.round()==1.);
        if turnover_on && aux.iter().any(|a|!a.open_interest.is_finite()) {return Err("The source OpenInt turnover field is missing or invalid".into());}
        let turnover:Vec<_>=aux.iter().map(|a|a.open_interest as f64).collect();
        crate::books::ytcr::run_with_eligibility(id,x,from,to,params,Some(&eligible),Some(&turnover)).ok_or_else(||"Candle Range source signals unavailable".into())
    }
}

/// Caller authenticates bytes against the catalogue source hash. Align exact
/// dates, open and close to the derived series; never forward-fill membership.
pub(crate) fn align(bytes:&[u8],rows:&[Row])->Result<Vec<Aux>,String>{
    const HEADER:usize=1184; const REC:usize=40;
    if bytes.len()<HEADER || &bytes[..8]!=b"BROKDAt5" {return Err("Invalid AmiBroker source format".into());}
    let count=u32::from_le_bytes(bytes[1180..1184].try_into().unwrap()) as usize;
    let end=HEADER.checked_add(count.checked_mul(REC).ok_or("Invalid source record count")?).ok_or("Invalid source record count")?;
    if count>100_000 || end>bytes.len() || bytes.len()-end>4 {return Err("Truncated or invalid AmiBroker source records".into());}
    let mut source=BTreeMap::new();
    for record in bytes[HEADER..end].chunks_exact(REC) {
        let d=u64::from_le_bytes(record[..8].try_into().unwrap());
        let date=format!("{:04}-{:02}-{:02}",d>>52,(d>>48)&15,(d>>43)&31);
        if !crate::baskets::valid_date(&date) {continue;}
        let f=|offset|f32::from_le_bytes(record[offset..offset+4].try_into().unwrap());
        let (c,o,oi,a1,a2)=(f(8),f(12),f(28),f(32),f(36));
        if source.insert(date,(o,c,Aux{aux1:a1,aux2:a2,open_interest:oi})).is_some(){return Err("Duplicate dated AUX records".into());}
    }
    let mut aligned=Vec::with_capacity(rows.len());
    for row in rows {
        let &(open,close,aux)=source.get(&row.date).ok_or("AUX is missing for a stock price date")?;
        if !open.is_finite() || !close.is_finite() || !aux.aux1.is_finite() || !aux.aux2.is_finite() || (open as f64-row.open).abs()>0.00011 || (close as f64-row.close).abs()>0.00011 {return Err("AUX source and dated stock prices do not match".into());}
        aligned.push(aux);
    }
    Ok(aligned)
}

#[cfg(test)] mod tests {
    use super::*;
    fn row(date:&str)->Row { Row{ticker:"A".into(),date:date.into(),open:10.,high:11.,low:9.,close:10.,volume:None} }
    fn source()->Vec<u8> {
        let mut b=vec![0u8;1184+80+4];b[..8].copy_from_slice(b"BROKDAt5");b[1180..1184].copy_from_slice(&2u32.to_le_bytes());
        for (i,(day,a1,a2)) in [(2u64,1f32,0f32),(3,1.,1.)].iter().enumerate(){let k=1184+40*i; let date=(2024u64<<52)|(1u64<<48)|(day<<43);b[k..k+8].copy_from_slice(&date.to_le_bytes());for (offset,val) in [(8,10f32),(12,10.),(28,12_345_678.),(32,*a1),(36,*a2)] {b[k+offset..k+offset+4].copy_from_slice(&val.to_le_bytes());}}
        b
    }
    #[test] fn aligns_dated_membership_and_combines_conditions() {
        let a=align(&source(),&[row("2024-01-02"),row("2024-01-03")]).unwrap();
        assert!(Rule::Aux1.allows(a[0]));assert!(!Rule::Aux1AndAux2.allows(a[0]));assert!(Rule::Aux1AndAux2.allows(a[1]));
        assert!(!Rule::Aux2.allows(a[0]));assert!(Rule::All.allows(a[0]));
        assert_eq!(a[0].open_interest,12_345_678.);
    }
    /// Single-stock hook on the local C&P copy (skipped when the local basket index or AmiBroker copy is absent).
    #[test] fn local_single_stock_aux2_feeds_the_fo_afls() {
        let base=std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../jay-local/Data"));
        let (baskets,native)=(base.join("baskets/updated"),base.join("amibroker"));
        if !baskets.join("index.csv").is_file() || !native.join("cp_updated_2026").is_dir() {eprintln!("skipped: local data absent");return;}
        let cat=crate::baskets::Catalogue::load(&baskets).unwrap();
        for sym in ["SBIN","RELIANCE"] {
            let key=format!("basket:nse500:{sym}");
            let rows=cat.rows(&key).unwrap();
            assert!(native_aux2(&cat,&key,None,&rows).unwrap_err().contains("AMIBROKER_DATA_DIR"));
            let aux2=native_aux2(&cat,&key,Some(&native),&rows).unwrap();
            assert_eq!(aux2.len(),rows.len());
            let mut ctx=crate::connors::Ctx::new(&rows,&rows,&[]);
            let def=|id:&str|crate::connors::catalogue().into_iter().find(|d|d.id==id).unwrap();
            let p=crate::connors::Params::new();
            let to=rows.len()-1;
            assert_eq!(crate::readiness::check(&ctx,&def(crate::books::yttr::ATHC_FO),0,to,&p).unwrap().status,"needs_data");
            ctx.extra.insert(AUX2_SERIES.into(),aux2.clone());
            let trail:crate::connors::Params=[("rsi_trail".to_string(),20.),("hard_sl".to_string(),1.)].into_iter().collect();
            for id in crate::books::yttr::FO_IDS {
                assert!(crate::readiness::check(&ctx,&def(id),0,to,&p).is_none(),"{id}");
                for (label,q) in [("defaults",&p),("rsi_trail 20, hard SL on",&trail)] {
                    let t=crate::connors::run(id,&ctx,0,to,q).unwrap();
                    for t in &t {assert_eq!(aux2[t.legs[0].i],1.);}
                    eprintln!("single-stock {key} {id} {label}: {} trades ({} open)",t.len(),t.iter().filter(|t|t.open).count());
                    assert!(!t.is_empty());
                }
            }
        }
        // the F&O price database (allfo) is not the C&P cash database: no AUX2 mapping, so needs_data
        let key="basket:allfo:SBIN";
        let rows=cat.rows(key).unwrap();
        assert!(native_aux2(&cat,key,Some(&native),&rows).is_err());
    }
    #[test] fn rejects_missing_mismatched_or_truncated_source() {
        assert!(align(&source(),&[row("2024-01-04")]).is_err());
        let mut wrong=row("2024-01-02");wrong.close=12.;assert!(align(&source(),&[wrong]).is_err());
        assert!(align(&source()[..1200],&[row("2024-01-02")]).is_err());
    }
}
