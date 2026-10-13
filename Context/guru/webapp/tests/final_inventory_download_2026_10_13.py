import asyncio,csv,json,hashlib,io
from pathlib import Path
from playwright.async_api import async_playwright,expect
from accuracy_browser_2026_10_13 import CFG,URL,ready

async def main():
 async with async_playwright() as p:
  browser=await p.chromium.launch(executable_path='/usr/bin/google-chrome',headless=True,args=['--disable-dev-shm-usage'])
  context=await browser.new_context(viewport={'width':390,'height':844},accept_downloads=True);page=await context.new_page();errors=[]
  page.on('pageerror',lambda e:errors.append(str(e)))
  try:
   await page.goto(URL+'/login.html');await page.locator('#email').fill(CFG['VAULT_ADMIN_EMAIL']);await page.locator('#password').fill(CFG['VAULT_ADMIN_PASSWORD']);await page.get_by_role('button',name='Sign in',exact=True).click();await page.wait_for_url('**/full-variant1.html**')
   await page.goto(URL+'/full-variant1.html#/');await expect(page.get_by_role('link',name='Download book inventory',exact=True)).to_be_visible(timeout=90000)
   await ready(page,"!document.querySelector('#flipov') && libraryRecords().length>30000")
   assert await page.evaluate('document.documentElement.scrollWidth<=window.innerWidth+1')
   async with page.expect_download() as dl:await page.get_by_role('link',name='Download book inventory',exact=True).click()
   out=Path.home()/'.cache/strategy-vault-webapp/research-share-browser-2026-10-13/final-book-inventory.xlsx';await (await dl.value).save_as(str(out))
   source=Path('deliverables/Research-Share-Book-Conversion-Inventory-2026-10-13.xlsx')
   assert out.read_bytes()==source.read_bytes()
   response=await context.request.get(URL+'/data/book-coverage-by-author.csv');assert response.status==200
   rows=list(csv.DictReader(io.StringIO((await response.body()).decode('utf-8-sig'))))
   expected=await page.evaluate('({groups:LIB.length,entries:libraryRecords().length})')
   assert len(rows)==expected['groups']
   assert sum(int(r['library_entries']) for r in rows)==expected['entries']
   tharp=next(r for r in rows if r['group_id']=='tharp');assert int(tharp['library_entries'])==19 and int(tharp['catalogue_entries'])==3
   assert not errors,errors
   await page.screenshot(path=str(out.parent/'final-mobile-home.png'))
   report={'final_excel_matches_deliverable':True,'sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'bytes':out.stat().st_size,'csv_groups':len(rows),'csv_entries':expected['entries'],'mobile_fits':True,'page_errors':errors}
   (out.parent/'final-download-report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
  finally:
   try:
    r=await context.request.post(URL+'/api/auth/sign-out',data='{}',headers={'Origin':URL,'Content-Type':'application/json'});assert r.status==200
   finally:await context.close();await browser.close()

asyncio.run(main())
