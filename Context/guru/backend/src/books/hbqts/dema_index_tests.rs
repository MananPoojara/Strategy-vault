//! Synthetic source-rule checks; these are not market observations or book results.
use super::*;

const ID: &str = "hbqts_dema16_8";
fn fixture(close: &[f64]) -> Vec<crate::Row> {
    close.iter().enumerate().map(|(i,&c)| crate::Row {
        ticker: "SYNTHETIC_INDEX".into(),
        date: format!("2001-{:02}-{:02}",1+i/28,1+i%28),
        open:c, high:c+1., low:(c-1.).max(c/2.), close:c, volume:None,
    }).collect()
}
fn events(values: &[Signal], buy: bool) -> Vec<usize> {
    values.iter().enumerate().filter_map(|(i,s)|
        (if buy {s.raw_buy} else {s.sell}).then_some(i)).collect()
}

#[test]
fn source_first_close_seed_has_no_invented_sma_warmup() {
    // The dated official DEMA loop initializes BOTH smoothing states at C[0].
    let rows=fixture(&[100.,99.]); let x=Ctx::new(&rows,&rows,&[]);
    let s=signals(ID,&x,1,&Params::new()).expect("Figure A.7 reviewed signal path");
    assert_eq!(events(&s,true),vec![1]);
    assert!(events(&s,false).is_empty());
    assert!(!s[0].buy && !s[0].sell);
    let t=super::super::run(ID,&x,1,1,&Params::new()).unwrap();
    assert_eq!(t.len(),1);
    assert_eq!((t[0].legs[0].i,t[0].exit_i,t[0].entry_kind,t[0].open),(1,1,"close",true));
    assert_eq!(t[0].legs[0].px,99.);
    assert_eq!(t[0].exit_px,99.);
}

#[test]
fn source_sell_exrem_follows_raw_clock_and_resets_on_buy() {
    let rows=fixture(&[100.,99.,101.,101.,101.,98.,102.]);
    let x=Ctx::new(&rows,&rows,&[]);
    let s=signals(ID,&x,6,&Params::new()).unwrap();
    assert_eq!(events(&s,true),vec![1,5]);
    assert_eq!(events(&s,false),vec![2,6]);
    assert!(s.iter().all(|s|!(s.raw_buy&&s.sell)));
    let t=super::run(ID,&x,0,6,&Params::new()).unwrap();
    assert_eq!(t.iter().map(|t|(t.legs[0].i,t.exit_i,t.open)).collect::<Vec<_>>(),vec![(1,2,false),(5,6,false)]);
    // The first position is not inherited when the selected account starts later.
    let flat=super::run(ID,&x,2,6,&Params::new()).unwrap();
    assert_eq!(flat.iter().map(|t|(t.legs[0].i,t.exit_i)).collect::<Vec<_>>(),vec![(5,6)]);
}

#[test]
fn source_constants_equality_and_prefixes_do_not_fabricate_signals() {
    let rows=fixture(&vec![0.1283481;70]); let x=Ctx::new(&rows,&rows,&[]);
    let s=signals(ID,&x,69,&Params::new()).unwrap();
    assert!(s.iter().all(|s|!s.buy&&!s.sell&&!s.raw_buy));
    assert_eq!(double(&x.c,16),vec![0.1283481;70]);
    assert_eq!(double(&x.c,8),vec![0.1283481;70]);
    let rows=fixture(&[100.,99.,101.,101.,101.,98.,102.]);
    let x=Ctx::new(&rows,&rows,&[]);let full=signals(ID,&x,6,&Params::new()).unwrap();
    for end in 0..=6 {
        let prefix=signals(ID,&x,end,&Params::new()).unwrap();
        assert_eq!(events(&prefix,true),events(&full[..=end],true));
        assert_eq!(events(&prefix,false),events(&full[..=end],false));
    }
}
