import {existsSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
let names=['analyst-conclusion','author-order','backtest-trust','book-context','book-library','conclusion-presentation','contracts','exporthandlers','initial-equity','instrument-routing','legacy-response','library-backtest-context','library-context-integration','library-browser','library-discovery','library-market','optimization-export','report-data','research-library','research-review','summary-initialization','summary-loader'];
const capture=process.env.VAULT_OPT_FIXTURE||'Context/guru/webapp/verification/joe-ross/optimization-3-row-input.json';
if(!existsSync(capture)){names=names.filter(name=>!['analyst-conclusion','conclusion-presentation'].includes(name));process.stdout.write('Captured-ledger checks require private external VAULT_OPT_FIXTURE; other offline checks continue.\n');}
const result=spawnSync(process.execPath,['--test','--test-reporter=tap',...names.map(name=>`Context/guru/webapp/tests/${name}.test.mjs`)],{stdio:'inherit'});
process.exit(result.status??1);
