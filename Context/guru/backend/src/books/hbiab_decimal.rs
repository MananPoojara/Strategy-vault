//! Exact finite-decimal comparisons of round-trip loaded prices and cost rates.
//! This is arithmetic, not a tolerance or a market/ticker-specific signal rule.
use std::cmp::Ordering;
const BASE: u64 = 1_000_000_000;
#[derive(Clone, Debug)]
struct Natural(Vec<u32>);
impl Natural {
    fn zero() -> Self {
        Self(vec![0])
    }
    fn from_digits(d: &str) -> Self {
        let mut v = Vec::new();
        let mut end = d.len();
        while end > 0 {
            let begin = end.saturating_sub(9);
            v.push(d[begin..end].parse().unwrap());
            end = begin;
        }
        let mut n = Self(v);
        n.trim();
        n
    }
    fn trim(&mut self) {
        while self.0.len() > 1 && self.0.last() == Some(&0) {
            self.0.pop();
        }
    }
    fn cmp(&self, b: &Self) -> Ordering {
        self.0
            .len()
            .cmp(&b.0.len())
            .then_with(|| self.0.iter().rev().cmp(b.0.iter().rev()))
    }
    fn add(&self, b: &Self) -> Self {
        let mut out = Vec::new();
        let mut carry = 0u64;
        for i in 0..self.0.len().max(b.0.len()) {
            let v = self.0.get(i).copied().unwrap_or(0) as u64
                + b.0.get(i).copied().unwrap_or(0) as u64
                + carry;
            out.push((v % BASE) as u32);
            carry = v / BASE;
        }
        if carry > 0 {
            out.push(carry as u32);
        }
        Self(out)
    }
    fn sub(&self, b: &Self) -> Self {
        assert!(self.cmp(b) != Ordering::Less);
        let mut out = Vec::new();
        let mut borrow = 0i64;
        for i in 0..self.0.len() {
            let v = self.0[i] as i64 - b.0.get(i).copied().unwrap_or(0) as i64 - borrow;
            if v < 0 {
                out.push((v + BASE as i64) as u32);
                borrow = 1;
            } else {
                out.push(v as u32);
                borrow = 0;
            }
        }
        let mut n = Self(out);
        n.trim();
        n
    }
    fn mul(&self, b: &Self) -> Self {
        let mut out = vec![0u32; self.0.len() + b.0.len() + 1];
        for (i, &a) in self.0.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &b) in b.0.iter().enumerate() {
                let k = i + j;
                let v = out[k] as u64 + a as u64 * b as u64 + carry;
                out[k] = (v % BASE) as u32;
                carry = v / BASE;
            }
            let mut k = i + b.0.len();
            while carry > 0 {
                let v = out[k] as u64 + carry;
                out[k] = (v % BASE) as u32;
                carry = v / BASE;
                k += 1;
            }
        }
        let mut n = Self(out);
        n.trim();
        n
    }
    fn pow10(&self, n: usize) -> Self {
        let mut words = vec![0; n / 9];
        words.extend(&self.0);
        Self(words).mul(&Self(vec![10u32.pow((n % 9) as u32)]))
    }
    fn digits(&self) -> String {
        let mut s = self.0.last().unwrap().to_string();
        for w in self.0.iter().rev().skip(1) {
            s.push_str(&format!("{w:09}"));
        }
        s
    }
}
#[derive(Clone, Debug)]
pub struct Decimal {
    coefficient: Natural,
    exponent: i32,
}
impl Decimal {
    pub fn zero() -> Self {
        Self {
            coefficient: Natural::zero(),
            exponent: 0,
        }
    }
    pub fn new(value: f64) -> Self {
        assert!(value.is_finite() && value >= 0.);
        if value == 0. {
            return Self::zero();
        }
        let text = value.to_string();
        let (m, e) = text
            .split_once(['e', 'E'])
            .map_or((text.as_str(), 0), |(m, e)| (m, e.parse().unwrap()));
        let (whole, fraction) = m.split_once('.').unwrap_or((m, ""));
        let digits = format!("{whole}{fraction}");
        let significant = digits.trim_end_matches('0');
        if significant.is_empty() {
            return Self::zero();
        }
        Self {
            coefficient: Natural::from_digits(significant),
            exponent: e - fraction.len() as i32 + (digits.len() - significant.len()) as i32,
        }
    }
    fn aligned(&self, b: &Self) -> (Natural, Natural, i32) {
        let e = self.exponent.min(b.exponent);
        (
            self.coefficient.pow10((self.exponent - e) as usize),
            b.coefficient.pow10((b.exponent - e) as usize),
            e,
        )
    }
    pub fn add(&self, b: &Self) -> Self {
        let (a, b, e) = self.aligned(b);
        Self {
            coefficient: a.add(&b),
            exponent: e,
        }
    }
    pub fn mul(&self, b: &Self) -> Self {
        Self {
            coefficient: self.coefficient.mul(&b.coefficient),
            exponent: self.exponent + b.exponent,
        }
    }
    pub fn cmp(&self, b: &Self) -> Ordering {
        let (a, b, _) = self.aligned(b);
        a.cmp(&b)
    }
    pub fn difference(&self, b: &Self) -> f64 {
        let (a, b, e) = self.aligned(b);
        let (n, negative) = match a.cmp(&b) {
            Ordering::Equal => return 0.,
            Ordering::Greater => (a.sub(&b), false),
            Ordering::Less => (b.sub(&a), true),
        };
        format!("{}{}e{}", if negative { "-" } else { "" }, n.digits(), e)
            .parse()
            .unwrap()
    }
}
pub fn difference(a: f64, b: f64) -> f64 {
    Decimal::new(a).difference(&Decimal::new(b))
}
pub fn net_per_unit(entry: f64, exit: f64, cost: f64) -> f64 {
    net_contribution(entry, exit, &Decimal::new(cost), 1.)
}
pub fn rate(fees_bps: f64, slippage_bps: f64) -> Decimal {
    Decimal::new(fees_bps)
        .add(&Decimal::new(slippage_bps))
        .mul(&Decimal::new(0.0001))
}
pub fn gross_contribution(entry: f64, exit: f64, quantity: f64) -> f64 {
    let q = Decimal::new(quantity);
    Decimal::new(exit)
        .mul(&q)
        .difference(&Decimal::new(entry).mul(&q))
}
pub fn net_contribution(entry: f64, exit: f64, cost: &Decimal, quantity: f64) -> f64 {
    let e = Decimal::new(entry);
    let x = Decimal::new(exit);
    let q = Decimal::new(quantity);
    x.mul(&q).difference(&e.add(&e.add(&x).mul(cost)).mul(&q))
}

pub fn mean_order(prices: &[Decimal], i: usize, a: usize, b: usize) -> Ordering {
    let sum = |n: usize| {
        prices[i + 1 - n..=i]
            .iter()
            .fold(Decimal::zero(), |v, p| v.add(p))
    };
    sum(a)
        .mul(&Decimal::new(b as f64))
        .cmp(&sum(b).mul(&Decimal::new(a as f64)))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_price_cost_break_even_is_exact() {
        assert_eq!(net_per_unit(439.78, 440.22, 0.0005), 0.);
        assert!(net_per_unit(439.78, 440.2200001, 0.0005) > 0.);
        assert!(net_per_unit(439.78, 440.2199999, 0.0005) < 0.);
    }
    #[test]
    fn decimal_arithmetic_has_no_epsilon_window() {
        assert_eq!(difference(0.3, 0.1), 0.2);
        assert!(difference(0.30000000000000004, 0.3) > 0.);
        assert_eq!(
            Decimal::new(1e308).cmp(&Decimal::new(1e-308)),
            Ordering::Greater
        );
        assert!(net_per_unit(1., 1., 1e-100) < 0.);
        assert!(net_contribution(1e-308, 1e-308, &Decimal::new(1e-100), 1e308) < 0.);
        assert_eq!(Decimal::new(-0.0).cmp(&Decimal::new(0.)), Ordering::Equal);
        assert_eq!(net_contribution(999.97, 1000.03, &rate(0.1, 0.2), 1.), 0.);
    }
}
