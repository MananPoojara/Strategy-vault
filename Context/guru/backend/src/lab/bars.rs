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
    /// Published contract metadata, aligned daily; absent for cash bars.
    pub expiry: Option<Vec<String>>, pub expiry_sessions: Option<Vec<f64>>,
}

/// Market breadth of the instrument's market, aligned to its daily bars (NaN = no value that day).
#[derive(Clone, Debug, Default)]
pub struct Breadth { pub adv: Vec<f64>, pub dec: Vec<f64>, pub unch: Vec<f64>, pub adv_vol: Vec<f64>, pub dec_vol: Vec<f64>, pub unch_vol: Vec<f64> }

impl Bars {
    pub fn from_rows(rows: &[Row]) -> Bars {
        let volume = if !rows.is_empty() && rows.iter().all(|r| r.volume.is_some()) {
            Some(rows.iter().map(|r| r.volume.unwrap()).collect())
        } else {
            None
        };
        Bars {
            date: rows.iter().map(|r| r.date.clone()).collect(),
            open: rows.iter().map(|r| r.open).collect(),
            high: rows.iter().map(|r| r.high).collect(),
            low: rows.iter().map(|r| r.low).collect(),
            close: rows.iter().map(|r| r.close).collect(),
            volume,
            end: (0..rows.len()).collect(),
            breadth: None, expiry: None, expiry_sessions: None,
        }
    }
    /// Daily bars plus the market's advance/decline series when the engine loaded `<MARKET>_ADV_DEC.csv`.
    /// Each engine instance serves one market, so the `_ADV_DEC.` suffix is unique.
    pub fn from_ctx(ctx: &crate::connors::Ctx) -> Bars {
        let mut b = Bars::from_rows(ctx.r);
        let col = |c: &str| ctx.extra.iter().find(|(k, _)| k.ends_with(&format!("_ADV_DEC.{c}"))).map(|(_, v)| v.clone());
        // gl/crypto ids are "EXCHANGE:SYMBOL"; a wildcard extra file must not hand them an India/US market's breadth.
        let gl = ctx.r.first().is_some_and(|r| r.ticker.contains(':'));
        if let (false, Some(adv), Some(dec)) = (gl, col("Advances"), col("Declines")) {
            let nan = || vec![f64::NAN; adv.len()];
            b.breadth = Some(Breadth { unch: col("Unchanged").unwrap_or_else(nan), adv_vol: col("AdvVolume").unwrap_or_else(nan),
                dec_vol: col("DecVolume").unwrap_or_else(nan), unch_vol: col("UnchVolume").unwrap_or_else(nan), adv, dec });
        }
        b
    }
    pub fn len(&self) -> usize { self.close.len() }
    pub fn hl2(&self) -> Vec<f64> { (0..self.len()).map(|i| (self.high[i] + self.low[i]) / 2.).collect() }
    pub fn hlc3(&self) -> Vec<f64> { (0..self.len()).map(|i| (self.high[i] + self.low[i] + self.close[i]) / 3.).collect() }
    pub fn ohlc4(&self) -> Vec<f64> { (0..self.len()).map(|i| (self.open[i] + self.high[i] + self.low[i] + self.close[i]) / 4.).collect() }
}

#[cfg(test)]
mod tests {
    use super::Bars;
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
    }
}
