import asyncio, json
from pathlib import Path
from playwright.async_api import async_playwright, expect
config=dict(line.split('=',1) for line in (Path.home()/'.cache/strategy-vault-webapp/.env').read_text().splitlines() if line and not line.startswith('#'))
async def main():
 async with async_playwright() as p:
  browser=await p.chromium.launch(executable_path='/usr/bin/google-chrome',headless=True,args=['--disable-dev-shm-usage'])
  page=await browser.new_page()
  await page.goto(config['VAULT_PUBLIC_URL']+'/full-variant1.html#/s/rsi2_5d?from=2020-01-01&to=2025-12-31')
  await expect(page.get_by_role('heading',name='Welcome back')).to_be_visible()
  await page.locator('#email').fill(config['VAULT_ADMIN_EMAIL']);await page.locator('#password').fill(config['VAULT_ADMIN_PASSWORD'])
  await page.get_by_role('button',name='Sign in',exact=True).click()
  await page.wait_for_url('**/full-variant1.html**')
  assert '#/s/rsi2_5d?from=2020-01-01&to=2025-12-31' in page.url,page.url
  await expect(page.locator('#csv')).to_be_visible(timeout=30000)
  await page.get_by_role('button',name='Sign out',exact=True).click()
  await expect(page.get_by_role('heading',name='Welcome back')).to_be_visible()
  await browser.close();print('PASS: shared strategy link survives sign-in and opens the requested date range.')
asyncio.run(main())
