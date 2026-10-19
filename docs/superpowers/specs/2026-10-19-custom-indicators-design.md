# Custom indicators (Indicators lab) — design

**Why:** users want their own indicators like AmiBroker AFL / TradeStation "Custom 1–4 Lines" / MetaStock Indicator Builder: type a formula, name it, tune its parameters, chart it, use it in strategy rules and optimize it, and keep it in their own library.

**Goals**
- A small AmiBroker-flavoured formula language, evaluated by the Rust lab engine bar by bar on daily (and resampled W/M/Q/Y) bars.
- Saved per user ("My indicators" group at the top of the Library), editable, deletable.
- A custom indicator behaves like a built-in: chip on the chart (own pane or on price), Parameters window from its `Param()` calls, usable as an operand in Strategy rules, its params optimizable, works in walk-forward, Scan/Explore and Monte Carlo through the normal run.
- Clear errors with line/column.

**Non-goals:** arbitrary code, loops, variables that change bar to bar by assignment (AFL loops), intraday data, sharing/publishing custom indicators between users (later), importing real AFL/Pine files verbatim.

**Anti-goals (would make it a failure even if it works):** any look-ahead (a value at bar i may use only bars ≤ i); a formula that can hang or exhaust the engine; saved strategies that silently change when someone edits a custom indicator (specs carry the formula text, so a saved strategy keeps the formula it was saved with); book strategy results changing.

## Contract

### 1. Formula language (engine `src/lab/formula.rs`)
- Statements end with `;` or a newline. `//` line comments. Identifiers are case-insensitive.
- `name = expr;` defines a series (single assignment; later use refers to it; no reassignment, no recursion).
- `Param("Label", default, min, max, step)` — a tunable number; its key is the label (e.g. `Fast`). Max 16 params.
- `Plot(expr, "Name")` — one output named `Name`. 1–8 plots. Optional 3rd arg `"price"` draws the whole indicator on the price chart (any plot asking for it); otherwise it gets its own pane. Line style/colour are chosen in the Parameters window like built-ins.
- Literals: numbers, `true`, `false`. Price arrays: `O H L C V`, `Open High Low Close Volume`, `Avg`=(H+L+C)/3, `hl2`, `hlc3`, `ohlc4`.
- Operators: `+ - * / ^`, unary `-`, `> < >= <= == !=`, `AND OR NOT` (also `&& || !`). Booleans are 1/0 series.
- Lookback: `x[n]` (n bars ago, n a constant 0–2000) and `Ref(x, -n)`; positive Ref (future) is refused.
- Array functions: `MA/SMA(x,n) EMA(x,n) WMA(x,n) Wilders/RMA(x,n) StDev(x,n) Sum(x,n) HHV/Highest(x,n) LLV/Lowest(x,n) ROC(x,n) Change/Mom(x,n) LinRegSlope(x,n) AMA(x,alpha) Abs Sqrt Log Exp Max(a,b) Min(a,b) IIf(c,a,b) Cross(a,b) BarsSince(c) ValueWhen(c,x[,nth]) Nz(x[,v])`. `n` must be a constant or Param (whole number ≥1).
- Any computed catalogue indicator by id (or its AmiBroker/TradingView alias where unambiguous): positional args map to the catalogue parameter order, named args `rsi(length=14)`; `.output` selects an output (default first): `Supertrend(10,3).direction`. Its parameters must be constants/Params. Indicators that need data the run lacks fail with the catalogue's own message.
- NaN propagates (warm-up); comparisons with NaN are false.
- Limits: ≤ 4000 characters, ≤ 300 expression nodes, ≤ 8 plots, ≤ 16 params. Errors: `line L, col C: message`.

### 2. Engine API
- **Spec operand** (anywhere an indicator operand is allowed in rules, portfolio score, scan/explore):
  `{"formula": "<text>", "name": "My Momentum", "p": {"Fast": 9}, "out": "Osc"}` plus the usual operand timing fields (`tf`, `offset`, `expand`) with the same semantics as an indicator operand. `p` overrides Param defaults (validated against min/max, whole numbers when step is whole). `out` defaults to the first Plot. Labels read `My Momentum(Fast=9).Osc` (params at default may be omitted like LATE_PARAMS).
- **Series:** `GET /api/lab/series?formula=<text>&name=<n>&<param>=<v>…&tf=…&ticker=…` returns the same shape as an indicator series (`outputs` = plot names, `series`, `dates`) plus `"formula": {"params":[{key,default,min,max,step}], "plots":["Osc","Signal"], "placement":"pane"|"price"}`.
- **Validation:** `GET /api/lab/formula?formula=<text>` → `{ok:true, params, plots, placement}` or 400 `{error:"line 2, col 5: unknown function FOO"}`. Gateway allowlist: add `lab/formula` additively (cheap, read-only); spec-size checks apply to formula operands inside specs (formula text counts toward the spec byte budget).
- **Catalogue:** `/api/lab/catalogue` adds `formula_functions: [{name, args, desc}]` for the editor's help list.
- Optimization: `p.<Param>` numeric leaves inside a formula operand are optimizable like indicator params.

### 3. Storage (auth service, same pattern as lab strategies)
- Table `lab_indicator(id serial, owner → user, name text ≤ 60, formula text ≤ 4000, created_at, updated_at, version int)`, unique (owner, lower(name)). Migration `webapp/db/004-lab-indicators.sql`, loaded in bootstrap.mjs like 002/003.
- Routes (owner only; same auth/role and same-origin checks as lab strategies): `GET /api/app/lab/indicators`, `POST /api/app/lab/indicators {name, formula}`, `PUT /api/app/lab/indicators/:id {name, formula, version}`, `DELETE /api/app/lab/indicators/:id?version=`. Response row `{id, name, formula, version, createdAt, updatedAt}`. Server does NOT parse the formula (engine is the authority); it only enforces lengths.
- Gateway already proxies `/api/app/*` to auth — no gateway change.

### 4. Frontend
- Library: a "My indicators" group first (collapsed like the rest, count shown) listing the user's saved indicators + a "+ New indicator" button; clicking a name adds it to the chart like a built-in; an "Edit" action opens the editor.
- Formula editor (a box like the indicator detail card, full width under the chart): Name, formula textarea (monospace, tab inserts spaces), "Check" (calls `/api/lab/formula`, shows params/plots or the error with line/col highlighted), Save / Save as new / Delete / Close, and a collapsible function reference from `formula_functions` plus 3 examples (e.g. MACD-style oscillator, RSI of an EMA, Bollinger %B with Param).
- Chart chip for a custom indicator uses `/api/lab/series?formula=…`; the Parameters window shows its `Param()`s.
- Strategy rules operand picker: "My indicators" section; choosing one inserts the formula operand (formula text copied into the spec) with output select and its params.
- Optimize paths list the formula params (`My Momentum › Fast`).

### Constraints
- Book code/results untouched; protected files untouched; main.rs and gateway policy.rs additive-only; no new dependencies.
- Tests: engine parser/evaluator unit tests (each function, look-ahead refusal, limits, errors with positions, equivalence `EMA(C,9)` == catalogue ema 9), API route tests; auth route tests like lab-strategies tests; frontend vitest for library group, editor check/save, chip, operand picker, optimize path.

### Left to the builders
Exact evaluator structure, caching of parsed formulas, CSS details, wording of help text.

## 5. Drag-and-drop block builder (user request, 2026-10-19)
The user wants custom indicators buildable **completely by drag and drop**, with no typing required (typing stays available in the Formula tab).
- **Editor tabs:** "Blocks" (default for new indicators) and "Formula". Blocks always generates the formula text (the engine only ever sees formula text); the block layout is saved alongside in `blocks` (jsonb, nullable) so it can be edited later. A formula typed by hand opens in the Formula tab; "Convert to blocks" is offered when the formula uses only block-expressible constructs (all of them, since blocks cover the whole language).
- **Palette (left, grouped, searchable):** Price (Open/High/Low/Close/Volume/hl2/hlc3/ohlc4), Number, Parameter (label/default/min/max/step), My series (named results), Indicators (all computed catalogue indicators by category, with their params and output choice), Math (+ − × ÷ ^, abs, sqrt, log, exp, max, min, negate), Smoothing & windows (MA, EMA, WMA, Wilders, StDev, Sum, Highest, Lowest, ROC, Change, LinReg slope, AMA), Lookback (n bars ago), Compare (> < ≥ ≤ = ≠, crosses above/below), Logic (and/or/not, if-then-else), State (bars since, value when, nz).
- **Canvas:** one row per Plot ("Line 1 … Line 8": name + on price/own pane) and optional "Name this result" rows (assignments) above them. Each block shows its slots inline (e.g. `EMA( [series] , [length] )`); drop a block onto a slot to fill it; drag a placed block to move it; drop on the palette/bin to delete. Slots are typed (series / whole number / condition) and refuse wrong drops with a visible reason. Numbers are editable inline; a Parameter block makes a value tunable (it appears in the Parameters window and is optimizable).
- **Interaction:** pointer-event drag (works with mouse, touch and pen — native HTML5 DnD alone fails on touch), keyboard alternative (select a slot, then pick a block from the palette list; Delete/Backspace removes), undo/redo, live formula preview and live Check (debounced) with errors pointing at the offending block.
- **No new dependencies.** Light/dark themes, 400px layout (palette collapses into a drawer), accessible labels.
