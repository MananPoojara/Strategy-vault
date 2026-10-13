#!/usr/bin/env python3
"""Real Chromium workflow; credentials stay local and existing preferences/notes are restored."""
import asyncio, json, os, re, zipfile, secrets, subprocess
from pathlib import Path
from playwright.async_api import async_playwright, expect
APP=Path(__file__).resolve().parents[1]
ENV=Path(os.environ.get('VAULT_ENV_FILE',Path.home()/'.cache/strategy-vault-webapp/.env'))
config=dict(line.split('=',1) for line in ENV.read_text().splitlines() if line and not line.startswith('#'))
URL=os.environ.get('VAULT_TEST_URL',config['VAULT_PUBLIC_URL'])
OUT=Path.home()/'.cache/strategy-vault-webapp/browser-verification'
OUT.mkdir(exist_ok=True)
async def settle(page):
 await page.evaluate("Promise.all(document.getAnimations().filter(a=>a.effect.getTiming().iterations!==Infinity).map(a=>a.finished.catch(()=>{})))")
async def main():
 async with async_playwright() as p:
  browser=await p.chromium.launch(executable_path='/usr/bin/google-chrome',headless=True,args=['--disable-dev-shm-usage'])
  context=await browser.new_context(viewport={'width':1440,'height':1000},accept_downloads=True)
  page=await context.new_page()
  errors=[]; failures=[]; external=[]; completed=[]
  page.on('pageerror',lambda error:errors.append(str(error)))
  page.on('response',lambda response:failures.append(f'{response.status} {response.url}') if response.status>=400 and '/api/app/me' not in response.url else None)
  page.on('request',lambda request:external.append(request.url) if not request.url.startswith(URL) and request.url.startswith('http') else None)
  original=None; original_note=None
  viewer_email="viewer."+secrets.token_hex(5)+"@browser.vault.local"
  viewer_password=secrets.token_urlsafe(24);viewer_created=False
  try:
   await page.goto(URL+'/')
   await expect(page.get_by_role('heading',name='Welcome back')).to_be_visible()
   await page.screenshot(path=str(OUT/'login-desktop.png'))
   await page.locator('#email').fill(config['VAULT_ADMIN_EMAIL'])
   await page.locator('#password').fill(config['VAULT_ADMIN_PASSWORD'])
   await page.get_by_role('button',name='Sign in',exact=True).click()
   await page.wait_for_url('**/full-variant1.html**')
   await expect(page.locator('.account-tools')).to_be_visible(timeout=30000)
   original=(await (await context.request.get(URL+'/api/app/me')).json())['settings']
   # Sign-in may restore any saved strategy route. Select the library explicitly.
   await page.goto(URL+'/full-variant1.html#/')
   await expect(page.get_by_role('heading',name="Guru's",exact=True)).to_be_visible(timeout=30000)
   completed.append('Sign in and existing library page')
   await settle(page)
   await page.screenshot(path=str(OUT/'library-desktop.png'))
   await page.get_by_role('link',name='Accounts',exact=True).click()
   await expect(page.get_by_role('heading',name='Workspace accounts')).to_be_visible()
   await page.locator('#name').fill('Browser viewer')
   await page.locator('#email').fill(viewer_email)
   await page.locator('#role').select_option('viewer')
   await page.locator('#password').fill(viewer_password)
   await page.locator('#create').click()
   await expect(page.locator('#message')).to_have_text('Account created.')
   viewer_created=True
   await expect(page.locator('#users')).to_contain_text(viewer_email)
   completed.append('Administrator creates a viewer in account management')
   await page.goto(URL+'/full-variant1.html')
   await expect(page.locator('.account-tools')).to_be_visible(timeout=30000)
   async with page.expect_response(lambda r:'/api/app/settings' in r.url and r.request.method=='PUT') as response:
    await page.locator('#themes button[data-theme="glacier"]').click()
   assert (await response.value).status==200
   await expect(page.locator('html')).to_have_attribute('data-theme','glacier')
   await page.reload()
   await expect(page.locator('html')).to_have_attribute('data-theme','glacier')
   completed.append('Theme saved in database and survives reload')
   await page.goto(URL+'/full-variant1.html#/s/rsi2_5d?from=2020-01-01&to=2025-12-31')
   await expect(page.locator('#yt-text')).to_be_visible(timeout=30000)
   original_note=await (await context.request.get(URL+'/api/app/notes?key=rsi2_5d.in')).text()
   await page.locator('#yt-text').fill('Browser verification: research conclusion saved to PostgreSQL.')
   await page.locator('#yt-save').click()
   await expect(page.locator('#yt-msg')).to_contain_text('Saved at')
   await page.reload()
   await expect(page.locator('#yt-text')).to_have_value('Browser verification: research conclusion saved to PostgreSQL.',timeout=30000)
   completed.append('Strategy chart and conclusion save/reload')
   async with page.expect_download() as downloaded: await page.locator('#csv').click()
   csv=await downloaded.value; await csv.save_as(str(OUT/csv.suggested_filename))
   assert (OUT/csv.suggested_filename).read_text().count('\n')>2
   completed.append('Trades CSV export')
   await page.locator('#open-optimization').click()
   await expect(page.locator('#cc-xlsx')).to_be_visible(timeout=120000)
   async with page.expect_download(timeout=60000) as downloaded: await page.locator('#cc-xlsx').click()
   xlsx=await downloaded.value; await xlsx.save_as(str(OUT/xlsx.suggested_filename))
   with zipfile.ZipFile(OUT/xlsx.suggested_filename) as book: assert 'xl/workbook.xml' in book.namelist()
   completed.append('Separate optimization page, existing automatic conclusion and Excel workbook export')
   await page.locator('#open-optimization').click()
   await expect(page.locator('#csv')).to_be_visible(timeout=30000)
   await settle(page)
   await page.screenshot(path=str(OUT/'strategy-desktop.png'),full_page=True)
   await page.locator('.join.region').get_by_role('button',name='US',exact=True).click()
   await expect(page.locator('.tags2')).to_contain_text('S&P 500',timeout=30000)
   await expect(page.locator('#csv')).to_be_visible()
   assert await page.evaluate('API')=='/api/us'
   completed.append('US market uses authenticated Rust engine')
   await page.set_viewport_size({'width':390,'height':844})
   await settle(page)
   await page.screenshot(path=str(OUT/'strategy-mobile.png'))
   completed.append('Mobile viewport renders')
   viewer_context=await browser.new_context(viewport={'width':1440,'height':1000})
   viewer_page=await viewer_context.new_page()
   await viewer_page.goto(URL+'/login.html')
   await viewer_page.locator('#email').fill(viewer_email)
   await viewer_page.locator('#password').fill(viewer_password)
   await viewer_page.get_by_role('button',name='Sign in',exact=True).click()
   await viewer_page.wait_for_url('**/full-variant1.html**')
   await viewer_page.goto(URL+'/full-variant1.html#/s/rsi2_5d?from=2020-01-01&to=2025-12-31')
   await expect(viewer_page.locator('#yt-save')).to_be_disabled(timeout=30000)
   await expect(viewer_page.locator('#yt-text')).to_have_attribute('readonly','')
   await viewer_page.locator('#open-optimization').click()
   await expect(viewer_page.locator('#orun')).to_be_disabled(timeout=30000)
   await expect(viewer_page.locator('.account-tools')).not_to_contain_text('Accounts')
   completed.append('Viewer can read strategy but cannot edit or optimize')
   await viewer_context.close()
   assert not errors,errors
   assert not external,external
   assert not failures,failures
  except Exception:
   await page.screenshot(path=str(OUT/'failure.png'),full_page=True)
   raise
  finally:
   if original_note is not None: await context.request.post(URL+'/api/app/notes?key=rsi2_5d.in',data=original_note,headers={'Origin':URL,'Content-Type':'text/plain'})
   if original is not None: await context.request.put(URL+'/api/app/settings',data=json.dumps(original),headers={'Origin':URL,'Content-Type':'application/json'})
   await page.set_viewport_size({'width':1440,'height':1000})
   await page.get_by_role('button',name='Sign out',exact=True).click()
   await expect(page.get_by_role('heading',name='Welcome back')).to_be_visible()
   completed.append('Sign out revokes browser session')
   await context.close();await browser.close()
   if viewer_created:
    sql="DELETE FROM vault_audit WHERE entity IN (SELECT id FROM \"user\" WHERE email='"+viewer_email+"'); DELETE FROM \"user\" WHERE email='"+viewer_email+"';"
    subprocess.run(['docker','compose','--project-directory',str(APP),'--env-file',str(ENV),'exec','-T','postgres','psql','-U','vault','-d','vault','-v','ON_ERROR_STOP=1','-c',sql],check=True,capture_output=True)
   result={'completed':completed,'page_errors':errors,'failed_responses':failures,'external_requests':external}
   (OUT/'report.json').write_text(json.dumps(result,indent=2))
   print(json.dumps(result,indent=2))
asyncio.run(main())
