// "Export everything" for verifiers: the five page exports of one strategy over the verification matrix
// (3 India indices x 2 periods x book settings / optimiser #1), one engine run at a time, collected for one zip.
// The workbooks come from the page's own export code (injected as `workbook`), so each file equals its single export.

export const EXPORT_INDICES = [['NIFTY 50', 'Nifty 50'], ['NIFTYBANK', 'Bank Nifty'], ['NIFTYMIDCAP100', 'Midcap 100']] as const;
export const EXPORT_PERIODS = [['full-history', 'Full history', ''], ['from-2010-01-04', 'From 2010-01-04', '2010-01-04']] as const;
export const EXPORT_KINDS = ['trades', 'equity', 'monthly', 'benchmark', 'details'] as const;
export const MANIFEST_HEADERS = ['File', 'Index', 'Data key', 'Period', 'From (requested)', 'Settings', 'Parameters', 'Export', 'Run from', 'Run to', 'Engine ms', 'Run at', 'Status', 'Note'];

export type ExportKind = typeof EXPORT_KINDS[number];
export type Skip = {skip: string};
// A run as the page reads it: `params` = the full settings used, `fixed` = optimiser not admitted for this run (reason).
export type ExportRun = {dates: string[]; params: Record<string, number>; ms?: number; fixed?: string};
export type ExportDeps<R extends ExportRun> = {
  admit: (tk: string) => string | null;
  ready: (tk: string, from: string, p: string) => Promise<string | null>;
  run: (tk: string, from: string, p: string, settings: string) => Promise<R>;
  optimiserTop: (tk: string, from: string) => Promise<string | Skip>;
  workbook: (kind: ExportKind, run: R) => Promise<{name: string; bytes: Uint8Array} | Skip>;
  progress?: (done: number, total: number, text: string) => void;
  signal?: AbortSignal;
  now?: () => string;
};
export type ExportFile = {path: string; bytes: Uint8Array};

const paramText = (p: Record<string, number>) => Object.entries(p).map(([k, v]) => `${k}=${v}`).join('; ');
const isSkip = (x: unknown): x is Skip => typeof x === 'object' && x !== null && 'skip' in x;

export async function exportEverything<R extends ExportRun>(deps: ExportDeps<R>): Promise<{files: ExportFile[]; manifest: unknown[][]}> {
  const files: ExportFile[] = [], manifest: unknown[][] = [MANIFEST_HEADERS], now = deps.now || (() => new Date().toISOString());
  const total = EXPORT_INDICES.length * EXPORT_PERIODS.length * 2;
  let done = 0;
  const step = (text: string) => { deps.signal?.throwIfAborted(); deps.progress?.(done, total, text); };
  const skipRows = (index: string, tk: string, period: string, from: string, settings: string, note: string) => {
    manifest.push(['', index, tk, period, from, settings, '', '', '', '', '', now(), 'skipped', note]);
  };
  for (const [tk, index] of EXPORT_INDICES) for (const [periodKey, period, from] of EXPORT_PERIODS) {
    const not = deps.admit(tk);
    let hold: string | undefined; // why the optimiser row is not run: book data not ready, or a fixed reviewed configuration
    for (const [settingsKey, settings] of [['book', 'Book settings'], ['optimiser-1', 'Optimiser #1']] as const) {
      step(`${index} · ${period} · ${settings}`);
      try {
        if (not) { skipRows(index, tk, period, from, settings, not); continue; }
        let p = '';
        if (settingsKey === 'optimiser-1') {
          if (hold) { skipRows(index, tk, period, from, settings, hold); continue; }
          const top = await deps.optimiserTop(tk, from);
          if (isSkip(top)) { skipRows(index, tk, period, from, settings, top.skip); continue; }
          p = top;
        }
        step(`${index} · ${period} · ${settings}: checking data`);
        const why = await deps.ready(tk, from, p);
        if (why) { if (settingsKey === 'book') hold = why; skipRows(index, tk, period, from, settings, why); continue; }
        step(`${index} · ${period} · ${settings}: running`);
        const run = await deps.run(tk, from, p, settingsKey), at = now();
        if (settingsKey === 'book') hold = run.fixed;
        for (const kind of EXPORT_KINDS) {
          step(`${index} · ${period} · ${settings}: ${kind}`);
          const book = await deps.workbook(kind, run), row = [index, tk, period, from, settings, paramText(run.params), kind, run.dates[0], run.dates.at(-1), run.ms ?? '', at];
          if (isSkip(book)) { manifest.push(['', ...row, 'skipped', book.skip]); continue; }
          const path = `${tk.replace(/\s+/g, '')}/${periodKey}/${settingsKey}/${book.name}`;
          files.push({path, bytes: book.bytes}); manifest.push([path, ...row, 'ok', '']);
        }
      } catch (error) {
        if (deps.signal?.aborted) throw error;
        skipRows(index, tk, period, from, settings, 'Failed: ' + (error instanceof Error ? error.message : String(error)));
      } finally { done++; }
    }
  }
  deps.progress?.(done, total, 'Done');
  return {files, manifest};
}
