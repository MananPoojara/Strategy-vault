#!/usr/bin/env python3
"""Authenticated, sequential analyst presentation checks using genuine Rust results.

No market results are intercepted or substituted. Settings are restored at exit.
"""
import asyncio
import json
import math
import os
import re
from pathlib import Path, PurePosixPath
import xml.etree.ElementTree as ET
import zipfile
from playwright.async_api import async_playwright, expect

OUT=Path.home()/'.cache/strategy-vault-webapp/browser-verification'
NS={'x':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
KEYS=['cagr','net_profit','max_dd','win_rate','sharpe','monthly_thickness','trades','expectancy_ratio','car_mdd']
LABELS=['CAGR','Net profit','Max DD','Win %','Sharpe','Monthly thickness','Trades','Expectancy ratio','CAR/MDD']

def formatted(key,value):
    if not isinstance(value,(int,float)) or not math.isfinite(value):return '–'
    if key in ['cagr','net_profit','max_dd','win_rate','monthly_thickness']:
        sign='+' if value>0 and key in ['cagr','net_profit','monthly_thickness'] else ''
        return sign+f'{value*100:.2f}%'
    return f'{value:.0f}' if key=='trades' else f'{value:.2f}'

def workbook(path):
    with zipfile.ZipFile(path) as archive:
        strings=[]
        if 'xl/sharedStrings.xml' in archive.namelist():
            strings=[''.join(n.itertext()) for n in ET.fromstring(archive.read('xl/sharedStrings.xml')).findall('x:si',NS)]
        relationships={n.get('Id'):n.get('Target') for n in ET.fromstring(archive.read('xl/_rels/workbook.xml.rels'))}
        out={}
        for sheet in ET.fromstring(archive.read('xl/workbook.xml')).findall('x:sheets/x:sheet',NS):
            rid=sheet.get('{http://schemas.openxmlformats.org/officeDocument/2006/relationships}id')
            target=relationships[rid]
            name=target.lstrip('/') if target.startswith('/') else str(PurePosixPath('xl')/target)
            cells={}
            for cell in ET.fromstring(archive.read(name)).findall('.//x:sheetData/x:row/x:c',NS):
                node=cell.find('x:v',NS);kind=cell.get('t');formula=cell.find('x:f',NS)
                if kind=='s' and node is not None:value=strings[int(node.text)]
                elif kind=='inlineStr':value=''.join(cell.find('x:is',NS).itertext())
                elif kind=='str' and node is not None:value=node.text
                elif node is not None:value=float(node.text)
                else:value=None
                cells[cell.get('r')]={'value':value,'formula':formula.text if formula is not None else None}
            out[sheet.get('name')]=cells
        return out

async def main():
    env=Path(os.environ.get('VAULT_ENV_FILE',Path.home()/'.cache/strategy-vault-webapp/.env'))
    config=dict(line.split('=',1) for line in env.read_text().splitlines() if line and not line.startswith('#'))
    url=os.environ.get('VAULT_TEST_URL',config['VAULT_PUBLIC_URL']).rstrip('/')
    OUT.mkdir(parents=True,exist_ok=True)
    checks=[];errors=[];original=None;download_name=None
    async with async_playwright() as p:
        browser=await p.chromium.launch(executable_path='/usr/bin/google-chrome',headless=True,args=['--disable-dev-shm-usage'])
        context=await browser.new_context(viewport={'width':1440,'height':1000},accept_downloads=True)
        page=await context.new_page();page.on('pageerror',lambda error:errors.append(str(error)))
        try:
            auth=await context.request.post(url+'/api/auth/sign-in/email',data={'email':config['VAULT_ADMIN_EMAIL'],'password':config['VAULT_ADMIN_PASSWORD']},headers={'Origin':url})
            assert auth.status==200,'Authenticated session could not be established'
            me=await context.request.get(url+'/api/app/me');assert me.status==200
            original=(await me.json())['settings']
            await page.goto(url+'/full-variant1.html#/guru/ross')
            portrait=page.locator('.portrait-ross img');await expect(portrait).to_be_visible(timeout=30000)
            await page.wait_for_function("(()=>{const image=document.querySelector('.portrait-ross img');return image&&image.complete&&image.naturalWidth>0;})()")
            dimensions=await portrait.evaluate('(image)=>({width:image.naturalWidth,height:image.naturalHeight,src:image.currentSrc,fit:getComputedStyle(image).objectFit})')
            assert dimensions['width']==317,dimensions
            assert '/img/gurus/joe-ross.png' in dimensions['src'] and dimensions['src'].startswith(url)
            assert dimensions['fit']=='contain',dimensions
            credit=page.locator('.photo-credit a');await expect(credit).to_have_text('Trading Educators')
            assert (await credit.get_attribute('href')).startswith('https://tradingeducators.com/')
            await page.screenshot(path=str(OUT/'analyst-ross-portrait-desktop.png'),full_page=True)
            await page.set_viewport_size({'width':390,'height':844})
            await expect(portrait).to_be_visible();assert await page.evaluate('document.documentElement.scrollWidth<=innerWidth')
            await page.screenshot(path=str(OUT/'analyst-ross-portrait-mobile.png'),full_page=True)
            await page.set_viewport_size({'width':1440,'height':1000})
            checks.append('Original 317px Ross portrait is local, attributed and contained on desktop/mobile')

            await page.goto(url+'/full-variant1.html#/o/jr_hook_l?region=in&tk=NIFTY%2050')
            await expect(page.locator('#gpt-c')).to_contain_text('Backtest withheld',timeout=30000)
            await expect(page.locator('#gpt-c .gpt-evidence tbody tr')).to_have_count(6)
            for label in ['Source record','Rule assessment','Available evidence','Missing or unresolved evidence','Optimization assessment','Next prerequisite']:
                await expect(page.locator('#gpt-c')).to_contain_text(label)
            for selector in ['#gpt-more','#gpt-xlsx','#gpt-body','.metrics','#c-eq','#c-px','#optim','#cc-xlsx']:
                assert await page.locator(selector).count()==0,selector
            await page.screenshot(path=str(OUT/'analyst-ross-withheld.png'),full_page=True)
            checks.append('Blocked Ross has all six structured research checks and no candidate/performance artifacts')

            for region,ticker,encoded in [('in','NIFTY 50','NIFTY%2050'),('us','SPX','SPX')]:
                await page.goto(url+f'/full-variant1.html#/o/f1a?region={region}&tk={encoded}&from=2020-01-01&to=2025-12-31&cost=3')
                await page.wait_for_function("expected=>OPT.last?.region===expected.region&&OPT.last?.tk===expected.ticker&&OPT.last?.selectionLabel==='All history'",arg={'region':region,'ticker':ticker},timeout=180000)
                await expect(page.locator('#gpt-xlsx')).to_be_visible(timeout=180000)
                await expect(page.locator('#gpt-more')).to_be_visible()
                assert await page.locator('#gpt-c').evaluate('(element)=>element.previousElementSibling.id')=='claude-c'
                assert await page.locator('#gpt-detail').evaluate('(element)=>element.hidden&&element.innerHTML===""')
                captured=await page.evaluate('(()=>{const D=OPT.last;return {cost:D.cost,region:D.region,tk:D.tk,selectionLabel:D.selectionLabel,selection:{from:D.res.from,to:D.res.to},runs:D.runs.map(r=>({name:r.name,book:!!r.book,p:r.p,m:r.m,c:r.c,before:r.before?.m,after:r.after?.m})),groups:D.res.rank.groups};})()')
                assert captured['region']==region and captured['tk']==ticker and captured['cost']==3,captured
                assert captured['selectionLabel']=='All history'
                assert len(captured['runs'])==4 and len(captured['groups'])==5
                table=page.locator('#gpt-body table.cc-tbl').first
                headers=await table.locator('thead th').all_text_contents()
                assert headers[2:11]==LABELS,headers
                rows=table.locator('tbody tr');await expect(rows).to_have_count(4)
                for index,run in enumerate(captured['runs']):
                    cells=await rows.nth(index).locator('td').all_text_contents()
                    assert cells[0]==run['name']
                    for offset,key in enumerate(KEYS,2):
                        assert cells[offset]==formatted(key,run['m'].get(key)),(region,index,key,cells[offset],run['m'].get(key))
                await page.locator('#gpt-more').click();await expect(page.locator('#gpt-detail')).to_be_visible()
                group_headings=page.locator('#gpt-detail h4').filter(has_text=re.compile(r'^Top 5 by '))
                await expect(group_headings).to_have_count(5)
                for heading in await group_headings.all():
                    group_headers=await heading.evaluate('(element)=>Array.from(element.nextElementSibling.querySelectorAll("thead th"),node=>node.textContent)')
                    assert group_headers[2:]==['CAGR','Sharpe','Monthly thickness','Expectancy ratio','CAR/MDD'],group_headers
                await expect(page.locator('#gpt-detail svg')).to_have_count(2)
                for text in ['peak reset each calendar year','Cross-comparison of the candidates','Research limits','untouched holdout','independent confirmations','local sensitivity']:
                    await expect(page.locator('#gpt-detail')).to_contain_text(text)
                # These are actual trade/month summaries rendered by the detailed analyst section.
                await expect(page.locator('#gpt-detail [data-gpt-trades]')).to_have_count(4)
                await expect(page.locator('#gpt-detail [data-gpt-monthly]')).to_have_count(4)
                await page.screenshot(path=str(OUT/f'analyst-f1a-{region}-expanded.png'),full_page=True)
                await page.locator('#gpt-body').scroll_into_view_if_needed()
                await page.screenshot(path=str(OUT/f'analyst-f1a-{region}-viewport.png'))
                checks.append(region+': nine displayed statistics equal genuine book/top-three Rust results; groups, charts, limits, trades and monthly summaries expand')
                if region=='in':
                    (OUT/'analyst-india-real-runs.json').write_text(json.dumps(captured,indent=2))
                    async with page.expect_download(timeout=60000) as pending:await page.locator('#gpt-xlsx').click()
                    download=await pending.value;download_name=download.suggested_filename
                    path=OUT/download_name;await download.save_as(str(path));sheets=workbook(path)
                    for required in ['Summary','Formulas','Method','MoM','Drawdown','Daily','Trades Book','Trades Top 1','Trades Top 2','Trades Top 3']:assert required in sheets,(required,list(sheets))
                    summary=sheets['Summary'];text=[c['value'] for c in summary.values() if isinstance(c['value'],str)]
                    assert any('GPT' in value and 'conclusion' in value for value in text)
                    assert any('fees 3 + slippage 0 bps a side' in value for value in text)
                    assert any('All history' in value and captured['selection']['from'] in value for value in text)
                    for index,run in enumerate(captured['runs']):
                        label='Book' if index==0 else f'Top {index}'
                        address=next(address for address,c in summary.items() if address.startswith('A') and c['value']==label)
                        row=''.join(char for char in address if char.isdigit())
                        for offset,key in enumerate(KEYS):
                            got=summary.get(chr(ord('C')+offset)+row,{}).get('value');expected=run['m'].get(key)
                            assert got==expected or (isinstance(got,(int,float)) and isinstance(expected,(int,float)) and abs(got-expected)<1e-12),(index,key,got,expected)
                    formulas=sheets['Formulas'];assert sum(c['formula'] is not None for c in formulas.values())>=36
                    checks.append('India GPT workbook retains captured costs/title/context, required detail sheets and all 36 actual summary statistics')
                    # A complete three-row real grid permits neighborhood evidence;
                    # automatic slim results above must not pretend to provide it.
                    for row in await page.locator('#optim tr[data-k]').all():await row.locator('input[data-f="on"]').uncheck()
                    hold=page.locator('#optim tr[data-k="hold"]');await hold.locator('[data-f="on"]').check()
                    for field,value in [('min','2'),('max','4'),('step','1')]:await hold.locator(f'[data-f="{field}"]').fill(value)
                    async with page.expect_response(lambda response:'/optimize?' in response.url and 'g.hold=2%3A4%3A1' in response.url,timeout=60000) as pending:await page.locator('#orun').click()
                    response=await pending.value;assert response.status==200;grid=await response.json();assert grid['combos']==3
                    await expect(page.locator('#gpt-grid-review')).to_be_visible(timeout=30000)
                    await page.locator('#gpt-grid-review').click()
                    await page.wait_for_function("()=>OPT.last?.selectionLabel==='Manual grid window'&&OPT.last?.res.combos===3",timeout=60000)
                    await expect(page.locator('#gpt-grid-review')).to_have_text('GPT analysis of this grid')
                    manual=await page.evaluate('(()=>{const D=OPT.last,A=VaultAnalyst.analyze(D);return {selectionLabel:D.selectionLabel,from:D.res.from,to:D.res.to,cost:D.cost,rows:D.res.rows,runs:D.runs.map(r=>({name:r.name,p:r.p,m:r.m})),candidates:A.candidates};})()')
                    assert manual['selectionLabel']=='Manual grid window' and manual['cost']==3 and manual['rows']==grid['rows']
                    assert manual['from']==grid['from'] and manual['to']==grid['to']
                    centered=next(candidate for candidate in manual['candidates'] if candidate['parameters']['hold']==3)
                    assert centered['neighborhood']['available'] and centered['neighborhood']['inspected']==2
                    cagr_idx=grid['cols'].index('cagr');neighbor_cagrs=[grid['rows'][index][cagr_idx] for index in centered['neighborhood']['rows']]
                    cagr_range=next(metric for metric in centered['neighborhood']['metrics'] if metric['key']=='cagr')
                    assert cagr_range['min']==min(neighbor_cagrs) and cagr_range['max']==max(neighbor_cagrs)
                    await expect(page.locator('#gpt-body')).to_contain_text('Manual grid window')
                    await page.locator('#gpt-more').click();await expect(page.locator('#gpt-detail')).to_contain_text('One-step sensitivity · 2 neighbors')
                    manual_rows=page.locator('#gpt-body table.cc-tbl').first.locator('tbody tr')
                    for index,run in enumerate(manual['runs']):
                        actual=await manual_rows.nth(index).locator('td').all_text_contents()
                        for offset,key in enumerate(KEYS,2):assert actual[offset]==formatted(key,run['m'].get(key)),(index,key,actual[offset])
                    async with page.expect_download(timeout=60000) as pending:await page.locator('#gpt-xlsx').click()
                    manual_download=await pending.value;manual_path=OUT/'analyst-manual-grid.xlsx';await manual_download.save_as(str(manual_path));manual_sheets=workbook(manual_path)
                    manual_text=[cell['value'] for cell in manual_sheets['Summary'].values() if isinstance(cell['value'],str)]
                    assert any('settings chosen on Manual grid window' in text and grid['from'] in text and grid['to'] in text for text in manual_text)
                    assert any('fees 3 + slippage 0 bps a side' in text for text in manual_text)
                    (OUT/'analyst-manual-real-runs.json').write_text(json.dumps(manual,indent=2))
                    await page.screenshot(path=str(OUT/'analyst-f1a-manual-expanded.png'),full_page=True)
                    checks.append('Real three-row manual grid keeps its selection window, actual candidate statistics, exact two-neighbor sensitivity and captured-cost GPT workbook')
            assert not errors,errors
            report={'checks':checks,'page_errors':errors,'gpt_workbook':download_name,'performance_source':'Unmodified real Rust optimization and run responses; no market fixtures','browser_count':1,'optimization_concurrency':1}
            (OUT/'analyst-browser-report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
        except Exception:
            (OUT/'analyst-browser-failure.json').write_text(json.dumps({'checks':checks,'page_errors':errors},indent=2))
            await page.screenshot(path=str(OUT/'analyst-browser-failure.png'),full_page=True)
            raise
        finally:
            if original is not None:
                await page.goto('about:blank')
                restore=await context.request.put(url+'/api/app/settings',data=json.dumps(original),headers={'Origin':url,'Content-Type':'application/json'})
                assert restore.status==200,'Original settings restoration failed'
            await context.close();await browser.close()

if __name__=='__main__':asyncio.run(main())
