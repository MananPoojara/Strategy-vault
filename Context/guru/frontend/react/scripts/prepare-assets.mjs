import {cp,mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
const directory=fileURLToPath(new URL('..',import.meta.url)),source=resolve(directory,'..');
await mkdir(resolve(directory,'public'),{recursive:true});
for(const path of ['img','vendor','backtest-trust.js','joe-ross-evidence.js','chande-evidence.js','research-review.js','analyst-conclusion.js','report-data.js','optimization-export.js','library-browser.js','library-discovery.js','optim-core.js'])await cp(resolve(source,path),resolve(directory,'public',path),{recursive:true});
await writeFile(resolve(directory,'public/full-variant1.html'),'<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Strategy Vault</title><script>location.replace("/"+location.search+location.hash)</script></head><body><a href="/">Open Strategy Vault</a></body></html>');
