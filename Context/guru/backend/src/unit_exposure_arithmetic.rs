//! Scoped exact canonical-decimal unit-return arithmetic. No legacy dispatch.
//! Contract: DEMA-V2-ARITHMETIC-REPORT-CONTRACT-v2 (private receipt9cb09).
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { InvalidDomain, Underflow, Overflow, EquityDomain }
impl Error {
    pub fn code(self) -> &'static str {
        match self { Self::InvalidDomain=>"invalid_domain", Self::Underflow=>"reporting_arithmetic_underflow", Self::Overflow=>"reporting_arithmetic_overflow", Self::EquityDomain=>"unit_exposure_equity_domain" }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.write_str(self.code())}
}
impl std::error::Error for Error {}

/// Unsigned little-endian base2^32; an empty vector is canonical zero.
#[derive(Clone,Debug)]
struct Natural(Vec<u32>);
impl Natural {
    fn zero()->Self{Self(vec![])}
    fn from_u64(v:u64)->Self{let mut n=Self(vec![v as u32,(v>>32)as u32]);n.trim();n}
    fn trim(&mut self){while self.0.last()==Some(&0){self.0.pop();}}
    fn is_zero(&self)->bool{self.0.is_empty()}
    fn cmp(&self,b:&Self)->Ordering{self.0.len().cmp(&b.0.len()).then_with(||self.0.iter().rev().cmp(b.0.iter().rev()))}
    fn add(&self,b:&Self)->Self{
        let mut out=Vec::with_capacity(self.0.len().max(b.0.len())+1);let mut carry=0u64;
        for i in 0..self.0.len().max(b.0.len()){
            let v=self.0.get(i).copied().unwrap_or(0)as u64+b.0.get(i).copied().unwrap_or(0)as u64+carry;
            out.push(v as u32);carry=v>>32;
        }
        if carry!=0{out.push(carry as u32);}Self(out)
    }
    fn sub(&self,b:&Self)->Self{
        assert!(self.cmp(b)!=Ordering::Less);let mut out=Vec::with_capacity(self.0.len());let mut borrow=0i64;
        for i in 0..self.0.len(){let v=self.0[i]as i64-b.0.get(i).copied().unwrap_or(0)as i64-borrow;out.push(v as u32);borrow=if v<0{1}else{0};}
        assert_eq!(borrow,0);let mut n=Self(out);n.trim();n
    }
    fn mul(&self,b:&Self)->Self{
        if self.is_zero()||b.is_zero(){return Self::zero();}
        let mut out=vec![0u32;self.0.len()+b.0.len()+1];
        for (i,&a) in self.0.iter().enumerate(){
            let mut carry=0u128;
            for (j,&b) in b.0.iter().enumerate(){let k=i+j;let v=out[k]as u128+a as u128*b as u128+carry;out[k]=v as u32;carry=v>>32;}
            let mut k=i+b.0.len();while carry!=0{let v=out[k]as u128+carry;out[k]=v as u32;carry=v>>32;k+=1;}
        }
        let mut n=Self(out);n.trim();n
    }
    fn shl(&self,bits:usize)->Self{
        if self.is_zero(){return Self::zero();}
        let mut out=vec![0;bits/32];let shift=bits%32;let mut carry=0u64;
        for &word in &self.0{let v=((word as u64)<<shift)|carry;out.push(v as u32);carry=v>>32;}
        if carry!=0{out.push(carry as u32);}Self(out)
    }
    fn from_digits(s:&str)->Result<Self,Error>{
        if s.is_empty()||!s.bytes().all(|b|b.is_ascii_digit()){return Err(Error::InvalidDomain);}
        let mut n=Self::zero();let ten=Self::from_u64(10);
        for b in s.bytes(){n=n.mul(&ten).add(&Self::from_u64((b-b'0')as u64));}Ok(n)
    }
    fn pow10(n:usize)->Self{let ten=Self::from_u64(10);(0..n).fold(Self::from_u64(1),|v,_|v.mul(&ten))}
}

/// All signs and rational operations are exact; no fixed-precision division.
#[derive(Clone,Debug)]
struct Rational { negative:bool, numerator:Natural, denominator:Natural }
impl Rational {
    fn new(negative:bool,numerator:Natural,denominator:Natural)->Self{
        assert!(!denominator.is_zero());
        if numerator.is_zero(){Self::zero()}else{Self{negative,numerator,denominator}}
    }
    fn zero()->Self{Self{negative:false,numerator:Natural::zero(),denominator:Natural::from_u64(1)}}
    fn integer(v:u64)->Self{Self::new(false,Natural::from_u64(v),Natural::from_u64(1))}
    /// Specifically pinned Rust std Display, not arbitrary decimal serialization.
    fn canonical(v:f64)->Result<Self,Error>{
        if !v.is_finite(){return Err(Error::InvalidDomain);}if v==0.{return Ok(Self::zero());}
        let text=v.to_string();let (negative,text)=text.strip_prefix('-').map_or((false,text.as_str()),|s|(true,s));
        let (mantissa,exponent)=text.split_once(['e','E']).map_or(Ok((text,0i32)),|(m,e)|e.parse::<i32>().map(|e|(m,e)).map_err(|_|Error::InvalidDomain))?;
        let (whole,fraction)=mantissa.split_once('.').unwrap_or((mantissa,""));let digits=format!("{whole}{fraction}");
        let significant=digits.trim_end_matches('0');
        let exponent=exponent-fraction.len()as i32+(digits.len()-significant.len())as i32;
        let coefficient=Natural::from_digits(significant)?;
        Ok(if exponent>=0{Self::new(negative,coefficient.mul(&Natural::pow10(exponent as usize)),Natural::from_u64(1))}else{Self::new(negative,coefficient,Natural::pow10((-exponent)as usize))})
    }
    fn add(&self,b:&Self)->Self{
        if self.numerator.is_zero(){return b.clone();}if b.numerator.is_zero(){return self.clone();}
        let a=self.numerator.mul(&b.denominator);let bb=b.numerator.mul(&self.denominator);let denominator=self.denominator.mul(&b.denominator);
        if self.negative==b.negative{Self::new(self.negative,a.add(&bb),denominator)}else{
            match a.cmp(&bb){Ordering::Equal=>Self::zero(),Ordering::Greater=>Self::new(self.negative,a.sub(&bb),denominator),Ordering::Less=>Self::new(b.negative,bb.sub(&a),denominator)}
        }
    }
    fn sub(&self,b:&Self)->Self{let mut b=b.clone();if !b.numerator.is_zero(){b.negative=!b.negative;}self.add(&b)}
    fn div(&self,b:&Self)->Self{assert!(!b.numerator.is_zero());Self::new(self.negative!=b.negative,self.numerator.mul(&b.denominator),self.denominator.mul(&b.numerator))}
    fn times(&self,k:u8)->Self{Self::new(self.negative,self.numerator.mul(&Natural::from_u64(k as u64)),self.denominator.clone())}
    fn positive(&self)->bool{!self.negative&&!self.numerator.is_zero()}
    /// Compare magnitude N/D *2^shift with exact m*2^exponent.
    fn compare_binary(&self,m:&Natural,exponent:i32,shift:usize)->Ordering{
        if exponent>=0{self.numerator.shl(shift).cmp(&self.denominator.mul(m).shl(exponent as usize))}
        else{self.numerator.shl(shift+(-exponent)as usize).cmp(&self.denominator.mul(m))}
    }
    fn binary(bits:u64)->(Natural,i32){
        let biased=(bits>>52)as i32;let fraction=bits&((1u64<<52)-1);
        if biased==0{(Natural::from_u64(fraction),-1074)}else{(Natural::from_u64(fraction|(1u64<<52)),biased-1023-52)}
    }
    fn compare_bits(&self,bits:u64)->Ordering{let(m,e)=Self::binary(bits);self.compare_binary(&m,e,0)}
    fn round(&self)->Result<f64,Error>{
        const MAX:u64=0x7fefffffffffffff;
        if self.numerator.is_zero(){return Ok(0.);}
        let bits=if self.compare_bits(MAX)==Ordering::Greater{
            // Exact halfway from maximum finite to the next exponent's infinity.
            let threshold=Natural::from_u64(1).shl(1024).sub(&Natural::from_u64(1).shl(970));
            if self.numerator.cmp(&self.denominator.mul(&threshold))!=Ordering::Less{return Err(Error::Overflow);}MAX
        }else{
            let(mut low,mut high)=(0u64,MAX);
            while low<high{let middle=low+(high-low+1)/2;if self.compare_bits(middle)==Ordering::Less{high=middle-1;}else{low=middle;}}
            if self.compare_bits(low)==Ordering::Equal{low}else{
                let upper=low+1;let(a,ea)=Self::binary(low);let(b,eb)=Self::binary(upper);let exponent=ea.min(eb);
                let sum=a.shl((ea-exponent)as usize).add(&b.shl((eb-exponent)as usize));
                match self.compare_binary(&sum,exponent,1){Ordering::Less=>low,Ordering::Greater=>upper,Ordering::Equal=>if low&1==0{low}else{upper}}
            }
        };
        if bits==0{return Err(Error::Underflow);}
        Ok(f64::from_bits(bits|if self.negative{1u64<<63}else{0}))
    }
    fn round_positive_factor(&self)->Result<f64,Error>{if !self.positive(){Err(Error::EquityDomain)}else{self.round()}}
}

/// Exact source-rule values share the established canonical-decimal parser and
/// nearest-even converter. The integer representation stays private so callers
/// cannot construct an invalid denominator or a negative zero.
#[derive(Clone, Debug)]
pub(crate) struct ExactValue(Rational);

/// Lossless rational evidence, not a display price or a reduced-fraction claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExactAudit {
    pub negative: bool,
    pub numerator_hex: String,
    pub denominator_hex: String,
}

impl ExactValue {
    pub(crate) fn canonical(value: f64) -> Result<Self, Error> {
        Rational::canonical(value).map(Self)
    }

    pub(crate) fn ratio(numerator: u32, denominator: u32) -> Result<Self, Error> {
        if denominator == 0 { return Err(Error::InvalidDomain); }
        Ok(Self(Rational::new(false, Natural::from_u64(numerator.into()), Natural::from_u64(denominator.into()))))
    }

    pub(crate) fn add(&self, other: &Self) -> Self { Self(self.0.add(&other.0)) }
    pub(crate) fn sub(&self, other: &Self) -> Self { Self(self.0.sub(&other.0)) }

    pub(crate) fn scale(&self, multiplier: u32) -> Self {
        Self(Rational::new(self.0.negative, self.0.numerator.mul(&Natural::from_u64(multiplier.into())), self.0.denominator.clone()))
    }

    pub(crate) fn divide(&self, denominator: u32) -> Result<Self, Error> {
        if denominator == 0 { return Err(Error::InvalidDomain); }
        Ok(Self(Rational::new(self.0.negative, self.0.numerator.clone(), self.0.denominator.mul(&Natural::from_u64(denominator.into())))))
    }

    pub(crate) fn signed_cmp(&self, other: &Self) -> Ordering {
        if self.0.negative != other.0.negative {
            return if self.0.negative { Ordering::Less } else { Ordering::Greater };
        }
        let magnitude = self.0.numerator.mul(&other.0.denominator)
            .cmp(&other.0.numerator.mul(&self.0.denominator));
        if self.0.negative { magnitude.reverse() } else { magnitude }
    }

    pub(crate) fn positive(&self) -> bool { self.0.positive() }
    pub(crate) fn rounded(&self) -> Result<f64, Error> { self.0.round() }

    pub(crate) fn audit(&self) -> ExactAudit {
        fn hex(value: &Natural) -> String {
            let mut words = value.0.iter().rev();
            let Some(first) = words.next() else { return "0".into(); };
            let mut text = format!("{first:x}");
            for word in words { text.push_str(&format!("{word:08x}")); }
            text
        }
        ExactAudit { negative: self.0.negative, numerator_hex: hex(&self.0.numerator), denominator_hex: hex(&self.0.denominator) }
    }

    /// A fresh bounded window prevents unreduced denominators from growing
    /// with the length of a market series. Domain-specific positivity and
    /// genuine-session validation belong to the calling strategy.
    pub(crate) fn mean20(values: &[f64]) -> Result<Self, Error> {
        if values.len() != 20 { return Err(Error::InvalidDomain); }
        let mut sum = Self(Rational::zero());
        for &value in values { sum = sum.add(&Self::canonical(value)?); }
        sum.divide(20)
    }
}

#[derive(Clone,Debug)]
pub struct ModelScalars {
    pub gross:Result<f64,Error>,pub fee:Result<f64,Error>,pub slippage:Result<f64,Error>,
    pub net:Result<f64,Error>,pub gross_factor:Result<f64,Error>,pub net_factor:Result<f64,Error>,
}
/// Separate canonical cost components; all finite nonnegative values are allowed
/// by this helper. HTTP caps and report-shape admission remain separate.
pub fn evaluate(entry:f64,mark:f64,fees_bps:f64,slippage_bps:f64,sides:u8)->Result<ModelScalars,Error>{
    if !entry.is_finite()||entry<=0.||!mark.is_finite()||mark<=0.||!fees_bps.is_finite()||fees_bps<0.||!slippage_bps.is_finite()||slippage_bps<0.||sides>2{return Err(Error::InvalidDomain);}
    let e=Rational::canonical(entry)?;let x=Rational::canonical(mark)?;let denominator=Rational::integer(10000);
    let fee=Rational::canonical(fees_bps)?.div(&denominator).times(sides);
    let slippage=Rational::canonical(slippage_bps)?.div(&denominator).times(sides);
    let ratio=x.div(&e);let gross=ratio.sub(&Rational::integer(1));let deduction=fee.add(&slippage);
    let net=gross.sub(&deduction);let net_factor=ratio.sub(&deduction);
    Ok(ModelScalars{gross:gross.round(),fee:fee.round(),slippage:slippage.round(),net:net.round(),gross_factor:ratio.round_positive_factor(),net_factor:net_factor.round_positive_factor()})
}
/// Ordinary binary64 multiplication with explicit unsupported range failures.
pub fn checked_product(a:f64,b:f64)->Result<f64,Error>{
    if !a.is_finite()||!b.is_finite(){return Err(Error::InvalidDomain);}
    if a==0.||b==0.{return Ok(0.);}
    let product=a*b;if !product.is_finite(){Err(Error::Overflow)}else if product==0.{Err(Error::Underflow)}else{Ok(product)}
}
#[cfg(test)]
fn evaluate_for_test(entry:f64,mark:f64,fees_bps:f64,slippage_bps:f64,sides:u8)->Result<ModelScalars,Error>{evaluate(entry,mark,fees_bps,slippage_bps,sides)}
#[cfg(test)]
fn round_ratio_for_test(numerator:&str,denominator:&str)->Result<f64,Error>{
    let(negative,digits)=numerator.strip_prefix('-').map_or((false,numerator),|s|(true,s));
    let n=Natural::from_digits(digits)?;let d=Natural::from_digits(denominator)?;
    if d.is_zero(){return Err(Error::InvalidDomain);}Rational::new(negative,n,d).round()
}
#[cfg(test)]
#[path="unit_exposure_arithmetic_tests.rs"]
mod tests;
#[cfg(test)]
#[path="exact_value_tests.rs"]
mod exact_value_tests;
