#!/usr/bin/env python3
"""Verify the login illustration, accessibility, responsive layout and real session flow."""
import asyncio, json, os
from pathlib import Path
from playwright.async_api import async_playwright, expect
APP=Path(__file__).resolve().parents[1]
ENV=Path(os.environ.get('VAULT_ENV_FILE',Path.home()/'.cache/strategy-vault-webapp/.env'))
config=dict(line.split('=',1) for line in ENV.read_text().splitlines() if line and not line.startswith('#'))
OUT=Path.home()/'.cache/strategy-vault-webapp/browser-verification'
OUT.mkdir(parents=True,exist_ok=True)
URL=os.environ.get('VAULT_TEST_URL',config['VAULT_PUBLIC_URL'])
async def main():
 async with async_playwright() as p:
  browser=await p.chromium.launch(executable_path='/usr/bin/google-chrome',headless=True,args=['--disable-dev-shm-usage'])
  ctx=await browser.new_context(viewport={'width':1440,'height':960})
  if os.environ.get('VAULT_PREVIEW')=='1':
   for name in ['login.html','login.css','login.js']:
    body=(APP/'public'/name).read_text()
    mime={'html':'text/html','css':'text/css','js':'application/javascript'}[name.split('.')[-1]]
    async def serve(route, request, body=body, mime=mime): await route.fulfill(status=200,content_type=mime,body=body)
    await ctx.route('**/'+name+'*',serve)
  page=await ctx.new_page()
  errors=[];external=[]
  page.on('pageerror',lambda e:errors.append(str(e)))
  page.on('request',lambda r:external.append(r.url) if r.url.startswith('http') and not r.url.startswith(URL) else None)
  await page.goto(URL+'/login.html')
  await expect(page.get_by_role('heading',name='Welcome back')).to_be_visible()
  await page.evaluate('document.fonts.ready')
  await page.evaluate('Promise.all(document.getAnimations().map(a=>a.finished))')
  assert await page.evaluate('document.documentElement.scrollWidth<=innerWidth')
  assert not await page.evaluate('document.getAnimations().some(a=>a.playState==="running")')
  await page.screenshot(path=str(OUT/'login-redesign-desktop.png'),full_page=True)
  await page.get_by_role('button',name='Define the rules').click()
  await expect(page.locator('#workflow-insight')).to_contain_text('entry, exit and filters')
  await expect(page.get_by_role('button',name='Define the rules')).to_have_attribute('aria-pressed','true')
  await page.get_by_role('button',name='Study the evidence').click()
  await expect(page.locator('#workflow-insight')).to_contain_text('drawdowns')
  await page.locator('#email').fill(config['VAULT_ADMIN_EMAIL'])
  await page.locator('#password').fill('wrong-password')
  await page.get_by_role('button',name='Show password').click()
  await expect(page.locator('#password')).to_have_attribute('type','text')
  await page.get_by_role('button',name='Hide password').click()
  await expect(page.locator('#password')).to_have_attribute('type','password')
  await page.get_by_role('button',name='Sign in',exact=True).click()
  await expect(page.locator('#message')).to_contain_text('Could not sign in')
  await expect(page.get_by_role('button',name='Sign in',exact=True)).to_be_enabled()
  assert await page.locator('#submit svg').count()==1
  await page.locator('#password').fill(config['VAULT_ADMIN_PASSWORD'])
  await page.locator('#password').press('Enter')
  await page.wait_for_url('**/full-variant1.html**')
  me=await (await ctx.request.get(URL+'/api/app/me')).json()
  assert me['user']['email']==config['VAULT_ADMIN_EMAIL'] and me['user']['role']=='admin'
  await expect(page.locator('.account-tools')).to_be_visible(timeout=30000)
  await page.get_by_role('link',name='Accounts',exact=True).click()
  await expect(page.get_by_role('heading',name='Workspace accounts')).to_be_visible()
  assert await page.locator('body').evaluate('(e)=>!e.classList.contains("login-page")')
  await page.goto(URL+'/full-variant1.html')
  await page.get_by_role('button',name='Sign out',exact=True).click()
  await expect(page.get_by_role('heading',name='Welcome back')).to_be_visible()
  for width,height in [(390,844),(320,740),(768,1024),(1024,768),(1920,1080)]:
   await page.set_viewport_size({'width':width,'height':height})
   await page.evaluate('document.fonts.ready')
   await page.evaluate('Promise.all(document.getAnimations().map(a=>a.finished))')
   assert await page.evaluate('document.documentElement.scrollWidth<=innerWidth'),f'Overflow at {width}'
   await expect(page.locator('#email')).to_be_visible()
   if width==390:
    await page.screenshot(path=str(OUT/'login-redesign-mobile.png'),full_page=True)
    rect=await page.locator('#submit').bounding_box()
    assert rect['y']+rect['height']<height,'Mobile submit below initial viewport'
  await page.emulate_media(reduced_motion='reduce')
  await page.reload()
  await expect(page.get_by_role('heading',name='Welcome back')).to_be_visible()
  assert not await page.evaluate('document.getAnimations().length')
  await page.get_by_role('button',name='Read the source').focus()
  await page.keyboard.press('Tab')
  await page.keyboard.press('Enter')
  await expect(page.get_by_role('button',name='Define the rules')).to_have_attribute('aria-pressed','true')
  assert not errors, errors
  assert not external,external
  result={'checks':['desktop illustration and completed motion','interactive research insights','password show/hide','invalid login and retry','keyboard login with configured administrator credentials','admin screen unchanged','sign-out','responsive widths 320/390/768/1024/1920','mobile submit visible immediately','reduced motion','keyboard workflow controls'], 'page_errors':errors,'external_requests':external,'preview':os.environ.get('VAULT_PREVIEW')=='1'}
  print(json.dumps(result,indent=2))
  (OUT/'login-redesign-report.json').write_text(json.dumps(result,indent=2))
  await browser.close()
asyncio.run(main())
