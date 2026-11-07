import {describe, expect, it} from 'vitest';
import {buildTsWorkbook, maeOf, maeBars} from '../src/compat/ts-workbook.js';
import {zipDeflate} from '../src/compat/optimize-sheet.js';

const bars = {d: ['2020-01-01', '2020-01-02', '2020-01-03', '2020-01-06'], o: [10, 11, 12, 13], h: [11, 12, 13, 14], l: [9, 10, 11, 12], c: [10.5, 11.5, 12.5, 13.5]};
const trades = [
  {dir: 'Long', entry: '2020-01-01', entryPx: 10, exit: '2020-01-03', exitPx: 12},
  {dir: 'Long', entry: '2020-01-03', entryPx: 12, exit: '2020-01-06', exitPx: 11},
];
const rules = {timeframe: 'Daily', index: 'NIFTY 50', period: '01-01-2020 to 06-01-2020', position: 'Long', filter: 'Without filter', setup: '—', entry: 'ADR <= 0.2', exit: 'N candle 16'};

const XLSX0 = () => ( {utils: {book_new: () => ({Sheets: {}, SheetNames: [] as string[]}), book_append_sheet: (b: any, ws: any, n: string) => { b.SheetNames.push(n); b.Sheets[n] = ws; },
    encode_range: (r: any) => `A1:${r.e.c < 26 ? String.fromCharCode(65 + r.e.c) : 'A' + String.fromCharCode(39 + r.e.c)}${r.e.r + 1}`}});
function book() {
  const XLSX = XLSX0();
  return buildTsWorkbook(XLSX, {ticker: 'NIFTY 50', tabName: 'NF_En_ADR thr 0.2_EX_Ncandle-16', trades, bars, rules, costs: {impact: 0.0008, rollLong: 0, rollShort: 0}});
}

describe('Download TS workbook (ADR)', () => {
  it('has the five sheets of the team template, trade sheet last', () => {
    expect(book().SheetNames).toEqual(['All Index Explore', 'Rules', '2008-2011 Summary', 'YOY Summary', 'NF_En_ADR thr 0.2_EX_Ncandle-16']);
  });
  it('writes costs into C1:E1 and the trades from row 21 as live formulas', () => {
    const ws = book().Sheets['NF_En_ADR thr 0.2_EX_Ncandle-16'];
    expect([ws.C1.v, ws.D1.v, ws.E1.v]).toEqual([0.0008, 0, 0]);
    expect(ws.D21.v).toBe(10); expect(ws.F22.v).toBe(11);
    expect(ws.G21.f).toBe('IF(B21="Long",(F21/D21-1),-(F21/D21-1))');
    expect(ws.K22.f).toBe('G22-SUM(I22,J22)');
    expect(ws.L22.f).toBe('L21*(1+G22)');       // the running curve points at the previous trade row
    expect(ws.B3.f).toContain('SUM(G21:G');
    expect(ws.A23).toBeUndefined();            // nothing below the last trade
  });
  it('keeps the whole price history on All Index Explore for the MAE formulas', () => {
    const ix = book().Sheets['All Index Explore'];
    expect(ix.E3.v).toBe(10); expect(ix.A5.v).toBe('NIFTY 50'); expect(ix.A6).toBeUndefined();
  });
});

describe('Download TS workbook (ATH stock basket)', () => {
  // two stocks, each trade with its own bars (entry to exit), as the engine's bars=1 answer gives them
  const own = {
    AAA: [['2020-01-01', 10, 11, 9, 10.5], ['2020-01-02', 11, 12, 8.5, 11.5], ['2020-01-03', 12, 13, 11, 12.5]],
    BBB: [['2020-01-02', 50, 52, 49, 51], ['2020-01-03', 51, 55, 47, 54], ['2020-01-06', 54, 56, 53, 55]],
  } as Record<string, any[]>;
  const tr = [
    {dir: 'Long', entry: '2020-01-01', entryPx: 10, exit: '2020-01-03', exitPx: 12, sym: 'AAA'},
    {dir: 'Long', entry: '2020-01-02', entryPx: 50, exit: '2020-01-06', exitPx: 55, sym: 'BBB'},
  ];
  const bars = {d: [] as string[], o: [] as number[], h: [] as number[], l: [] as number[], c: [] as number[], s: [] as string[]};
  for (const t of tr) for (const [d, o, h, l, c] of own[t.sym]) { bars.d.push(d); bars.o.push(o); bars.h.push(h); bars.l.push(l); bars.c.push(c); bars.s.push(t.sym); }
  const make = (mae?: number[]) => buildTsWorkbook(XLSX0(), {ticker: '', tabName: 'basket', trades: tr, bars, rules, costs: {impact: 0.0008, rollLong: 0, rollShort: 0}, mae});
  it('writes each trade and each bar with its own symbol', () => {
    const b = make(), ws = b.Sheets.basket, ix = b.Sheets['All Index Explore'];
    expect([ws.A21.v, ws.A22.v]).toEqual(['AAA', 'BBB']);
    expect([ix.A2.v, ix.A5.v]).toEqual(['AAA', 'BBB']);
  });
  it('maeOf gives what the sheet\'s MAE formula works out', () => {
    const ws = make().Sheets.basket, mae = tr.map(t => maeOf(t, own[t.sym]));
    expect(mae[0]).toBeCloseTo(8.5 / 10 - 1, 12); expect(mae[1]).toBeCloseTo(47 / 50 - 1, 12);
    expect([ws.W21.v, ws.W22.v].map(x => +x.toFixed(12))).toEqual(mae.map(x => +x.toFixed(12)));
    const pre = make(mae).Sheets.basket;
    expect(pre.W21.f).toBe(ws.W21.f); expect(pre.W21.v).toBe(mae[0]); expect(pre.W21.pre).toBeUndefined();
  });
  it('keeps only the bar that sets each MAE, and the MAE formulas give the same results', () => {
    const own2 = tr.map(t => ({...t, bars: own[t.sym]})), few = maeBars(own2);
    expect(few.s).toEqual(['AAA', 'BBB']); expect(few.l).toEqual([8.5, 47]);
    const ws = buildTsWorkbook(XLSX0(), {ticker: '', tabName: 'basket', trades: tr, bars: few, rules, costs: {impact: 0, rollLong: 0, rollShort: 0}}).Sheets.basket, full = make().Sheets.basket;
    expect([ws.W21.v, ws.W22.v]).toEqual([full.W21.v, full.W22.v]);
  });
});

describe('zipDeflate', () => {
  it('writes a deflated ZIP whose entries unpack to the original bytes', async () => {
    const text = '<x>' + 'row '.repeat(5000) + '</x>', out = await zipDeflate([['a.xml', text], ['b.bin', new Uint8Array([1, 2, 3])]]);
    const dv = new DataView(out.buffer), method = dv.getUint16(8, true), csize = dv.getUint32(18, true), usize = dv.getUint32(22, true), nl = dv.getUint16(26, true);
    expect(method).toBe(8); expect(usize).toBe(text.length); expect(csize).toBeLessThan(usize / 20);
    const body = out.slice(30 + nl, 30 + nl + csize), back = await new Response(new ReadableStream({start(c) { c.enqueue(body); c.close(); }}).pipeThrough(new DecompressionStream('deflate-raw'))).text();
    expect(back).toBe(text);
  });
});
