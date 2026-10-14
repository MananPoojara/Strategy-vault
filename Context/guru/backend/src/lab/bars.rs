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
}

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
        }
    }
    pub fn len(&self) -> usize { self.close.len() }
    pub fn hl2(&self) -> Vec<f64> { (0..self.len()).map(|i| (self.high[i] + self.low[i]) / 2.).collect() }
    pub fn hlc3(&self) -> Vec<f64> { (0..self.len()).map(|i| (self.high[i] + self.low[i] + self.close[i]) / 3.).collect() }
    pub fn ohlc4(&self) -> Vec<f64> { (0..self.len()).map(|i| (self.open[i] + self.high[i] + self.low[i] + self.close[i]) / 4.).collect() }
}
