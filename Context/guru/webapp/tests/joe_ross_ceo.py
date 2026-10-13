#!/usr/bin/env python3
"""Real authenticated API/browser checks; source references cannot manufacture P&L."""
import asyncio,json,os,re,zipfile,xml.etree.ElementTree as ET
from pathlib import Path
from playwright.async_api import async_playwright,expect
ENV=Path(os.environ.get('VAULT_ENV_FILE',Path.home()/'.cache/strategy-vault-webapp/.env'))
config=dict(l.split('=',1) for l in ENV.read_text().splitlines() if l and not l.startswith('#'))
URL=os.environ.get('VAULT_TEST_URL',config['VAULT_PUBLIC_URL'])
OUT=Path.home()/'.cache/strategy-vault-webapp/browser-verification';OUT.mkdir(parents=True,exist_ok=True)
NS={'x':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
SPEC=Path(__file__).resolve().parents[4]/'library/joe-ross/core-specifications.json'
METHODS=json.loads(SPEC.read_text())['methods']
assert all(m.get('missing_data') and m.get('rule_gaps') for m in METHODS)
EXPECTED_ROSS={m['key']+('_l' if direction=='Long' else '_s')
 for m in METHODS for direction in m['directions']}
def sheet_rows(path,sheet=1):
 with zipfile.ZipFile(path) as z:
  strings=[]
  if 'xl/sharedStrings.xml' in z.namelist():strings=[''.join(n.itertext()) for n in ET.fromstring(z.read('xl/sharedStrings.xml')).findall('x:si',NS)]
  rows=[]
  for row in ET.fromstring(z.read(f'xl/worksheets/sheet{sheet}.xml')).findall('.//x:sheetData/x:row',NS):
   cells={}
   for cell in row.findall('x:c',NS):
    col=''.join(c for c in cell.attrib['r'] if c.isalpha());idx=0
    for c in col:idx=idx*26+ord(c)-64
    value=cell.find('x:v',NS);kind=cell.get('t')
    if kind=='s':v=strings[int(value.text)]
    elif kind=='inlineStr':v=''.join(cell.find('x:is',NS).itertext())
    elif kind=='str' and value is not None:v=value.text
    elif value is not None:v=float(value.text)
    else:v=None
    cells[idx-1]=v
   rows.append(cells)
  return rows
async def main():
 checks=[];errors=[];requests=[];original=None
 async with async_playwright() as p:
  browser=await p.chromium.launch(executable_path='/usr/bin/google-chrome',headless=True,args=['--disable-dev-shm-usage'])
  ctx=await browser.new_context(viewport={'width':1440,'height':1000});page=await ctx.new_page()
  page.on('pageerror',lambda e:errors.append(str(e)))
  page.on('request',lambda r:requests.append(r.url))
  try:
   r=await ctx.request.post(URL+'/api/auth/sign-in/email',data={'email':config['VAULT_ADMIN_EMAIL'],'password':config['VAULT_ADMIN_PASSWORD']},headers={'Origin':URL});assert r.status==200
   original=(await (await ctx.request.get(URL+'/api/app/me')).json())['settings']
   async def api(market,guru,endpoint,params=None):return await ctx.request.get(URL+f'/api/{market}/guru/{guru}/{endpoint}',params=params)
   gurus=await (await ctx.request.get(URL+'/api/in/gurus')).json();assert next(g for g in gurus if g['id']=='ross')['testable']==0
   for market,tk in [('in','NIFTY 50'),('us','SPX')]:
    r=await api(market,'ross','strategies');assert r.status==200;cat=await r.json();assert len(cat)==len(EXPECTED_ROSS)
    assert {v['id'] for v in cat}==EXPECTED_ROSS
    assert all(v['book'].startswith('jr_') and v['claim'] is None and v['status']!='live' for v in cat)
    old=await (await api(market,'connors','strategies')).json();assert all(not v['book'].startswith('jr_') for v in old)
    q={'ticker':tk,'from':'2020-01-01','to':'2021-01-01'}
    summary=await (await api(market,'ross','summary',q)).json();assert len(summary)==len(EXPECTED_ROSS) and all(v.get('blocked') and 'stats' not in v and 'spark' not in v for v in summary)
    assert {v['id'] for v in summary}==EXPECTED_ROSS
    assert await (await api(market,'ross','params')).json()=={}
    for v in cat:
     query={**q,'id':v['id']};state=await (await api(market,'ross','readiness',query)).json();assert state['strategies'][0]['status']!='ready'
     for endpoint in ['run','optimize']:
      r=await api(market,'ross',endpoint,query);assert r.status==400;body=await r.text();assert not any('"'+k+'"' in body for k in ['stats','trades','equity','rows'])
    for guru,sid in [('connors','jr_gimmee_l'),('ross','f1a')]:
     for endpoint in ['readiness','run','optimize']:
      r=await api(market,guru,endpoint,{**q,'id':sid,'_guru':'ross' if guru=='connors' else 'connors'});assert r.status==400
    checks.append(market+f': all {len(EXPECTED_ROSS)} Ross references withheld; ownership, claims, summary and optimization checked')
   await page.goto(URL+'/full-variant1.html#/');card=page.locator('[data-source-research] a[href*="/guru/ross"]');await expect(card).to_be_visible(timeout=30000)
   await expect(page.locator('.legend-card[href*="/guru/ross"]')).to_have_count(0)
   await expect(card).not_to_contain_text('Strongest:');await expect(card).not_to_contain_text('Sharpe')
   for label in ['Data missing','Rules unresolved','0 implemented backtests']:await expect(card).to_contain_text(label)
   await card.click();await expect(page.locator('[data-ross-review]')).to_contain_text('full corpus review in progress',timeout=30000)
   await expect(page.locator('.bookcard[data-book]')).to_have_count(4)
   await expect(page.locator('.bc-stats')).to_have_count(0)
   await expect(page.locator('[data-source-counts]')).to_contain_text(re.compile(r'Implemented backtests\s*0'))
   await page.screenshot(path=str(OUT/'joe-ross-library.png'),full_page=True)
   await page.locator('[data-book="jr_manual"]').click();await expect(page.locator('#results')).to_contain_text('DATA MISSING · RULES UNRESOLVED',timeout=30000)
   await expect(page.locator('#results svg')).to_have_count(0)
   await expect(page.locator('#results [data-blocker="needs_data"]').first).to_contain_text('Actual traded futures')
   await expect(page.locator('#results [data-blocker="not_testable"]').first).to_contain_text('Rules unresolved')
   checks.append('Ross excluded from implemented legends; separate source research, four core sources, zero backtests and explicit independent data/rule blockers')
   for region,tk in [('in','NIFTY%2050'),('us','SPX')]:
    await page.goto(URL+f'/full-variant1.html#/s/jr_gimmee_l?region={region}&tk={tk}')
    await expect(page.locator('#gpt-c')).to_contain_text('Backtest withheld',timeout=30000)
    await expect(page.locator('.tags2')).to_contain_text('Nifty 50' if region=='in' else 'S&P 500')
    await expect(page.locator('#gpt-c')).to_contain_text('95.45%')
    await expect(page.locator('[data-blocker="needs_data"]')).to_contain_text('Actual traded futures')
    await expect(page.locator('[data-blocker="not_testable"]')).to_contain_text('Rules unresolved')
    await expect(page.locator('[data-blocker="source_missing"]')).to_contain_text('p.115')
    for selector in ['.metrics','#c-eq','#c-px','#optim','#csv','#claude-c','#cc-xlsx']:assert await page.locator(selector).count()==0
   await page.set_viewport_size({'width':390,'height':844});await page.evaluate("Promise.all(document.getAnimations().filter(a=>a.effect.getTiming().iterations!==Infinity).map(a=>a.finished.catch(()=>{})))")
   await page.screenshot(path=str(OUT/'joe-ross-gimmee-us-mobile.png'),full_page=True);assert await page.evaluate('document.documentElement.scrollWidth<=innerWidth')
   await page.set_viewport_size({'width':1440,'height':1000})
   await page.goto(URL+'/full-variant1.html#/o/jr_hook_l');await expect(page.locator('#gpt-c')).to_contain_text('Backtest withheld',timeout=30000);assert await page.locator('#orun').count()==0
   checks.append('India/US and direct optimization links withhold unsupported artifacts; mobile checked')
   for region in ['in','us']:
    for sid,detail in [('jr_keltner_l','15 minutes after'),
                       ('jr_reverse_hook_l','volume'),
                       ('jr_slaughterbeck_l','Candidate recognition'),
                       ('jr_cci_hook_s','factor-of-100'),
                       ('jr_envelope_hook_l','modern EMA/ATR'),
                       ('jr_channel_reversal_s','regression channel')]:
     await page.goto(URL+f'/full-variant1.html#/s/{sid}?region={region}')
     await expect(page.locator('[data-backtest-state]')).to_contain_text(detail,timeout=30000) if sid in ['jr_keltner_l','jr_reverse_hook_l'] else await expect(page.locator('#gpt-c')).to_contain_text(detail,timeout=30000)
     await expect(page.locator('#gpt-c')).to_contain_text('Backtest withheld')
     for selector in ['.metrics','#c-eq','#c-px','#optim','#csv','#claude-c','#cc-xlsx']:assert await page.locator(selector).count()==0
   checks.append('New SE, CCI, envelope and channel references show their specific source assessments on both markets, with no performance artifacts')
   requests.clear()
   await page.goto(URL+'/full-variant1.html#/s/f1a?from=2020-01-01&to=2025-12-31&cost=3&p=hold:6')
   await expect(page.locator('#c-eq')).to_be_visible(timeout=30000);await expect(page.locator('#gpt-c')).to_contain_text('source fidelity not certified')
   await expect(page.locator('#optim')).to_have_count(0);await expect(page.locator('#cc-xlsx')).to_have_count(0)
   await page.evaluate("Promise.all(document.getAnimations().filter(a=>a.effect.getTiming().iterations!==Infinity).map(a=>a.finished.catch(()=>{})))")
   assert not any('/optimize?' in u for u in requests)
   await page.screenshot(path=str(OUT/'ceo-compact-strategy.png'),full_page=True)
   await page.locator('#open-optimization').click();await expect(page.locator('#optim')).to_be_visible(timeout=30000)
   assert '#/o/f1a' in page.url and 'p=hold%3A6' in page.url and 'cost=3' in page.url
   await expect(page.locator('#cc-xlsx')).to_be_visible(timeout=120000)
   assert await page.locator('#gpt-c').evaluate('(e)=>e.previousElementSibling.id')=='claude-c'
   assert await page.locator('#c-eq').count()==0
   # Wait for the saved route, then prove PostgreSQL restoration of the optimization view.
   await expect(page.locator('#otime')).to_be_visible()
   await page.wait_for_timeout(700)
   saved=(await (await ctx.request.get(URL+'/api/app/me')).json())['settings']['lastView'];assert saved.startswith('#/o/'),saved
   await page.goto(URL+'/full-variant1.html');await expect(page.locator('#optim')).to_be_visible(timeout=30000)
   await expect(page.locator('#cc-xlsx')).to_be_visible(timeout=120000)
   checks.append('C03/C18: compact main page, preserved context, separate research, GPT below Claude and saved route')
   # Small real grid: vary only hold, three values. All other settings remain book defaults.
   for row in await page.locator('#optim tr[data-k]').all():
    check=row.locator('input[data-f="on"]');await check.uncheck()
   row=page.locator('#optim tr[data-k="hold"]');await row.locator('[data-f="on"]').check()
   for field,value in [('min','2'),('max','4'),('step','1')]:await row.locator(f'[data-f="{field}"]').fill(value)
   async with page.expect_response(lambda r:'/optimize?' in r.url and 'g.hold=2%3A4%3A1' in r.url,timeout=60000) as pending:await page.locator('#orun').click()
   response=await pending.value;assert response.status==200;res=await response.json();assert res['combos']==3
   (OUT/'full-grid-response.json').write_text(json.dumps(res,indent=2))
   await expect(page.locator('#grid-xlsx')).to_have_text('Download all 3 rows',timeout=30000)
   async with page.expect_download(timeout=60000) as pending:await page.locator('#grid-xlsx').click()
   download=await pending.value;path=OUT/download.suggested_filename;await download.save_as(str(path));rows=sheet_rows(path)
   assert len(rows)==res['combos']+1
   schema=await (await api('in','connors','params')).json();settings=schema['f1a'];nd=len(res['dims']);metrics=res['cols'][nd:]
   assert [rows[0].get(i) for i in range(1+len(metrics),len(rows[0]))]==[s['label'] for s in settings]
   for i,apirow in enumerate(res['rows'],1):
    assert rows[i][0]==i
    for j,value in enumerate(apirow[nd:],1):
     got=rows[i].get(j);assert got==value or (got is not None and value is not None and abs(got-value)<1e-12),(i,j,got,value)
    for j,param in enumerate(settings,1+len(metrics)):
     idx=next((k for k,d in enumerate(res['dims']) if d['key']==param['key']),None);assert rows[i].get(j)==(apirow[idx] if idx is not None else param['def'])
   checks.append('C04: workbook contains all three real grid rows, every metric and all fixed/varied parameters at the end; values match API')
   await page.screenshot(path=str(OUT/'ceo-optimization.png'),full_page=True)
   # Delay only the local Excel library; changing market must not relabel an
   # export already requested for India/cost 3. No strategy response is mocked.
   library_started=asyncio.Event();release_library=asyncio.Event()
   async def slow_library(route):
    library_started.set();await release_library.wait();await route.continue_()
   await page.route('**/vendor/xlsx-js-style-1.2.0.min.js',slow_library)
   await page.evaluate('window.XLSX=null')
   async with page.expect_download(timeout=60000) as pending:
    await page.locator('#grid-xlsx').click();await asyncio.wait_for(library_started.wait(),15)
    await page.locator('.region button').get_by_text('US',exact=True).click()
    await expect(page.locator('#gpt-c')).to_contain_text('S&P 500',timeout=30000)
    release_library.set()
   download=await pending.value;race_path=OUT/'grid-navigation-metadata.xlsx';await download.save_as(str(race_path))
   meta={row.get(0):row.get(1) for row in sheet_rows(race_path,2)}
   assert meta['Market']=='in' and meta['Instrument']=='NIFTY 50' and meta['Cost bps per side']==3,meta
   assert len(sheet_rows(race_path))==4
   await page.unroute('**/vendor/xlsx-js-style-1.2.0.min.js',slow_library)
   checks.append('In-flight Excel export keeps its original market, instrument, costs and rows while navigating')
   await page.locator('#open-optimization').click();await expect(page.locator('#c-eq')).to_be_visible(timeout=30000)
   # Browser-local note fixtures test delayed region routing, without writing
   # company notes or substituting any price/performance data.
   note_started=asyncio.Event();release_note=asyncio.Event()
   async def slow_note(route):
    if 'key=f1a.in' in route.request.url:
     note_started.set();await release_note.wait();await route.fulfill(status=200,content_type='text/plain',body='India routing fixture')
    else:await route.fulfill(status=200,content_type='text/plain',body='US routing fixture')
   await page.route('**/notes?key=f1a.*',slow_note)
   await page.locator('.region button').get_by_text('India',exact=True).click()
   await asyncio.wait_for(note_started.wait(),30)
   await expect(page.locator('#yt-text')).to_be_visible(timeout=30000)
   await page.locator('.region button').get_by_text('US',exact=True).click()
   await expect(page.locator('#yt-text')).to_have_value('US routing fixture',timeout=30000)
   async with page.expect_response(lambda r:'key=f1a.in' in r.url):release_note.set()
   await page.wait_for_timeout(200)
   await expect(page.locator('#yt-text')).to_have_value('US routing fixture')
   await page.unroute('**/notes?key=f1a.*',slow_note)
   checks.append('A delayed India note cannot overwrite the same strategy\'s US note after navigation')
   assert not errors,errors
   report={'checks':checks,'page_errors':errors,'grid_rows_verified':res['combos'],'sources_reviewed':'Core sources only; full corpus review in progress'}
   (OUT/'joe-ross-ceo-report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
  except Exception:
   print(json.dumps({'page_errors':errors,'export_message':await page.locator('#grid-xlsx').all_text_contents()},indent=2),flush=True)
   await page.screenshot(path=str(OUT/'joe-ross-ceo-failure.png'),full_page=True)
   raise
  finally:
   if original is not None:await ctx.request.put(URL+'/api/app/settings',data=json.dumps(original),headers={'Origin':URL,'Content-Type':'application/json'})
   await ctx.close();await browser.close()
if __name__=='__main__':asyncio.run(main())
