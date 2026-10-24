//! Source-qualified, request-owned native daily data. No startup indicator population or permanent row leaks.
use crate::{Row,esc,parse_csv_fields};
use std::{collections::BTreeMap,path::{Path,PathBuf,Component},io::{Read,Write},process::{Command,Stdio}};

#[path="pristine_data.rs"]
pub(crate) mod pristine_data;

#[path="macs_source_data.rs"]
pub(crate) mod macs_source_data;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Market { India, UnitedStates }
impl Market {
    pub fn code(self)->&'static str {match self {Self::India=>"in",Self::UnitedStates=>"us"}}
    pub fn currency(self)->&'static str {match self {Self::India=>"INR",Self::UnitedStates=>"USD"}}
}
/// Created only by the server's fixed, local whitelist parser. A query currency,
/// a ticker spelling, or a strategy/book title cannot construct this evidence.
#[derive(Clone,Debug)]
pub struct NativeEvidence {
    market:Market,membership_kind:String,membership_source:String,membership_source_sha:String,
    mapped_index:&'static str,adjustment_status:String,
}
impl NativeEvidence {
    pub fn market(&self)->Market {self.market}
    pub fn currency(&self)->&'static str {self.market.currency()}
    pub fn asset_type(&self)->&'static str {"individual_stock"}
    pub fn membership_kind(&self)->&str {&self.membership_kind}
    pub fn mapped_index(&self)->&'static str {self.mapped_index}
    pub fn adjustment_status(&self)->&str {&self.adjustment_status}
    // No staged source currently supplies reviewed historical price-unit/tick
    // conventions. A manifest boolean cannot certify this scientific claim.
    pub fn source_price_units_verified(&self)->bool {false}
    #[cfg(test)]
    pub(crate) fn us_test_fixture()->Self {
        Self{market:Market::UnitedStates,membership_kind:"current_snapshot".into(),membership_source:"Test source list".into(),membership_source_sha:"a".repeat(64),mapped_index:"SPX",adjustment_status:"not_independently_verified".into()}
    }
    #[cfg(test)]
    pub(crate) fn india_test_fixture()->Self {
        Self{market:Market::India,..Self::us_test_fixture()}
    }
    fn json(&self)->String {
        format!("\"trusted_market\":\"{}\",\"currency\":\"{}\",\"asset_type\":\"{}\",\"timeframe\":\"D\",\"membership_kind\":\"{}\",\"membership_source\":\"{}\",\"membership_source_sha256\":{},\"mapped_index\":\"{}\",\"adjustment_status\":\"{}\",\"source_price_units_verified\":false,\"historical_membership_verified\":false,\"corporate_actions_independently_verified\":false",
            self.market.code(),self.currency(),self.asset_type(),esc(&self.membership_kind),esc(&self.membership_source),
            if self.membership_source_sha.is_empty(){"null".into()}else{format!("\"{}\"",self.membership_source_sha)},self.mapped_index,esc(&self.adjustment_status))
    }
}
fn native_evidence<'a>(basket:&str,optional:impl Fn(&str)->Option<&'a str>)->Result<NativeEvidence,String> {
    let legacy_index=match basket {"nse500"|"nifty50"|"midcap100"|"allfo"=>Some("NIFTY 50"),_=>None};
    let us_index=match basket {
        "us_sp500"|"us_sp500_past"=>Some("SPX"),"us_nasdaq100"|"us_nasdaq100_past"=>Some("NDX"),
        "us_dow30"|"us_dow30_past"=>Some("DJI"),"us_russell2000"|"us_russell2000_past"=>Some("RUT"),_=>None,
    };
    let market=match optional("trusted_market") {
        None if legacy_index.is_some()=>Market::India,
        Some("in") if legacy_index.is_some()=>Market::India,
        Some("us") if us_index.is_some()=>Market::UnitedStates,
        _=>return Err("Native basket lacks a supported explicit market/namespace mapping".into()),
    };
    let explicit_us=market==Market::UnitedStates;
    for (field,expected) in [("currency",market.currency()),("asset_type","individual_stock"),("timeframe","D")] {
        match optional(field) {Some(value) if value==expected=>{},None if !explicit_us=>{},_=>return Err(format!("Native basket {field} contradicts its trusted market/type"))}
    }
    match optional("source_price_units_verified") {None if !explicit_us=>{},Some("false")=>{},_=>return Err("Native source price/tick units are not independently verified; a manifest cannot self-certify them".into())}
    let adjustment_status=optional("adjustment_status").unwrap_or("not_independently_verified");
    if adjustment_status!="not_independently_verified" {return Err("Unsupported native adjustment verification claim".into());}
    if explicit_us&&optional("adjustment_status").is_none(){return Err("US source adjustment status is required".into());}
    let mapped_index=if explicit_us {us_index.unwrap()}else{legacy_index.unwrap()};
    match optional("mapped_index") {Some(value) if value==mapped_index=>{},None if !explicit_us=>{},_=>return Err("Native basket mapped index disagrees with its source namespace".into())}
    let expected_kind=if explicit_us&&basket.ends_with("_past"){"current_plus_past_snapshot"}else{"current_snapshot"};
    let membership_kind=optional("membership_kind").unwrap_or(expected_kind);
    if membership_kind!=expected_kind||explicit_us&&optional("membership_kind").is_none(){return Err("Native membership must be the named current/current-plus-past snapshot, not a historical membership claim".into());}
    let membership_source=optional("membership_source").unwrap_or("Legacy Indian native watchlist source; see source provenance");
    let membership_source_sha=optional("membership_source_sha256").unwrap_or("");
    if !text_safe(membership_source)||explicit_us&&(!valid_sha(membership_source_sha)||optional("membership_source").is_none())||!membership_source_sha.is_empty()&&!valid_sha(membership_source_sha){return Err("Native membership source and exact source SHA-256 are required".into());}
    Ok(NativeEvidence{market,membership_kind:membership_kind.into(),membership_source:membership_source.into(),membership_source_sha:membership_source_sha.to_ascii_lowercase(),mapped_index,adjustment_status:adjustment_status.into()})
}

#[derive(Default)]
pub struct Catalogue {root:PathBuf,entries:BTreeMap<String,Entry>,expected_members:BTreeMap<String,usize>,unavailable_members:BTreeMap<String,BasketMember>}
/// Server-owned universe inventory. This exposes no filesystem paths and does
/// not turn a static source watchlist into dated historical membership.
#[derive(Clone)]
pub struct BasketMember {pub key:String,pub symbol:String,pub first:String,pub last:String,pub market:Market,pub membership_kind:String}
struct Entry {basket:String,symbol:String,path:PathBuf,first:String,last:String,bars:usize,source:String,source_sha:String,csv_sha:String,membership:String,basket_member:bool,has_volume:bool,excluded_prefix:usize,original_bars:usize,coverage_note:String,evidence:NativeEvidence}

fn sha256_hex(bytes:&[u8])->Result<String,String>{
    // Coreutils is present in the engine's Debian base. Hash the very bytes
    // parsed below, rather than reopening a path and introducing a TOCTOU gap.
    let mut child=Command::new("sha256sum").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|_|"Source checksum verification is unavailable")?;
    child.stdin.take().ok_or("Source checksum input unavailable")?.write_all(bytes).map_err(|_|"Source checksum input failed")?;
    let output=child.wait_with_output().map_err(|_|"Source checksum verification failed")?;
    let hash=String::from_utf8_lossy(&output.stdout).split_whitespace().next().unwrap_or("").to_string();
    if !output.status.success()||!valid_sha(&hash){return Err("Source checksum verification failed".into());}Ok(hash)
}
fn valid_sha(value:&str)->bool{value.len()==64&&value.bytes().all(|c|c.is_ascii_hexdigit())}
fn text_safe(value:&str)->bool{!value.is_empty()&&!value.chars().any(|c|c.is_control())}
pub(crate) fn valid_date(value:&str)->bool{
    if value.len()!=10||value.as_bytes()[4]!=b'-'||value.as_bytes()[7]!=b'-'{return false;}
    let parts=value.split('-').map(str::parse::<usize>).collect::<Result<Vec<_>,_>>();
    let Ok(p)=parts else{return false;};if p.len()!=3||p[0]<1900||p[0]>2200||p[1]<1||p[1]>12{return false;}
    let days=[31,if p[0]%4==0&&(p[0]%100!=0||p[0]%400==0){29}else{28},31,30,31,30,31,31,30,31,30,31];p[2]>0&&p[2]<=days[p[1]-1]
}
pub fn parse_rows(text:&str,symbol:&str)->Result<Vec<Row>,String>{
    let mut lines=text.lines();let header=parse_csv_fields(lines.next().ok_or("Native CSV is empty")?);
    let col=|name:&str|header.iter().position(|key|key.eq_ignore_ascii_case(name)).ok_or_else(||format!("Native CSV missing {name}"));
    let [di,oi,hi,li,ci,vi]=[col("date")?,col("open")?,col("high")?,col("low")?,col("close")?,col("volume")?];
    let mut rows:Vec<Row>=vec![];
    for (line_number,line) in lines.enumerate(){
        if line.trim().is_empty(){continue;}
        let fields=parse_csv_fields(line);let date=fields.get(di).ok_or("Native date missing")?;
        if !valid_date(date)||rows.last().is_some_and(|r|r.date>=*date){return Err(format!("Native date invalid, duplicated or unsorted at row {}",line_number+2));}
        let n=|i:usize|fields.get(i).and_then(|v|v.parse::<f64>().ok()).filter(|v|v.is_finite()).ok_or_else(||format!("Native numeric field invalid at row {}",line_number+2));
        let (o,h,l,c,v)=(n(oi)?,n(hi)?,n(li)?,n(ci)?,n(vi)?);
        if o<=0.||h<=0.||l<=0.||c<=0.||v<0.||l>h||o<l||o>h||c<l||c>h{return Err(format!("Native OHLC/volume invalid at row {}",line_number+2));}
        rows.push(Row{ticker:symbol.into(),date:date.clone(),open:o,high:h,low:l,close:c,volume:(v>0.).then_some(v)});
        if rows.len()>100000{return Err("Native series exceeds the bounded daily-series limit".into());}
    }
    if rows.len()<2{return Err("Native series has fewer than two usable source bars".into());}Ok(rows)
}
impl Catalogue {
    pub fn load(directory:&Path)->Result<Self,String>{
        let root=directory.canonicalize().map_err(|_|"Basket data directory unavailable")?;
        let index=root.join("index.csv").canonicalize().map_err(|_|"Basket whitelist index.csv unavailable")?;
        if !index.starts_with(&root){return Err("Basket whitelist escapes its source directory".into());}
        let mut bytes=Vec::new();std::fs::File::open(index).map_err(|_|"Basket whitelist index.csv unavailable")?.take(16*1024*1024+1).read_to_end(&mut bytes).map_err(|_|"Basket whitelist read failed")?;
        if bytes.len()>16*1024*1024{return Err("Basket whitelist is too large".into());}
        let text=std::str::from_utf8(&bytes).map_err(|_|"Basket whitelist is not UTF-8")?;
        let mut lines=text.lines();let header=parse_csv_fields(lines.next().ok_or("Basket whitelist empty")?);
        if header.iter().any(|name|!text_safe(name))||header.iter().enumerate().any(|(i,name)|header[..i].contains(name)){return Err("Basket whitelist has unsafe or duplicate columns".into());}
        let mut catalogue=Self{root,entries:BTreeMap::new(),expected_members:BTreeMap::new(),unavailable_members:BTreeMap::new()};
        let mut source_keys=std::collections::BTreeSet::new();
        for line in lines.filter(|line|!line.trim().is_empty()){
            let fields=parse_csv_fields(line);
            let field=|name:&str|header.iter().position(|key|key==name).and_then(|i|fields.get(i)).map(|v|v.trim()).ok_or_else(||format!("Basket whitelist missing {name}"));
            let optional=|name:&str|header.iter().position(|key|key==name).and_then(|i|fields.get(i)).map(|v|v.trim());
            if header.iter().any(|key|key=="basket_expected_members"){
                let expected=field("basket_expected_members")?.parse::<usize>().ok().filter(|n|(1..=25_000).contains(n)).ok_or("Invalid expected source-basket member count")?;
                let basket=field("basket")?;
                if !text_safe(basket)||!basket.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'_'||c==b'-'){return Err("Unsafe expected-basket namespace".into());}
                if catalogue.expected_members.insert(basket.into(),expected).is_some_and(|old|old!=expected){return Err("Inconsistent expected source-basket member counts".into());}
            }
            let source_key=format!("basket:{}:{}",field("basket")?,field("ticker")?);
            if !source_keys.insert(source_key){return Err("Duplicate qualified basket symbol".into());}
            if !matches!(optional("basket_member"),None|Some("true")|Some("false")){return Err("Invalid basket member flag".into());}
            if field("status")?!="ready" {
                if optional("basket_member")!=Some("false") {
                    let (basket,symbol)=(field("basket")?,field("ticker")?);
                    if !text_safe(basket)||!text_safe(symbol)||symbol.contains(':') {return Err("Unsafe missing member identity".into());}
                    let evidence=native_evidence(basket,&optional)?;
                    let key=format!("basket:{basket}:{symbol}");
                    if catalogue.unavailable_members.insert(key.clone(),BasketMember{key,symbol:symbol.into(),first:String::new(),last:String::new(),market:evidence.market(),membership_kind:evidence.membership_kind().into()}).is_some(){return Err("Duplicate missing basket member".into());}
                }
                continue;
            }
            let (basket,symbol,path)=(field("basket")?,field("ticker")?,PathBuf::from(field("path")?));
            if !text_safe(basket)||!basket.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'_'||c==b'-')||!text_safe(symbol)||symbol.contains(':')||path.as_os_str().is_empty()||path.components().any(|c|!matches!(c,Component::Normal(_))){return Err("Unsafe basket namespace or source path".into());}
            let first=field("first")?;let last=field("last")?;let bars=field("bars")?.parse::<usize>().map_err(|_|"Invalid native source bar count")?;
            let source_sha=field("source_sha256")?;let csv_sha=field("csv_sha256")?;
            if !valid_date(first)||!valid_date(last)||first>last||bars<2||bars>100000||!valid_sha(source_sha)||!valid_sha(csv_sha){return Err("Invalid native source provenance".into());}
            let key=format!("basket:{basket}:{symbol}");
            let evidence=native_evidence(basket,&optional)?;
            let has_volume=match field("has_volume")? {"true"=>true,"false"=>false,_=>return Err("Invalid native volume-availability status".into())};
            let basket_member=match optional("basket_member"){None|Some("true")=>true,Some("false")=>false,_=>return Err("Invalid native current-basket admission flag".into())};
            if !text_safe(field("source")?)||!text_safe(field("membership")?){return Err("Native source/membership provenance is empty or unsafe".into());}
            let entry=Entry{basket:basket.into(),symbol:symbol.into(),path,first:first.into(),last:last.into(),bars,source:field("source")?.into(),source_sha:source_sha.to_ascii_lowercase(),csv_sha:csv_sha.to_ascii_lowercase(),membership:field("membership")?.into(),basket_member,has_volume,excluded_prefix:optional("excluded_prefix_count").unwrap_or("0").parse().map_err(|_|"Invalid prefix exclusion count")?,original_bars:optional("original_bars").map_or(Ok(bars),str::parse).map_err(|_|"Invalid original source count")?,coverage_note:optional("coverage_note").unwrap_or("No source prefix exclusion recorded").into(),evidence};
            if entry.original_bars<entry.bars||entry.original_bars-entry.bars!=entry.excluded_prefix{return Err("Native prefix counts do not reconcile".into());}
            if catalogue.entries.insert(key,entry).is_some(){return Err("Duplicate qualified basket symbol".into());}
        }
        Ok(catalogue)
    }
    pub fn contains(&self,key:&str)->bool{self.entries.contains_key(key)}
    pub fn expected_member_count(&self,basket:&str)->Option<usize>{self.expected_members.get(basket).copied()}
    pub fn members(&self,basket:&str)->Vec<BasketMember>{
        let mut members:Vec<_>=self.entries.iter().filter(|(_,e)|e.basket==basket&&e.basket_member).map(|(key,e)|BasketMember{key:key.clone(),symbol:e.symbol.clone(),first:e.first.clone(),last:e.last.clone(),market:e.evidence.market(),membership_kind:e.evidence.membership_kind().into()}).collect();
        members.sort_by(|a,b|a.symbol.cmp(&b.symbol));members
    }
    /// Includes named source constituents whose price files are not admitted.
    pub fn source_members(&self,basket:&str)->Vec<BasketMember>{
        let mut members=self.members(basket);
        members.extend(self.unavailable_members.values().filter(|m|m.key.starts_with(&format!("basket:{basket}:"))).cloned());
        members.sort_by(|a,b|a.symbol.cmp(&b.symbol));members
    }
    pub fn native_evidence(&self,key:&str)->Option<&NativeEvidence>{self.entries.get(key).map(|entry|&entry.evidence)}
    pub fn rows(&self,key:&str)->Result<Vec<Row>,String>{self.read_rows(key,true)}
    /// Basket admission uses the verified file's actual dates. Metadata dates
    /// remain strict on all existing single-symbol and precision-domain paths.
    pub fn basket_rows(&self,key:&str)->Result<Vec<Row>,String>{self.read_rows(key,false)}
    fn read_rows(&self,key:&str,strict_coverage:bool)->Result<Vec<Row>,String>{
        let entry=self.entries.get(key).ok_or("Unknown qualified basket symbol")?;
        let path=self.root.join(&entry.path).canonicalize().map_err(|_|"Native source CSV unavailable")?;
        if !path.starts_with(&self.root){return Err("Native source path escapes its whitelist directory".into());}
        if std::fs::metadata(&path).map_err(|_|"Native source metadata unavailable")?.len()>32*1024*1024{return Err("Native source CSV exceeds memory admission limit".into());}
        let bytes=std::fs::read(path).map_err(|_|"Native source CSV unavailable")?;
        if sha256_hex(&bytes)?!=entry.csv_sha{return Err("Native source SHA-256 mismatch; data changed after whitelist generation".into());}
        let text=std::str::from_utf8(&bytes).map_err(|_|"Native source CSV is not UTF-8")?;
        let rows=parse_rows(text,&entry.symbol)?;
        if strict_coverage && (rows.len()!=entry.bars||rows.first().unwrap().date!=entry.first||rows.last().unwrap().date!=entry.last){return Err("Native source coverage disagrees with the whitelist".into());}
        Ok(rows)
    }
    /// Separate explicit research decoder. Legacy rows()/parse_rows() and all
    /// existing admission/default behavior are deliberately unchanged.
    pub(crate) fn pristine_series(&self,key:&str,mode:Option<pristine_data::ResearchMode>)->Result<pristine_data::Series,pristine_data::DataError>{
        use pristine_data::DataError;
        pristine_data::mode(mode)?;
        let entry=self.entries.get(key).ok_or(DataError::UnknownQualifiedTicker)?;
        pristine_data::evidence(&entry.evidence)?;
        let error=|message:&str|DataError::Source(message.into());
        let path=self.root.join(&entry.path).canonicalize().map_err(|_|error("Native source CSV unavailable"))?;
        if !path.starts_with(&self.root){return Err(error("Native source path escapes its whitelist directory"));}
        let file=std::fs::File::open(path).map_err(|_|error("Native source CSV unavailable"))?;
        if file.metadata().map_err(|_|error("Native source metadata unavailable"))?.len()>32*1024*1024{return Err(error("Native source CSV exceeds memory admission limit"));}
        // Bound the actual read as well as metadata; do not trust a pre-read
        // length if the file changes. Hash precisely the bytes decoded below.
        let mut bytes=Vec::new();
        file.take(32*1024*1024+1).read_to_end(&mut bytes).map_err(|_|error("Native source CSV read failed"))?;
        if bytes.len()>32*1024*1024{return Err(error("Native source CSV exceeds memory admission limit"));}
        if sha256_hex(&bytes).map_err(DataError::Source)?!=entry.csv_sha{return Err(error("Native source SHA-256 mismatch; data changed after whitelist generation"));}
        let text=std::str::from_utf8(&bytes).map_err(|_|error("Native source CSV is not UTF-8"))?;
        let decoded=pristine_data::decode(text)?;
        if decoded.rows.len()!=entry.bars||decoded.rows.first().unwrap().date!=entry.first||decoded.rows.last().unwrap().date!=entry.last{return Err(error("Native source coverage disagrees with the whitelist"));}
        let provenance=self.provenance_json(key).ok_or(DataError::UnknownQualifiedTicker)?;
        Ok(pristine_data::from_verified_catalogue(key,&entry.symbol,&entry.evidence,provenance,decoded))
    }
    /// Separate strategy-specific precision domain. Legacy rows()/parse_rows(),
    /// Pristine and all other methods retain their existing admission behavior.
    /// Whole-file hash/format/coverage remain mandatory; raw close/date/binary64 identity and exact RAW decimal values
    /// are additionally retained for every actual observation0..=to, without repair.
    pub(crate) fn macs_raw_decimal_series(&self,key:&str,to:usize)
        -> Result<macs_source_data::ValidatedSeries,macs_source_data::DataError> {
        use macs_source_data::DataError;
        let entry=self.entries.get(key).ok_or_else(||DataError::Source("Unknown qualified MACS stock symbol".into()))?;
        if entry.evidence.asset_type()!="individual_stock" { return Err(DataError::UnsupportedStockEvidence); }
        let error=|message:&str|DataError::Source(message.into());
        let path=self.root.join(&entry.path).canonicalize().map_err(|_|error("Native MACS CSV unavailable"))?;
        if !path.starts_with(&self.root) { return Err(error("Native MACS path escapes its whitelist directory")); }
        let file=std::fs::File::open(path).map_err(|_|error("Native MACS CSV unavailable"))?;
        if file.metadata().map_err(|_|error("Native MACS metadata unavailable"))?.len()>32*1024*1024 {
            return Err(error("Native MACS CSV exceeds memory admission limit"));
        }
        let mut bytes=Vec::new();
        file.take(32*1024*1024+1).read_to_end(&mut bytes).map_err(|_|error("Native MACS CSV read failed"))?;
        if bytes.len()>32*1024*1024 { return Err(error("Native MACS CSV exceeds memory admission limit")); }
        if sha256_hex(&bytes).map_err(DataError::Source)?!=entry.csv_sha { return Err(error("Native MACS SHA-256 changed from whitelist")); }
        let text=std::str::from_utf8(&bytes).map_err(|_|error("Native MACS CSV is not UTF-8"))?;
        let provenance=self.provenance_json(key).ok_or_else(||error("Native MACS provenance unavailable"))?;
        let series=macs_source_data::from_verified_catalogue(key,&entry.symbol,&entry.evidence,provenance,&entry.csv_sha,text,to)?;
        let rows=series.rows();
        if rows.len()!=entry.bars || rows.first().unwrap().date!=entry.first || rows.last().unwrap().date!=entry.last {
            return Err(error("Native MACS coverage disagrees with whitelist"));
        }
        Ok(series)
    }

    pub fn provenance_json(&self,key:&str)->Option<String>{
        self.entries.get(key).map(|e|{let mut json=format!("{{\"qualified_ticker\":\"{}\",\"symbol\":\"{}\",\"basket\":\"{}\",\"source\":\"{}\",\"source_sha256\":\"{}\",\"csv_sha256\":\"{}\",\"membership\":\"{}\",\"first\":\"{}\",\"last\":\"{}\",\"bars\":{},\"excluded_prefix_count\":{},\"original_bars\":{},\"coverage_note\":\"{}\",\"quote_accuracy_independently_verified\":false,\"basis\":\"Actual native daily vendor OHLC and volume; unknown auxiliary fields are not trading signals. Current watchlist membership is a snapshot, not historical membership: retrospective universe selection has survivorship bias. Cash-stock F&O eligibility does not establish futures contracts or roll events.\"}}",esc(key),esc(&e.symbol),esc(&e.basket),esc(&e.source),e.source_sha,e.csv_sha,esc(&e.membership),e.first,e.last,e.bars,e.excluded_prefix,e.original_bars,esc(&e.coverage_note));json.pop();json.push_str(&format!(",\"basket_member\":{},\"basket_expected_members\":{},{}}}",e.basket_member,self.expected_member_count(&e.basket).map_or("null".into(),|n|n.to_string()),e.evidence.json()));json})
    }
    pub fn tickers_json(&self)->Vec<String>{
        self.entries.iter().map(|(key,e)|format!("{{\"ticker\":\"{}\",\"symbol\":\"{}\",\"basket\":\"{}\",\"first\":\"{}\",\"last\":\"{}\",\"has_volume\":{},\"series\":[],\"price_status\":\"ready\",\"native\":true,{},\"provenance\":{}}}",esc(key),esc(&e.symbol),esc(&e.basket),e.first,e.last,e.has_volume,format!("\"basket_member\":{},\"basket_expected_members\":{},{}",e.basket_member,self.expected_member_count(&e.basket).map_or("null".into(),|n|n.to_string()),e.evidence.json()),self.provenance_json(key).unwrap())).collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs,path::PathBuf,sync::atomic::{AtomicUsize,Ordering}};
    static NEXT:AtomicUsize=AtomicUsize::new(0);
    struct Fixture(PathBuf);
    impl Drop for Fixture{fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
    fn fixture()->Fixture{
        let p=std::env::temp_dir().join(format!("vault-native-test-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));fs::create_dir_all(&p).unwrap();
        let csv="date,open,high,low,close,volume,open_interest,aux1,aux2\n2020-01-01,100,110,90,105,1000,0,777,888\n2020-01-02,105,115,95,110,2000,0,777,888\n";
        fs::write(p.join("quote.csv"),csv).unwrap();let sha=sha256_hex(csv.as_bytes()).unwrap();
        fs::write(p.join("index.csv"),format!("basket,ticker,path,first,last,bars,source,source_sha256,csv_sha256,membership,status,has_volume\nnifty50,RELIANCE,quote.csv,2020-01-01,2020-01-02,2,Native current watchlist,{}, {},Current snapshot; historical survivorship bias,ready,true\n", "a".repeat(64),sha)).unwrap();Fixture(p)
    }
    #[test]
    fn archived_aliases_keep_actual_stock_prices_but_cannot_enter_current_basket(){
        let f=fixture();let old=fs::read_to_string(f.0.join("index.csv")).unwrap();let row=old.lines().nth(1).unwrap();
        let mut index=format!("{},basket_member\n{},true\n",old.lines().next().unwrap(),row);
        for symbol in ["HEROMOTOCO","INDUSINDBK","WIPRO"]{index.push_str(&format!("{},false\n",row.replace(",RELIANCE,",&format!(",{symbol},"))));}
        fs::write(f.0.join("index.csv"),&index).unwrap();let c=Catalogue::load(&f.0).unwrap();let members=c.members("nifty50");assert_eq!(members.len(),1);assert_eq!(members[0].symbol,"RELIANCE");
        for symbol in ["HEROMOTOCO","INDUSINDBK","WIPRO"]{let key=format!("basket:nifty50:{symbol}");assert!(c.contains(&key));assert_eq!(c.rows(&key).unwrap()[0].ticker,symbol);assert!(c.provenance_json(&key).unwrap().contains("\"basket_member\":false"));assert!(c.tickers_json().iter().any(|json|json.contains(&key)&&json.contains("\"basket_member\":false")));}
        assert!(c.members("unknown").is_empty());
        fs::write(f.0.join("index.csv"),index.replace(",false",",dead")).unwrap();assert!(Catalogue::load(&f.0).is_err());
    }
    #[test]
    fn expected_coverage_is_strict_consistent_and_includes_unready_source_rows(){
        let f=fixture();let old=fs::read_to_string(f.0.join("index.csv")).unwrap();let header=old.lines().next().unwrap();let row=old.lines().nth(1).unwrap();
        let missing=row.replace(",RELIANCE,",",GSPL,").replace(",ready,",",missing,");let index=format!("{header},basket_expected_members\n{row},2\n{missing},2\n");
        fs::write(f.0.join("index.csv"),&index).unwrap();let c=Catalogue::load(&f.0).unwrap();assert_eq!(c.expected_member_count("nifty50"),Some(2));assert_eq!(c.members("nifty50").len(),1);assert_eq!(c.source_members("nifty50").len(),2);assert!(c.source_members("nifty50").iter().any(|m|m.symbol=="GSPL"));assert!(!c.contains("basket:nifty50:GSPL"));assert!(c.tickers_json()[0].contains("\"basket_expected_members\":2"));
        fs::write(f.0.join("index.csv"),format!("{header},basket_expected_members\n{row},2\n{missing},3\n")).unwrap();assert!(Catalogue::load(&f.0).is_err());
        for value in ["0","25001","NaN","1.5","-1",""]{fs::write(f.0.join("index.csv"),format!("{header},basket_expected_members\n{row},{value}\n")).unwrap();assert!(Catalogue::load(&f.0).is_err(),"{value}");}
        fs::write(f.0.join("index.csv"),old).unwrap();assert_eq!(Catalogue::load(&f.0).unwrap().expected_member_count("nifty50"),None);
    }
    #[test]
    fn metadata_is_lazy_namespaced_and_does_not_relabel_original_symbol(){
        let f=fixture();let catalogue=Catalogue::load(&f.0).unwrap();let key="basket:nifty50:RELIANCE";
        let rows=catalogue.rows(key).unwrap();assert_eq!(rows[0].ticker,"RELIANCE");assert_eq!(rows[1].volume,Some(2000.));
        assert!(catalogue.tickers_json()[0].contains(key));assert!(catalogue.provenance_json(key).unwrap().contains("survivorship"));
        fs::remove_file(f.0.join("quote.csv")).unwrap();assert!(catalogue.rows(key).is_err(),"Catalogue must not retain an unbounded cache of loaded quote arrays");
    }
    #[test]
    fn traversal_duplicate_keys_and_tampered_data_are_rejected(){
        let f=fixture();let index=fs::read_to_string(f.0.join("index.csv")).unwrap();
        fs::write(f.0.join("index.csv"),index.replace("quote.csv","../quote.csv")).unwrap();assert!(Catalogue::load(&f.0).is_err());
        fs::write(f.0.join("index.csv"),format!("{}{}",index,index.lines().nth(1).unwrap())).unwrap();assert!(Catalogue::load(&f.0).is_err());
        fs::write(f.0.join("index.csv"),index).unwrap();let catalogue=Catalogue::load(&f.0).unwrap();fs::write(f.0.join("quote.csv"),"tampered").unwrap();assert!(catalogue.rows("basket:nifty50:RELIANCE").unwrap_err().contains("SHA-256"));
    }
    #[test]
    fn malformed_ohlc_duplicates_and_missing_volume_are_not_filled_or_dropped(){
        let valid="date,open,high,low,close,volume\n2020-01-01,100,110,90,105,0\n2020-01-02,105,115,95,110,1000\n";
        let rows=parse_rows(valid,"RELIANCE").unwrap();assert_eq!(rows[0].volume,None);
        assert!(parse_rows(&valid.replace("100,110,90,105","100,90,110,105"),"RELIANCE").is_err());
        assert!(parse_rows(&valid.replace("2020-01-02","2020-01-01"),"RELIANCE").is_err());
        assert!(parse_rows(&valid.replace("2020-01-02","2020-02-31"),"RELIANCE").is_err());
        assert!(parse_rows(&valid.replace("105,115,95,110","105,115,95,NaN"),"RELIANCE").is_err());
    }
    #[test]
    fn same_name_in_two_baskets_cannot_override_preloaded_index(){
        let f=fixture();let index=fs::read_to_string(f.0.join("index.csv")).unwrap();let other=index.lines().nth(1).unwrap().replace("nifty50,","nse500,");
        fs::write(f.0.join("index.csv"),format!("{index}{other}\n")).unwrap();let c=Catalogue::load(&f.0).unwrap();
        assert_eq!(c.tickers_json().len(),2);assert!(!c.contains("RELIANCE"));assert!(c.rows("RELIANCE").is_err());
    }
    #[test]
    fn lazy_endpoint_uses_original_symbol_native_bars_and_explicit_coverage_blocks(){
        let f=fixture();let state=crate::State{ctxs:Default::default(),baskets:Catalogue::load(&f.0).unwrap(),vix:vec![],market_extra:Default::default()};
        let mut q=std::collections::BTreeMap::from([("ticker".into(),"basket:nifty50:RELIANCE".into()),("id".into(),"f1a".into())]);
        let readiness=crate::readiness_json(&state,&q).unwrap();assert!(readiness.contains("\"status\":\"ready\"")&&readiness.contains("\"symbol\":\"RELIANCE\""));
        let run=crate::detail(&state,&q).unwrap();assert!(run.contains("\"ohlc\":{"));assert!(run.contains("\"o\":[100,105]"));assert!(run.contains("\"v\":[1000,2000]"));assert!(run.contains("\"benchmark_symbol\":\"RELIANCE\""));assert!(!run.contains("777"));
        // auto-start (2026-10-23): an earlier requested start begins at the stock's first actual bar, with a note
        q.insert("from".into(),"2019-01-01".into());assert!(crate::readiness_json(&state,&q).unwrap().contains("requested start 2019-01-01 is earlier"));assert!(crate::detail(&state,&q).unwrap().contains("\"auto_start\":\"Started "));
        q.remove("from");q.insert("id".into(),"tps_stocks".into());assert!(crate::readiness_json(&state,&q).unwrap().contains("benchmark"));assert!(crate::detail(&state,&q).unwrap_err().contains("Liquidity Filter"));
    }
    #[test]
    fn staged_manifest_and_one_actual_native_series_match_declared_coverage(){
        let Ok(root)=std::env::var("VAULT_NATIVE_TEST_DIR") else{return;};
        let c=Catalogue::load(Path::new(&root)).unwrap();
        // The snapshot may retain additional individual-stock aliases. Verify
        // every ready whitelist key instead of pinning an obsolete file count.
        let index=fs::read_to_string(Path::new(&root).join("index.csv")).unwrap();
        let mut lines=index.lines();let header=parse_csv_fields(lines.next().unwrap());
        let column=|name:&str|header.iter().position(|value|value==name).unwrap();
        let (status,basket,ticker)=(column("status"),column("basket"),column("ticker"));
        let expected=lines.filter(|line|!line.trim().is_empty()).map(parse_csv_fields)
            .filter(|row|row[status].trim()=="ready")
            .map(|row|format!("basket:{}:{}",row[basket].trim(),row[ticker].trim()))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(c.entries.keys().cloned().collect::<std::collections::BTreeSet<_>>(),expected);
        let key=c.entries.keys().find(|key|key.ends_with(":RELIANCE")).unwrap();let rows=c.rows(key).unwrap();assert!(rows.len()>1000);assert_eq!(rows[0].ticker,"RELIANCE");
        assert!(c.provenance_json(key).unwrap().contains("quote_accuracy_independently_verified\":false"));
    }
    fn us_index(f:&Fixture,basket:&str,symbol:&str)->String {
        let old=fs::read_to_string(f.0.join("index.csv")).unwrap();
        let mapped=match basket {"us_sp500"|"us_sp500_past"=>"SPX","us_nasdaq100"|"us_nasdaq100_past"=>"NDX","us_dow30"|"us_dow30_past"=>"DJI",_=>"RUT"};
        let kind=if basket.ends_with("_past"){"current_plus_past_snapshot"}else{"current_snapshot"};
        format!("{},trusted_market,currency,asset_type,timeframe,adjustment_status,source_price_units_verified,membership_kind,membership_source,membership_source_sha256,mapped_index\n{},us,USD,individual_stock,D,not_independently_verified,false,{kind},Exact test source list,{},{}\n",old.lines().next().unwrap(),old.lines().nth(1).unwrap().replace("nifty50,RELIANCE,",&format!("{basket},{symbol},")),"b".repeat(64),mapped)
    }
    #[test]
    fn trusted_native_metadata_comes_from_fixed_manifest_not_ticker_or_client_currency(){
        let f=fixture();let c=Catalogue::load(&f.0).unwrap();
        let e=c.native_evidence("basket:nifty50:RELIANCE").unwrap();assert_eq!(e.market(),Market::India);assert_eq!(e.currency(),"INR");
        assert!(c.native_evidence("basket:nifty50:RELIANCE?currency=USD").is_none());
        let old=fs::read_to_string(f.0.join("index.csv")).unwrap();
        fs::write(f.0.join("index.csv"),format!("{},currency\n{},USD\n",old.lines().next().unwrap(),old.lines().nth(1).unwrap())).unwrap();assert!(Catalogue::load(&f.0).is_err(),"An explicit USD assertion cannot change the Indian source market");
        fs::write(f.0.join("index.csv"),old).unwrap();
        let index=us_index(&f,"us_sp500","SPY");fs::write(f.0.join("index.csv"),index).unwrap();
        let c=Catalogue::load(&f.0).unwrap();let e=c.native_evidence("basket:us_sp500:SPY").unwrap();
        assert_eq!(e.market(),Market::UnitedStates);assert_eq!(e.currency(),"USD");assert_eq!(e.asset_type(),"individual_stock","Ticker spelling cannot turn the company-stock source into an ETF");
        assert!(!e.source_price_units_verified());assert_eq!(e.mapped_index(),"SPX");
        let provenance=c.provenance_json("basket:us_sp500:SPY").unwrap();assert!(provenance.contains("\"historical_membership_verified\":false"));assert!(provenance.contains("\"corporate_actions_independently_verified\":false"));
    }
    #[test]
    fn all_us_namespaces_require_explicit_matching_market_membership_and_asset_evidence(){
        for basket in ["us_sp500","us_sp500_past","us_nasdaq100","us_nasdaq100_past","us_dow30","us_dow30_past","us_russell2000","us_russell2000_past"] {
            let f=fixture();let index=us_index(&f,basket,"AABA-201910");fs::write(f.0.join("index.csv"),&index).unwrap();
            let c=Catalogue::load(&f.0).unwrap();assert!(c.contains(&format!("basket:{basket}:AABA-201910")));
            assert_eq!(c.rows(&format!("basket:{basket}:AABA-201910")).unwrap()[0].ticker,"AABA-201910");
            for (old,new) in [("us,USD","us,INR"),("us,USD","in,USD"),("individual_stock,D","etf,D"),("individual_stock,D","index,D"),("individual_stock,D","individual_stock,weekly"),("not_independently_verified,false","not_independently_verified,true"),("not_independently_verified,false","fully_verified,false"),("Exact test source list",""),("b".repeat(64).as_str(),"invalid_sha")] {
                fs::write(f.0.join("index.csv"),index.replace(old,new)).unwrap();assert!(Catalogue::load(&f.0).is_err(),"Must reject unsupported {old} -> {new}");
            }
            fs::write(f.0.join("index.csv"),index.replace("current_snapshot","point_in_time").replace("current_plus_past_snapshot","point_in_time")).unwrap();assert!(Catalogue::load(&f.0).is_err());
        }
        let f=fixture();let index=fs::read_to_string(f.0.join("index.csv")).unwrap();fs::write(f.0.join("index.csv"),index.replace("nifty50,","us_sp500,")).unwrap();assert!(Catalogue::load(&f.0).is_err(),"Legacy untyped US manifest cannot inherit INR stock evidence");
    }
    #[test]
    fn source_metadata_and_dates_are_not_silently_repaired(){
        let f=fixture();let index=us_index(&f,"us_sp500","AAPL");
        for modified in [index.replace("2020-01-02,2","2020-02-31,2"),index.replace("ready,true,us","ready,unknown,us"),index.replace(",SPX\n",",NDX\n")] {
            fs::write(f.0.join("index.csv"),modified).unwrap();assert!(Catalogue::load(&f.0).is_err());
        }
        fs::write(f.0.join("index.csv"),index.replace("2020-01-02,2","2020-01-03,2")).unwrap();let c=Catalogue::load(&f.0).unwrap();assert!(c.rows("basket:us_sp500:AAPL").unwrap_err().contains("coverage"));
        fs::write(f.0.join("index.csv"),index.replace("trusted_market,currency","trusted_market,currency,currency").replace(",us,USD,",",us,USD,INR,")).unwrap();assert!(Catalogue::load(&f.0).is_err(),"Conflicting duplicate metadata columns cannot be silently selected");
    }
}
