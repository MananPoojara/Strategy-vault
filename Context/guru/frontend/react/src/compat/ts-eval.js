// Calculates the formulas of the Download TS workbook so the file also stores their results (the team's own file does), as Excel would:
// it understands exactly the functions the template uses. Excel still recalculates on open (fullCalcOnLoad).

const ERR = {NA: {err: '#N/A', code: 42}, VALUE: {err: '#VALUE!', code: 15}, DIV0: {err: '#DIV/0!', code: 7}, REF: {err: '#REF!', code: 23}, NUM: {err: '#NUM!', code: 36}};
const isErr = v => v !== null && typeof v === 'object' && !Array.isArray(v) && 'err' in v;
const colN = c => c.split('').reduce((n, ch) => n * 26 + ch.charCodeAt(0) - 64, 0);
const colL = n => { let s = ''; for (; n > 0; n = Math.floor((n - 1) / 26)) s = String.fromCharCode(65 + ((n - 1) % 26)) + s; return s; };

const TOKEN = /\s*(?:(\d+\.?\d*(?:[eE][+-]?\d+)?)|("(?:[^"]|"")*")|((?:'(?:[^']|'')+'|[A-Za-z_][A-Za-z0-9_.]*)!)?(\$?[A-Z]{1,3}\$?\d+)(?![A-Za-z0-9(])|([A-Za-z_][A-Za-z0-9_.]*)(?=\()|(<=|>=|<>|[-+*/^&=<>(),:]))/y;
function tokenize(src) {
  const out = []; TOKEN.lastIndex = 0;
  while (TOKEN.lastIndex < src.length) {
    const at = TOKEN.lastIndex, m = TOKEN.exec(src);
    if (!m) { if (/^\s*$/.test(src.slice(at))) break; throw new Error('formula: cannot read ' + src.slice(at, at + 20)); }
    if (m[1] !== undefined) out.push({t: 'num', v: parseFloat(m[1])});
    else if (m[2] !== undefined) out.push({t: 'str', v: m[2].slice(1, -1).replace(/""/g, '"')});
    else if (m[4] !== undefined) out.push({t: 'ref', sheet: m[3] ? m[3].slice(0, -1).replace(/^'|'$/g, '').replace(/''/g, "'") : null, a: m[4].replace(/\$/g, '')});
    else if (m[5] !== undefined) out.push({t: 'fn', v: m[5].toUpperCase()});
    else out.push({t: 'op', v: m[6]});
  }
  return out;
}

// precedence: comparison < & < + - < * / < ^ < unary minus
function parse(src) {
  const tk = tokenize(src); let p = 0;
  const peek = () => tk[p], isOp = v => tk[p] && tk[p].t === 'op' && tk[p].v === v;
  const bin = (next, ops) => () => { let l = next(); while (tk[p] && tk[p].t === 'op' && ops.includes(tk[p].v)) { const o = tk[p++].v; l = {k: 'bin', o, l, r: next()}; } return l; };
  const primary = () => {
    const t = tk[p++];
    if (!t) throw new Error('formula ends early');
    if (t.t === 'num') return {k: 'lit', v: t.v};
    if (t.t === 'str') return {k: 'lit', v: t.v};
    if (t.t === 'ref') {
      if (isOp(':')) { p++; const b = tk[p++]; return {k: 'range', sheet: t.sheet, a: t.a, b: b.a}; }
      return {k: 'cell', sheet: t.sheet, a: t.a};
    }
    if (t.t === 'fn') { p++; const args = []; if (!isOp(')')) { for (;;) { args.push(expr()); if (isOp(',')) { p++; continue; } break; } } p++; return {k: 'fn', n: t.v, args}; }
    if (t.t === 'op' && t.v === '(') { const e = expr(); p++; return e; }
    throw new Error('formula: unexpected ' + t.v);
  };
  const unary = () => { if (isOp('-')) { p++; return {k: 'neg', e: unary()}; } if (isOp('+')) { p++; return unary(); } return primary(); };
  const pow = bin(unary, ['^']), mul = bin(pow, ['*', '/']), add = bin(mul, ['+', '-']), cat = bin(add, ['&']), expr = bin(cat, ['=', '<>', '<', '>', '<=', '>=']);
  const e = expr(); return e;
}

export function evaluateWorkbook(book) {
  const memo = new Map(), busy = new Set(), asts = new Map(), rowStack = [];
  const extent = ws => { const m = /^[A-Z]+(\d+):([A-Z]+)(\d+)$/.exec(ws['!ref'] || ''); return m ? {rows: +m[3], cols: colN(m[2])} : {rows: 0, cols: 0}; };
  const ext = new Map(book.SheetNames.map(n => [n, extent(book.Sheets[n])]));

  function cellValue(sheet, a) {
    const key = sheet + '!' + a;
    if (memo.has(key)) return memo.get(key);
    const ws = book.Sheets[sheet]; if (!ws) return ERR.REF;
    const c = ws[a]; let v;
    if (!c || c.t === 'z') v = null;
    else if (c.pre !== undefined) v = c.pre;                // a result the builder already worked out (a basket's MAE)
    else if (c.f !== undefined) {
      if (busy.has(key)) return ERR.REF;
      busy.add(key);
      let ast = asts.get(key); if (!ast) { ast = parse(c.f); asts.set(key, ast); }
      rowStack.push(+/\d+/.exec(a)[0]);
      try { v = evalNode(ast, sheet); } catch (e) { v = ERR.VALUE; }
      rowStack.pop();
      busy.delete(key);
      if (Array.isArray(v)) v = v[0] ?? null;
    } else v = c.t === 'e' ? ERR.NA : c.v === undefined ? null : c.v;
    memo.set(key, v); return v;
  }
  function rangeValues(sheet, a, b) {
    const ma = /^([A-Z]+)(\d+)$/.exec(a), mb = /^([A-Z]+)(\d+)$/.exec(b), e = ext.get(sheet) || {rows: 0};
    const r1 = +ma[2], r2 = Math.min(+mb[2], e.rows), c1 = colN(ma[1]), c2 = colN(mb[1]);
    if (c1 !== c2) { const row = []; for (let c = c1; c <= c2; c++) row.push(cellValue(sheet, colL(c) + r1)); return row; }
    const col = []; for (let r = r1; r <= r2; r++) col.push(cellValue(sheet, ma[1] + r));
    return col;
  }
  const num = v => (v === null || v === undefined ? 0 : typeof v === 'boolean' ? +v : typeof v === 'number' ? v : (typeof v === 'string' && v !== '' && !isNaN(+v) ? +v : ERR.VALUE));
  const lift = (x, y, f) => {
    if (Array.isArray(x) || Array.isArray(y)) { const n = Array.isArray(x) ? x.length : y.length; return Array.from({length: n}, (_, i) => lift(Array.isArray(x) ? x[i] : x, Array.isArray(y) ? y[i] : y, f)); }
    if (isErr(x)) return x; if (isErr(y)) return y; return f(x, y);
  };
  const cmpv = v => (v === null ? 0 : v);
  const compare = (o, x, y) => {
    x = cmpv(x); y = cmpv(y);
    const rank = v => (typeof v === 'string' ? 2 : typeof v === 'boolean' ? 3 : 1);
    let c; if (rank(x) !== rank(y)) c = rank(x) - rank(y); else if (typeof x === 'string') { x = x.toLowerCase(); y = y.toLowerCase(); c = x < y ? -1 : x > y ? 1 : 0; } else c = x - y;
    return o === '=' ? c === 0 : o === '<>' ? c !== 0 : o === '<' ? c < 0 : o === '>' ? c > 0 : o === '<=' ? c <= 0 : c >= 0;
  };
  const arith = (o, x, y) => {
    if (o === '&') return String(x ?? '') + String(y ?? '');
    if (['=', '<>', '<', '>', '<=', '>='].includes(o)) return compare(o, x, y);
    const a = num(x), b = num(y); if (isErr(a)) return a; if (isErr(b)) return b;
    if (o === '+') return a + b; if (o === '-') return a - b; if (o === '*') return a * b;
    if (o === '/') return b === 0 ? ERR.DIV0 : a / b;
    const r = Math.pow(a, b); return Number.isFinite(r) ? r : ERR.NUM;
  };
  const flat = args => args.flatMap(a => (Array.isArray(a) ? a : [a]));
  const numbers = args => { const out = []; for (const a of args) { if (Array.isArray(a)) { for (const v of a) { if (isErr(v)) return v; if (typeof v === 'number') out.push(v); } } else { if (isErr(a)) return a; const n = num(a); if (isErr(n)) return n; if (a !== null) out.push(n); } } return out; };
  const crit = c => {
    if (typeof c === 'number') return v => typeof v === 'number' && v === c;
    const m = /^(<=|>=|<>|<|>|=)?(.*)$/.exec(String(c)), o = m[1] || '=', t = m[2], n = parseFloat(t);
    if (!isNaN(n) && String(n) === t.trim()) return v => typeof v === 'number' && compare(o, v, n);
    return v => typeof v === 'string' && compare(o, v, t);
  };
  const toSerialYear = s => new Date(Date.UTC(1899, 11, 30) + s * 86400000).getUTCFullYear();

  function evalNode(n, sheet) {
    switch (n.k) {
      case 'lit': return n.v;
      case 'cell': return cellValue(n.sheet || sheet, n.a);
      case 'range': return rangeValues(n.sheet || sheet, n.a, n.b);
      case 'neg': { const v = evalNode(n.e, sheet); return Array.isArray(v) ? v.map(x => (isErr(x) ? x : -num(x))) : isErr(v) ? v : -num(v); }
      case 'bin': return lift(evalNode(n.l, sheet), evalNode(n.r, sheet), (x, y) => arith(n.o, x, y));
      case 'fn': return callFn(n, sheet);
      default: return ERR.VALUE;
    }
  }
  function callFn(n, sheet) {
    const name = n.n, A = n.args;
    if (name === 'IF') {
      const c = evalNode(A[0], sheet), t = () => evalNode(A[1], sheet), f = () => (A[2] ? evalNode(A[2], sheet) : false);
      if (Array.isArray(c)) { const tv = t(), fv = f(); return c.map((x, i) => (isErr(x) ? x : (x === true || (typeof x === 'number' && x !== 0)) ? (Array.isArray(tv) ? tv[i] : tv) : (Array.isArray(fv) ? fv[i] : fv))); }
      if (isErr(c)) return c;
      return (c === true || (typeof c === 'number' && c !== 0)) ? t() : f();
    }
    if (name === 'IFERROR') { const v = evalNode(A[0], sheet); return isErr(v) ? evalNode(A[1], sheet) : v; }
    if (name === 'NA') return ERR.NA;
    if (name === 'ROW') return rowStack[rowStack.length - 1];
    // a cell reference that holds text is skipped by SUM/MAX/..., a typed-in text argument is an error
    const a = A.map(x => { const v = evalNode(x, sheet); return x.k === 'cell' && typeof v === 'string' && ['SUM', 'AVERAGE', 'COUNT', 'MAX', 'MIN'].includes(name) ? [v] : v; });
    switch (name) {
      case 'SUM': { const v = numbers(a); return isErr(v) ? v : v.reduce((s, x) => s + x, 0); }
      case 'AVERAGE': { const v = numbers(a); return isErr(v) ? v : v.length ? v.reduce((s, x) => s + x, 0) / v.length : ERR.DIV0; }
      case 'COUNT': { const v = numbers(a); return isErr(v) ? v : v.length; }
      case 'MAX': { const v = numbers(a); return isErr(v) ? v : v.length ? Math.max(...v) : 0; }
      case 'MIN': { const v = numbers(a); return isErr(v) ? v : v.length ? Math.min(...v) : 0; }
      case 'ABS': { const x = num(a[0]); return isErr(x) ? x : Math.abs(x); }
      case 'YEAR': { const x = num(a[0]); return isErr(x) ? x : toSerialYear(x); }
      case 'COUNTIF': { const f = crit(a[1]); return a[0].filter(f).length; }
      case 'SUMIF': case 'AVERAGEIF': {
        const f = crit(a[1]), rng = a[0], sr = a[2] || a[0]; let s = 0, k = 0;
        rng.forEach((v, i) => { if (f(v)) { const x = sr[i]; if (isErr(x)) return; if (typeof x === 'number') { s += x; k++; } } });
        return name === 'SUMIF' ? s : k ? s / k : ERR.DIV0;
      }
      case 'MATCH': {
        const v = a[0], rng = a[1], type = a[2] === undefined ? 1 : a[2];
        if (type === 0) { const i = rng.findIndex(x => x !== null && !isErr(x) && compare('=', x, v)); return i < 0 ? ERR.NA : i + 1; }
        let best = -1; rng.forEach((x, i) => { if (typeof x === 'number' && typeof v === 'number' && (type > 0 ? x <= v : x >= v)) best = i; });
        return best < 0 ? ERR.NA : best + 1;
      }
      case 'INDEX': { const i = a[1]; if (isErr(i)) return i; const r = a[0][i - 1]; return r === undefined ? ERR.REF : r; }
      case 'INDIRECT': {
        const m = /^(?:'((?:[^']|'')+)'|([^!]+))!\$?([A-Za-z]+\$?\d+)$/.exec(String(a[0])); if (!m) return ERR.REF;
        return cellValue((m[1] || m[2]).replace(/''/g, "'"), m[3].replace(/\$/g, '').toUpperCase());
      }
      default: return ERR.VALUE;
    }
  }

  let done = 0;
  for (const name of book.SheetNames) {
    const ws = book.Sheets[name];
    for (const addr of Object.keys(ws)) {
      if (addr[0] === '!') continue;
      const c = ws[addr]; if (!c || c.f === undefined) continue;
      const v = cellValue(name, addr); done++; delete c.pre;
      if (isErr(v)) { c.t = 'e'; c.v = v.code; }
      else if (typeof v === 'number') { c.t = 'n'; c.v = v; }
      else if (typeof v === 'boolean') { c.t = 'b'; c.v = v; }
      else { c.t = 's'; c.v = v === null ? '' : String(v); }
    }
  }
  return done;
}
