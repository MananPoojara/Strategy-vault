use crate::Row;

/// Columnar OHLCV. `end[i]` = index of the last source (daily) bar inside bar i.
#[derive(Clone, Debug, Default)]
pub struct Bars {
    pub date: Vec<String>,
    pub open: Vec<f64>,
    pub high: Vec<f64>,
    pub low: Vec<f64>,
    pub close: Vec<f64>,
    pub volume: Option<Vec<f64>>,
    pub end: Vec<usize>,
    /// Daily breadth of the market (from extra_data `<MARKET>_ADV_DEC.csv`); None on W/M bars and markets without it.
    pub breadth: Option<Breadth>,
    /// Pair indicator vs close aligned to these daily bars; absent outside pair computation.
    pub second: Option<Vec<f64>>,
    /// Published contract metadata, aligned daily; absent for cash bars.
    pub expiry: Option<Vec<String>>, pub expiry_sessions: Option<Vec<f64>>,
    /// Volume missing only on the first bars (NIFTY 50 before 2000): (first bar with volume, volume from that bar).
    /// `volume` stays None, so whatever needs volume on every bar is unchanged; catalogue::run starts volume
    /// indicators at that bar, as AmiBroker computes from the bars that have volume.
    pub late_volume: Option<(usize, Vec<f64>)>,
}

/// Market breadth of the instrument's market, aligned to its daily bars (NaN = no value that day).
#[derive(Clone, Debug, Default)]
pub struct Breadth {
    pub adv: Vec<f64>, pub dec: Vec<f64>, pub unch: Vec<f64>, pub adv_vol: Vec<f64>, pub dec_vol: Vec<f64>, pub unch_vol: Vec<f64>,
    /// Advance/decline ratio of the index's own member stocks (extra_data `<TICKER>__MEMBERS_AD_RATIO.csv`).
    pub members_ratio: Option<Vec<f64>>,
    /// The A/D ratio must come from the index's members: NSE counts span the whole exchange, not one index.
    pub members_required: bool,
}

impl Bars {
    pub fn from_rows(rows: &[Row]) -> Bars {
        let volume = if !rows.is_empty() && rows.iter().all(|r| r.volume.is_some()) {
            Some(rows.iter().map(|r| r.volume.unwrap()).collect())
        } else {
            None
        };
        let late_volume = match rows.iter().position(|r| r.volume.is_some()) {
            Some(k) if k > 0 && rows[k..].iter().all(|r| r.volume.is_some()) => Some((k, rows[k..].iter().map(|r| r.volume.unwrap()).collect())),
            _ => None,
        };
        Bars {
            date: rows.iter().map(|r| r.date.clone()).collect(),
            open: rows.iter().map(|r| r.open).collect(),
            high: rows.iter().map(|r| r.high).collect(),
            low: rows.iter().map(|r| r.low).collect(),
            close: rows.iter().map(|r| r.close).collect(),
            volume,
            end: (0..rows.len()).collect(),
            breadth: None, second: None, expiry: None, expiry_sessions: None, late_volume,
        }
    }
    /// Volume on every bar, or the late volume with the bars before it empty (NaN).
    pub fn volume_or_late(&self) -> Option<Vec<f64>> {
        self.volume.clone().or_else(|| self.late_volume.as_ref().map(|(k, v)| std::iter::repeat_n(f64::NAN, *k).chain(v.iter().copied()).collect()))
    }
    /// Bars k.. on their own with the late volume (plain OHLCV only: catalogue::run uses it for indicators that need
    /// nothing else).
    pub fn late_volume_bars(&self) -> Option<(usize, Bars)> {
        let (k, v) = self.late_volume.as_ref()?;
        let s = |x: &[f64]| x[*k..].to_vec();
        Some((*k, Bars { date: self.date[*k..].to_vec(), open: s(&self.open), high: s(&self.high), low: s(&self.low), close: s(&self.close),
            volume: Some(v.clone()), end: (0..self.len() - k).collect(), ..Default::default() }))
    }
    /// Daily bars plus the market's advance/decline series when the engine loaded `<MARKET>_ADV_DEC.csv`.
    /// Each engine instance serves one market, so the `_ADV_DEC.` suffix is unique.
    pub fn from_ctx(ctx: &crate::connors::Ctx) -> Bars {
        let mut b = Bars::from_rows(ctx.r);
        let key = |c: &str| ctx.extra.keys().find(|k| k.ends_with(&format!("_ADV_DEC.{c}"))).cloned();
        let col = |c: &str| key(c).map(|k| ctx.extra[&k].clone());
        // gl/crypto ids are "EXCHANGE:SYMBOL"; a wildcard extra file must not hand them an India/US market's breadth.
        let gl = ctx.r.first().is_some_and(|r| r.ticker.contains(':'));
        if let (false, Some(adv), Some(dec)) = (gl, col("Advances"), col("Declines")) {
            let nan = || vec![f64::NAN; adv.len()];
            b.breadth = Some(Breadth { unch: col("Unchanged").unwrap_or_else(nan), adv_vol: col("AdvVolume").unwrap_or_else(nan),
                dec_vol: col("DecVolume").unwrap_or_else(nan), unch_vol: col("UnchVolume").unwrap_or_else(nan), adv, dec,
                members_ratio: ctx.extra.get("MEMBERS_AD_RATIO").cloned(), members_required: key("Advances").is_some_and(|k| k.starts_with("NSE_")) });
        }
        b
    }
    pub fn len(&self) -> usize { self.close.len() }
    pub fn hl2(&self) -> Vec<f64> { (0..self.len()).map(|i| (self.high[i] + self.low[i]) / 2.).collect() }
    pub fn hlc3(&self) -> Vec<f64> { (0..self.len()).map(|i| (self.high[i] + self.low[i] + self.close[i]) / 3.).collect() }
    pub fn oc2(&self) -> Vec<f64> { (0..self.len()).map(|i| (self.open[i] + self.close[i]) / 2.).collect() }
    pub fn ohlc4(&self) -> Vec<f64> { (0..self.len()).map(|i| (self.open[i] + self.high[i] + self.low[i] + self.close[i]) / 4.).collect() }
}

#[cfg(test)]
mod tests {
    use super::Bars;
    /// 60 business days; volume missing on the first `gap` bars (or on bar `hole` in the middle).
    fn rows(gap: usize, hole: Option<usize>) -> Vec<crate::Row> {
        (0..60).map(|i| { let c = 100. + ((i * 7) % 11) as f64 - (i % 3) as f64;
            crate::Row { ticker: "NIFTY 50".into(), date: format!("2024-{:02}-{:02}", 1 + i / 20, 1 + i % 20), open: c - 0.5, high: c + 1., low: c - 1., close: c,
                volume: (i >= gap && Some(i) != hole).then_some(1000. + (i * 37 % 13) as f64 * 100.) } }).collect()
    }
    #[test] fn volume_missing_only_on_the_first_bars_starts_volume_indicators_there() {
        use crate::lab::catalogue::{get, run, Params};
        let (late, full) = (Bars::from_rows(&rows(10, None)), Bars::from_rows(&rows(10, None)[10..]));
        assert!(late.volume.is_none());
        assert_eq!(late.late_volume.as_ref().map(|l| (l.0, l.1.len())), Some((10, 50)));
        for id in ["mfi", "obv", "vwma", "cmf"] {
            let d = get(id).unwrap();
            let (a, b) = (run(&d, &late, &Params::new()).unwrap(), run(&d, &full, &Params::new()).unwrap());
            for (x, y) in a.iter().zip(&b) {
                assert!(x[..10].iter().all(|v| v.is_nan()), "{id}");
                assert!(x[10..].iter().zip(y).all(|(p, q)| p == q || (p.is_nan() && q.is_nan())), "{id}");
            }
        }
        // formulas and the Volume source see the same volume, empty before bar 10
        let v = late.volume_or_late().unwrap();
        assert!(v[..10].iter().all(|x| x.is_nan()) && v[10..] == late.late_volume.as_ref().unwrap().1[..]);
        // weekly: volume from the first week whose days all have it
        let w = crate::lab::resample::resample(&late, crate::lab::resample::Tf::W);
        let (j, wv) = w.late_volume.clone().unwrap();
        let start = |p: usize| if p == 0 { 0 } else { w.end[p - 1] + 1 };
        assert!(start(j) >= 10 && start(j - 1) < 10);
        assert_eq!(wv[0], (w.end[j - 1] + 1..=w.end[j]).map(|i| v[i]).sum::<f64>());
        // a hole after volume starts is still refused, as before
        let holed = Bars::from_rows(&rows(0, Some(30)));
        assert!(holed.volume.is_none() && holed.late_volume.is_none());
        assert_eq!(run(&get("mfi").unwrap(), &holed, &Params::new()).unwrap_err(), "needs volume");
    }
    #[test] fn from_ctx_reads_the_markets_adv_dec_columns() {
        let rows: Vec<crate::Row> = (0..3).map(|i| crate::Row { ticker: "NIFTY 50".into(), date: format!("2024-01-0{}", i + 1), open: 1., high: 1., low: 1., close: 1., volume: None }).collect();
        let mut ctx = crate::connors::Ctx::new(&rows, &rows, &[]);
        assert!(Bars::from_ctx(&ctx).breadth.is_none(), "no ADV_DEC series → no breadth");
        ctx.extra.insert("NSE_ADV_DEC.Advances".into(), vec![1., 2., f64::NAN]);
        ctx.extra.insert("NSE_ADV_DEC.Declines".into(), vec![3., 4., 5.]);
        ctx.extra.insert("ADV".into(), vec![9., 9., 9.]); // single-column file: not a breadth source
        let br = Bars::from_ctx(&ctx).breadth.unwrap();
        let gl: Vec<crate::Row> = rows.iter().map(|r| crate::Row { ticker: "CRYPTO:BTCUSD".into(), ..r.clone() }).collect();
        let mut gctx = crate::connors::Ctx::new(&gl, &gl, &[]);
        gctx.extra.insert("NSE_ADV_DEC.Advances".into(), vec![1.; 3]); gctx.extra.insert("NSE_ADV_DEC.Declines".into(), vec![1.; 3]);
        assert!(Bars::from_ctx(&gctx).breadth.is_none(), "gl/crypto have no breadth");
        assert_eq!((br.adv[1], br.dec[2]), (2., 5.));
        assert!(br.adv[2].is_nan() && br.adv_vol.iter().all(|v| v.is_nan()) && br.unch_vol.len() == 3);
        assert!(br.members_required && br.members_ratio.is_none(), "NSE counts are exchange-wide: an index needs its members' ratio");
        ctx.extra.insert("MEMBERS_AD_RATIO".into(), vec![1.27, f64::NAN, 0.5]);
        assert_eq!(Bars::from_ctx(&ctx).breadth.unwrap().members_ratio.unwrap()[0], 1.27);
        let mut us = crate::connors::Ctx::new(&rows, &rows, &[]);
        us.extra.insert("NYSE_ADV_DEC.Advances".into(), vec![1.; 3]); us.extra.insert("NYSE_ADV_DEC.Declines".into(), vec![1.; 3]);
        assert!(!Bars::from_ctx(&us).breadth.unwrap().members_required, "US keeps the NYSE ratio");
    }
}
