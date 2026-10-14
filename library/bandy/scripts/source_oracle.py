"""Independent Decimal source-array interpreter for the bounded QTS companion review.

No Rust output, source text, or indicators are imported. Contracts below were
transcribed from the supplied AFL and rendered book pages after errata review.
This verifies a declared research translation, not native AmiBroker exports.
"""
from decimal import Decimal as D, getcontext
getcontext().prec = 40

METHODS = {
 'hbqts_ma5_25': dict(figure='7_1', pages=[89], params=dict(fast=5, slow=25), family='hbqts_ma', rule='Buy Cross(MA(C,5),MA(C,25)); Sell reverse cross. Both prices C; delays zero.'),
 'hbqts_ma1_36': dict(figure='13_2', pages=[176], params=dict(fast=1, slow=36), family='hbqts_ma', rule='Buy Cross(MA(C,1),MA(C,36)); Sell reverse cross. Lengths are literal active constants, not commented Optimize defaults.'),
 'hbqts_limit_entry': dict(figure='7_2', pages=[89,90], params=dict(fast=5, slow=25, pct=1), family='hbqts_limit_entry', rule='BuySig Cross(MA(C,5),MA(C,25)); Buy on following bar only when L < .99*previous C. BuyPrice .99*previous C. Sell reverse cross at C. Zero delays.'),
 'hbqts_stop_entry': dict(figure='7_3', pages=[90], params=dict(hi=20, lo=10), family='hbqts_stop_entry', rule='Buy H>Ref(HHV(H,20),-1) at prior20 high; Sell L<Ref(LLV(L,10),-1) at prior10 low. ExRem Buy,Sell then Sell,Buy. Zero delays.', held='Entry and exit levels may both be touched on a daily bar. Source does not establish causal ordering; no ledger certification or readiness.'),
 'hbqts_fixed_interval': dict(figure='7_6', pages=[92,93], params=dict(every=6, hold=3), family='hbqts_fixed_interval', rule='Buy BarIndex()%6==0, C, zero delays; Sell BarsSince(raw Buy)>=3; Sell=ExRem(Sell,Buy). BarIndex starts at first loaded actual bar.'),
 'hbqts_profit_target': dict(figure='7_9', pages=[95,96,97], params=dict(fast=5, slow=25, tgt=10), family='hbqts_profit_target', rule='Raw Buy Cross(MA(C,5),MA(C,25)). Source scalar loop from bar0: if flat and Buy, set EntryPrice=C; ELSE if held and H>1.10*EntryPrice, emit Sell at target and reset. ExRem Buy,Sell then Sell,Buy; no entry-bar target check and no other exit.'),
 'hbqts_entry_id1': dict(figure='7_14', pages=[104], params=dict(fast=3, slow=16, hold=6), family='hbqts_entry_id1', rule='Buy Cross(MA(C,3),MA(C,16)); Sell BarsSince(raw Buy)>=6, THEN ExRem Buy,Sell and Sell,Buy. Raw repeats restart BarsSince; zero delays and prices C.'),
 'hbqts_bb_breakout': dict(figure='9_13', pages=[135], params=dict(lb=12, w=2.8, side=0), family='hbqts_bb_breakout', rule='Population SD bands MA(C,12)+/-2.8*SD. Buy Cross(C,top); Sell Cross(top,C); Short Cross(bottom,C); Cover Cross(C,bottom). Source defines both directions.'),
 'hbqts_cgo': dict(figure='10_1', pages=[137,138], params=dict(len=13, sm=2, hold=6), family='hbqts_cgo', rule='CGO initialized0; loop from i=13, weighted (1+j)*(H+L)/2 for j0..12; CGO=100*(-weighted/sum+7). Buy Cross(CGO,DEMA(CGO,2)); Sell reverse cross OR BarsSince(raw Buy)>=6; Sell ExRem only. Zero delays, C; SetBarsRequired(200,0).'),
 'hbqts_stoch_rsi': dict(figure='10_3', pages=[140], params=dict(rsi=10, lb=20, trig=3), family='hbqts_stoch_rsi', rule='r=RSIa(C,10); pir=(r-LLV(r,20))/(HHV(r,20)-LLV(r,20)); trigger=DEMA(pir,3); Buy Cross(pir,trigger); Sell reverse cross. Zero delays, C.'),
 'hbqts_apz': dict(figure='10_6', pages=[142,143], params=dict(period=50, band=2, adx_th=30, adx=14), family='hbqts_apz', rule='Double AMA alpha=2/(sqrt(50)+1) of C and H-L; dn=smoothed C-2*smoothed range; Buy ADX(14)<=30 AND L<=dn; Sell ADX(14)>30; ExRem Buy,Sell then Sell,Buy; zero delays, C.', held='Ten AAPL pre-2010 signal-field mismatches: f64 directional-movement predicates differ from exact decimal quote arithmetic on historical equal-move bars, shifting ADX threshold crossings. Selected trade ledgers match, but full array verification fails. No precision fudge or shared connors calculation change is authorized; held pending a separately validated quote/indicator precision contract.'),
 'hbqts_dpo': dict(figure='10_8', pages=[144], params=dict(ma=30, sm=10), family='hbqts_dpo', rule='Osc=C-MA(C,30); sm=DEMA(Osc,10); trigger=DEMA(sm,3); Buy Cross(sm,trigger); Sell reverse cross; zero delays, C. No positive Ref or displaced DPO.'),
 'hbqts_dom': dict(figure='11_3', pages=[150,151], params=dict(side=0), family='hbqts_dom', rule='Buy=Cover on calendar days1..5,10..16,21..31; Sell=Short on days6..9,17..20; zero delays. Buy/Sell=C; Short/Cover use declared Close settings.'),
 'hbqts_macd_daily': dict(figure='17_6', pages=[226,227], params=dict(p=5, q=37, r=7), family='hbqts_macd_daily', rule='SmDiff=EMA(EMA(C,5)-EMA(C,37),7); Permit=SmDiff>previous SmDiff, PermitSwitch=1. Buy Cross(MA(C,8),MA(C,7)) AND Permit; Sell reverse cross OR NOT Permit. Daily, no weekly compression.'),
 'hbqts_cmo': dict(figure='20_2', pages=[263,264], params=dict(pds=21, avg=96, hold=6), family='hbqts_cmo', rule='Sum positive/negative close changes over21; CMO=100*(up-down)/(up+down); avg=DEMA(CMO,96); trigger=DEMA(avg,3); Buy Cross(avg,trigger); Sell reverse cross OR BarsSince(raw Buy)>=6; ExRem Buy,Sell then Sell,Buy.'),
}
READY = [k for k,v in METHODS.items() if 'held' not in v]
for method, contract in METHODS.items():
 contract['instrument_requirement'] = 'etf' if method=='hbqts_cmo' else 'stock'
 contract['population_contract'] = ('Source XLB ETF walk-forward example; verify literal default signals on actual SPY ETF as a declared research application. No XLB or optimized walk-forward performance claim.' if method=='hbqts_cmo' else 'Selected actual AAPL and RELIANCE, each separately; no stock-universe membership or portfolio claim.')
 contract['parameter_basis'] = 'Active companion constants and Param/Optimize defaults; no parameters chosen for profitability. Existing engine fields allow research variants, including CMO hold6 which is a source constant rather than an AFL Param.'
COMMON = {
 'purpose': 'Single-symbol backtest/system signals; Equity/Plot calls are display only.',
 'inputs': 'Actual chronological daily OHLC. No volume, Foreign, calendar feed, ranking or auxiliary series in ready subset.',
 'statefulness': 'Vector arrays over all loaded actual warmup; negative Ref past only; ExRem assignment order retained; BarsSince uses pre-ExRem Buy; profit loop starts at bar0.',
 'timing': 'Explicit zero SetTradeDelays preserved. Where not set, same-bar Close settings from QTS pp65-66/88 are a declared research selection; not undocumented source constants. One position, no pyramiding; opposite close signals reverse. Held long cannot refill an intraday limit before its same-day close exit. When flat before a bar, its eligible prior-day limit may fill before the same bar close Sell (AllowSameBarExit enabled, HoldMinBars0 research settings).',
 'prices': 'Explicit BuyPrice/SellPrice retained. AFL research PriceBoundChecking clamps custom prices to observed low/high; this is not inferred open-gap price improvement. Limit Buy exists only on the next observed bar, strict low<limit, no pending order carry.',
 'stops': 'No ApplyStop calls in ready subset. Figure7.3 stop-entry is held. ApplyStop defaults/activation cannot be assumed across other source figures. N-bar exits here are formula BarsSince, not ApplyStop.',
 'indicator_conventions': 'Population SD and full-window MA/HHV/LLV. Stochastic RSI follows the AmiBroker published P=N=0 recurrence from bar1 with first ratio at period; a flat denominator is Null. MACD EMA uses a full-period SMA seed. DEMA uses first finite input alpha recursion; a nonfinite input remains missing without forward-filled output. Held APZ and its ADX are outside the executable reviewed helper. Native AmiBroker export parity is not claimed.',
 'lookahead': 'No positive Ref or future prices in ready subset. LastValue/Highest(Equity) and plotting in Figure13.2 do not feed signals. Formula history and BarIndex origin remain input-dependent.',
 'accounting': 'Price/direction ledger only. Flat account at selected start, formula-array state warmed from actual first bar. Final positions including final-bar entries stay open, marked at last close; no forced sale. No pooled portfolio, sizing, commissions, borrow, dividends, cash interest, equity claims or copied book performance.',
 'population': 'Stock rules: actual AAPL (US) and RELIANCE (India) separately. CMO: actual SPY ETF, declared application of the XLB source example. Retrospective selected samples, not historical NASDAQ100 membership, source513-stock population or source XLB walk-forward result. Current constituents do not establish historical membership. Stock rules cannot use Nifty or SPY as substitutes; ETFs cannot use stock/index prices.',
 'unknowns': 'Original AmiBroker database, QuickAFL history and native built-in seed/export parity are unavailable. Original source study accounting is not reproduced. Independent Python/Rust agreement checks the declared translation only.',
}

def avg(v,n):
 return [None if i+1<n or any(x is None for x in v[i+1-n:i+1]) else sum(v[i+1-n:i+1],D(0))/n for i in range(len(v))]

def smooth(v,alpha):
 out=[]; state=None
 for x in v:
  if x is None: out.append(None); continue
  state=x if state is None else state+alpha*(x-state)
  out.append(state)
 return out

def ema(v,n):
 out=[None]*len(v); first=next((i for i,x in enumerate(v) if x is not None),len(v))
 if first+n>len(v):return out
 state=sum(v[first:first+n],D(0))/n;out[first+n-1]=state
 for i in range(first+n,len(v)):
  if v[i] is not None:state=state+D(2)/(n+1)*(v[i]-state);out[i]=state
 return out
def dema(v,n):
 a=smooth(v,D(2)/(n+1)); b=smooth(a,D(2)/(n+1))
 return [None if x is None or y is None else x+(x-y) for x,y in zip(a,b)]
def div(a,b): return None if a is None or b is None or b==0 else a/b

def cross(a,b):
 return [i>0 and all(x is not None for x in (a[i],b[i],a[i-1],b[i-1])) and a[i]>b[i] and a[i-1]<=b[i-1] for i in range(len(a))]

def held(raw,n):
 previous=None; out=[]
 for i,hit in enumerate(raw):
  if hit: previous=i
  out.append(previous is not None and i-previous>=n)
 return out

def exrem(a,b):
 out=[]; latched=False
 for hit,reset in zip(a,b):
  emit=hit and not latched;out.append(emit)
  if emit:latched=True
  if reset:latched=False
 return out

def extremes(v,n,fn):
 return [None if i+1<n or any(x is None for x in v[i+1-n:i+1]) else fn(v[i+1-n:i+1]) for i in range(len(v))]

def wild(v,n):
 out=[None]*len(v)
 first=next((i for i,x in enumerate(v) if x is not None),len(v))
 if first+n>len(v):return out
 state=sum(v[first:first+n],D(0))/n;out[first+n-1]=state
 for i in range(first+n,len(v)):
  state=None if state is None or v[i] is None else (state*(n-1)+v[i])/n;out[i]=state
 return out

def rsi(c,n):
 out=[None]*len(c);up=dn=D(0)
 for i in range(1,len(c)):
  diff=c[i]-c[i-1];up=((n-1)*up+max(diff,D(0)))/n;dn=((n-1)*dn+max(-diff,D(0)))/n
  if i>=n and up+dn>0:out[i]=100*up/(up+dn)
 return out

def adx(rows,n):
 size=len(rows); pd=[None];md=[None];tr=[None]
 for i in range(1,size):
  h,l,c=rows[i]['high'],rows[i]['low'],rows[i-1]['close']
  u=h-rows[i-1]['high'];d=rows[i-1]['low']-l
  pd.append(u if u>d and u>0 else D(0));md.append(d if d>u and d>0 else D(0))
  tr.append(max(h-l,abs(h-c),abs(l-c)))
 a,b,t=wild(pd,n),wild(md,n),wild(tr,n);dx=[]
 for x,y,z in zip(a,b,t):
  if z is None or z<=0:dx.append(None)
  else:
   p,m=100*x/z,100*y/z;dx.append(D(0) if p+m==0 else 100*abs(p-m)/(p+m))
 return wild(dx,n)

def signals(method,rows,params):
 p={k:D(str(v)) for k,v in params.items()};u=lambda k:int(p[k])
 c,h,l=([r[k] for r in rows] for k in ('close','high','low'));n=len(rows)
 buy=[False]*n;sell=buy.copy();short=buy.copy();cover=buy.copy();bp=c.copy();sp=c.copy()
 if method in ('hbqts_ma5_25','hbqts_ma1_36','hbqts_limit_entry','hbqts_entry_id1','hbqts_profit_target'):
  a,b=avg(c,u('fast')),avg(c,u('slow'));buy=cross(a,b);sell=cross(b,a);raw=buy.copy()
  if method=='hbqts_limit_entry':
   bp=[None]+[c[i-1]*(1-p['pct']/100) for i in range(1,n)]
   buy=[i>0 and raw[i-1] and l[i]<bp[i] for i in range(n)]
  elif method=='hbqts_entry_id1':
   sell=held(buy,u('hold'));buy=exrem(buy,sell);sell=exrem(sell,buy)
  elif method=='hbqts_profit_target':
   sell=[False]*n;entry=None
   for i in range(n):
    if entry is None and buy[i]:entry=c[i]
    elif entry is not None and h[i]>entry*(1+p['tgt']/100):
     sell[i]=True;sp[i]=entry*(1+p['tgt']/100);entry=None
   buy=exrem(buy,sell);sell=exrem(sell,buy)
 elif method=='hbqts_fixed_interval':
  buy=[i%u('every')==0 for i in range(n)];raw=buy.copy();sell=exrem(held(buy,u('hold')),buy)
 elif method=='hbqts_bb_breakout':
  ma=avg(c,u('lb'));sd=[None if m is None else (sum((x-m)**2 for x in c[i+1-u('lb'):i+1])/u('lb')).sqrt() for i,m in enumerate(ma)]
  top=[None if m is None else m+p['w']*s for m,s in zip(ma,sd)];bot=[None if m is None else m-p['w']*s for m,s in zip(ma,sd)]
  buy=cross(c,top);sell=cross(top,c);short=cross(bot,c);cover=cross(c,bot);raw=buy.copy()
 elif method=='hbqts_cgo':
  price=[(a+b)/2 for a,b in zip(h,l)];cg=[D(0)]*n
  for i in range(u('len'),n):
   window=[price[i-j] for j in range(u('len'))];cg[i]=100*(-sum((j+1)*x for j,x in enumerate(window))/sum(window)+(p['len']+1)/2)
  trigger=dema(cg,u('sm'));buy=cross(cg,trigger);raw=buy.copy();sell=exrem([a or b for a,b in zip(cross(trigger,cg),held(buy,u('hold')))],buy)
 elif method=='hbqts_stoch_rsi':
  r=rsi(c,u('rsi'));lo=extremes(r,u('lb'),min);hi=extremes(r,u('lb'),max)
  pir=[None if a is None or b is None or d is None else div(a-b,d-b) for a,b,d in zip(r,lo,hi)]
  trigger=dema(pir,u('trig'));buy=cross(pir,trigger);sell=cross(trigger,pir);raw=buy.copy()
 elif method=='hbqts_apz':
  alpha=2/(p['period'].sqrt()+1);ds=smooth(smooth(c,alpha),alpha);rs=smooth(smooth([a-b for a,b in zip(h,l)],alpha),alpha);ax=adx(rows,u('adx'))
  buy=[v is not None and v<=p['adx_th'] and low<=mean-p['band']*ran for v,low,mean,ran in zip(ax,l,ds,rs)]
  sell=[v is not None and v>p['adx_th'] for v in ax];raw=buy.copy();buy=exrem(buy,sell);sell=exrem(sell,buy)
 elif method=='hbqts_dpo':
  osc=[None if m is None else a-m for a,m in zip(c,avg(c,u('ma')))];sm=dema(osc,u('sm'));trigger=dema(sm,3)
  buy=cross(sm,trigger);sell=cross(trigger,sm);raw=buy.copy()
 elif method=='hbqts_dom':
  days=[int(r['date'][8:10]) for r in rows];buy=[1<=d<=5 or 10<=d<=16 or 21<=d<=31 for d in days];short=[6<=d<=9 or 17<=d<=20 for d in days];sell=short.copy();cover=buy.copy();raw=buy.copy()
 elif method=='hbqts_macd_daily':
  sm=ema([a-b for a,b in zip(ema(c,u('p')),ema(c,u('q')))],u('r'));valid=[i>0 and sm[i] is not None and sm[i-1] is not None for i in range(n)];permit=[valid[i] and sm[i]>sm[i-1] for i in range(n)]
  a,b=avg(c,8),avg(c,7);buy=[a and b for a,b in zip(cross(a,b),permit)];sell=[cross(b,a)[i] or (valid[i] and not permit[i]) for i in range(n)];raw=buy.copy()
 elif method=='hbqts_cmo':
  up=[D(0)]+[max(c[i]-c[i-1],D(0)) for i in range(1,n)];dn=[D(0)]+[max(c[i-1]-c[i],D(0)) for i in range(1,n)]
  a,b=avg(up,u('pds')),avg(dn,u('pds'));cm=[None if x is None or y is None else div(100*(x-y),x+y) for x,y in zip(a,b)]
  sm=dema(cm,u('avg'));trigger=dema(sm,3);buy=cross(sm,trigger);raw=buy.copy();sell=[a or b for a,b in zip(cross(trigger,sm),held(buy,u('hold')))];buy=exrem(buy,sell);sell=exrem(sell,buy)
 else:raise ValueError(method)
 return [dict(bar=i,raw_buy=raw[i],buy=buy[i],sell=sell[i],short=short[i],cover=cover[i],buy_px=bp[i],sell_px=sp[i]) for i in range(n)]

def ledger(method,rows,first,last,params,arrays=None):
 s=arrays if arrays is not None else signals(method,rows,params);out=[];pos=None;side=int(params.get('side',0)) if method in ('hbqts_bb_breakout','hbqts_dom') else 1
 clamp=lambda i,p:max(rows[i]['low'],min(rows[i]['high'],p))
 fill='limit' if method=='hbqts_limit_entry' else 'close'
 for i in range(first,last+1):
  if method=='hbqts_limit_entry':
   if pos:
    if s[i]['sell']:
     pos.update(exit=i,exit_date=rows[i]['date'],exit_px=clamp(i,s[i]['sell_px']),open=False,exit_kind='close');out.append(pos);pos=None
   elif s[i]['buy']:
    pos=dict(entry=i,entry_date=rows[i]['date'],entry_px=clamp(i,s[i]['buy_px']),dir=1,entry_kind='limit')
    if s[i]['sell']:
     pos.update(exit=i,exit_date=rows[i]['date'],exit_px=clamp(i,s[i]['sell_px']),open=False,exit_kind='close');out.append(pos);pos=None
   continue
  exited=None
  if pos and (s[i]['sell'] if pos['dir']>0 else s[i]['cover']):
   xp=clamp(i,s[i]['sell_px']) if pos['dir']>0 else rows[i]['close'];exited=pos['dir']
   pos.update(exit=i,exit_date=rows[i]['date'],exit_px=xp,open=False,exit_kind='target' if method=='hbqts_profit_target' else 'close');out.append(pos);pos=None
  if pos is None:
   d=1 if side!=2 and s[i]['buy'] else -1 if side!=1 and s[i]['short'] else 0
   if d and d!=exited:pos=dict(entry=i,entry_date=rows[i]['date'],entry_px=clamp(i,s[i]['buy_px']) if d>0 else rows[i]['close'],dir=d,entry_kind=fill)
 if pos:pos.update(exit=last,exit_date=rows[last]['date'],exit_px=rows[last]['close'],open=True,exit_kind='mark');out.append(pos)
 return out
