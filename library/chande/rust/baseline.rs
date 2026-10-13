//! Chande 1997, printed pp.75–80 / supplied PDF pp.91–96.
//! Source-rule diagnostic only: no production registration or performance claims.
//! Input: strictly ordered ISO dates, observed open and close, tab-separated.
use std::io::{self, Read};

#[derive(Clone, Debug)]
struct Bar { date: String, open: f64, close: f64 }
#[derive(Debug, PartialEq)]
struct Fill { direction: i8, signal: usize, entry: usize, exit_signal: usize, exit: usize, open: bool }

fn baseline(bars: &[Bar], length: usize, confirmations: usize, from: usize) -> Result<Vec<Fill>, String> {
    if length == 0 || confirmations == 0 || from >= bars.len() {
        return Err("Invalid length, confirmations or start index".into());
    }
    if bars.iter().any(|b| !b.open.is_finite() || b.open <= 0.0 || !b.close.is_finite() || b.close <= 0.0)
        || bars.windows(2).any(|w| w[0].date >= w[1].date) {
        return Err("Missing/invalid observed prices or unordered/duplicate dates".into());
    }
    let mut fills = Vec::new();
    let mut position: Option<Fill> = None;
    let (mut run_dir, mut streak, mut sum) = (0i8, 0usize, 0.0);
    for i in 0..bars.len() {
        sum += bars[i].close;
        if i >= length { sum -= bars[i-length].close; }
        if i+1 < length { continue; }
        // Each close compares with its OWN completed SMA, including that close.
        let ma = sum / length as f64;
        let dir = if bars[i].close > ma { 1 } else if bars[i].close < ma { -1 } else { 0 };
        if dir == 0 { run_dir=0; streak=0; }
        else if dir == run_dir { streak += 1; }
        else { run_dir=dir; streak=1; }
        // Warm-up informs indicators/streaks; orders start inside the requested window.
        if i < from || i+1 == bars.len() || streak < confirmations || dir == 0 { continue; }
        if position.as_ref().is_some_and(|f| f.direction == dir) { continue; }
        if let Some(mut old) = position.take() {
            old.exit_signal=i; old.exit=i+1; old.open=false;
            fills.push(old);
        }
        // No close fills, stops, pyramiding, fixed-dollar conversions or made-up roll events.
        position=Some(Fill { direction:dir, signal:i, entry:i+1,
            exit_signal:i, exit:i+1, open:true });
    }
    if let Some(mut last) = position {
        last.exit=bars.len()-1; last.exit_signal=last.exit; last.open=true;
        fills.push(last);
    }
    Ok(fills)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let length = args.get(1).map_or(Ok(65), |v| v.parse::<usize>())?;
    let confirmations = args.get(2).map_or(Ok(3), |v| v.parse::<usize>())?;
    let from = args.get(3).map_or(Ok(0), |v| v.parse::<usize>())?;
    let mut text=String::new(); io::stdin().read_to_string(&mut text)?;
    let mut bars=Vec::new();
    for line in text.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len()!=3 { return Err("Expected date<TAB>observed open<TAB>observed close".into()); }
        let date=fields[0];
        if date.len()!=10 || date.as_bytes()[4]!=b'-' || date.as_bytes()[7]!=b'-'
            || !date.bytes().enumerate().all(|(i,b)| i==4 || i==7 || b.is_ascii_digit()) {
            return Err("Expected ISO trading date".into());
        }
        bars.push(Bar { date:date.into(), open:fields[1].parse()?, close:fields[2].parse()? });
    }
    println!("direction\tsignal_date\tentry_date\tentry_price\texit_signal_date\texit_date\texit_price\topen");
    for f in baseline(&bars,length,confirmations,from)? {
        let exit_price=if f.open { bars[f.exit].close } else { bars[f.exit].open };
        println!("{}\t{}\t{}\t{:.10}\t{}\t{}\t{:.10}\t{}", f.direction,
            bars[f.signal].date,bars[f.entry].date,bars[f.entry].open,
            bars[f.exit_signal].date,bars[f.exit].date,exit_price,f.open);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bars(values: &[f64]) -> Vec<Bar> {
        values.iter().enumerate().map(|(i,&c)| Bar { date:format!("2020-01-{:02}",i+1),open:200.0+i as f64,close:c }).collect()
    }
    #[test]
    fn confirmation_fills_next_open_and_reverses_at_same_observed_open() {
        let b=bars(&[10.,11.,12.,13.,14.,15.,14.,13.,12.,11.,10.]);
        let f=baseline(&b,3,2,0).unwrap();
        assert_eq!(f,vec![Fill{direction:1,signal:3,entry:4,exit_signal:7,exit:8,open:false},
            Fill{direction:-1,signal:7,entry:8,exit_signal:10,exit:10,open:true}]);
        assert_eq!(b[f[0].entry].open,204.0); // deliberately differs from signal close=13
    }
    #[test]
    fn equal_close_interrupts_confirmations() {
        let f=baseline(&bars(&[10.,11.,12.,11.5,13.,14.,15.]),2,2,0).unwrap();
        assert_eq!(f[0].signal,2);
        // [12,11.5] flips direction; sustained rise resumes without a duplicate long order.
        assert_eq!(f.len(),1);
        let f=baseline(&bars(&[10.,11.,11.,12.,13.,14.]),2,2,0).unwrap();
        assert_eq!(f[0].signal,4); // equality on bar2 resets prior +1 streak
    }
    #[test]
    fn final_close_cannot_create_a_future_fill_and_terminal_trade_stays_open() {
        assert!(baseline(&bars(&[10.,11.,12.]),2,2,0).unwrap().is_empty());
        let f=baseline(&bars(&[10.,11.,12.,13.]),2,2,0).unwrap();
        assert_eq!(f.len(),1);assert!(f[0].open);assert_eq!(f[0].entry,f[0].exit);
    }
    #[test]
    fn each_day_uses_its_own_average_not_the_latest_average_for_all_confirmations() {
        // First two above their averages, then below its own average: no long signal.
        assert!(baseline(&bars(&[10.,11.,12.,11.]),2,3,0).unwrap().is_empty());
    }
    #[test]
    fn warmup_does_not_carry_a_presample_position() {
        let f=baseline(&bars(&[10.,11.,12.,13.,14.,15.]),3,2,4).unwrap();
        assert_eq!(f[0].signal,4);assert_eq!(f[0].entry,5);
    }
    #[test]
    fn rejects_gaps_and_unordered_dates_instead_of_synthetic_prices() {
        let mut b=bars(&[10.,11.,12.,13.]);b[1].close=f64::NAN;
        assert!(baseline(&b,2,2,0).is_err());
        let mut b=bars(&[10.,11.,12.,13.]);b[1].date=b[0].date.clone();
        assert!(baseline(&b,2,2,0).is_err());
        assert!(baseline(&bars(&[10.,11.]),0,3,0).is_err());
        assert!(baseline(&bars(&[10.,11.]),2,0,0).is_err());
    }
}
