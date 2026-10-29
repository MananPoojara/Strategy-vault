import { Fragment, useRef, useState, type ReactNode } from "react";
import { COPY } from "./labels";
import { NumField, OperandEditor } from "./OperandEditor";
import { CANDLES, OPERATOR_LABELS, candleCount, candleCountNode, isSeries, isValue, ordinal, rulePhrase, setAt, type CandleKind, type Side } from "./spec-model";
import { EXIT_PRESETS, MAX_STEPS, MAX_WHOLE, describeNode, isLeaf, move, newFilter, newNode, nodeKind, unwrap, withOp, wrap, type ListPath } from "./rule-tree";
import type { Catalogue, Operand, RuleNode } from "./types";
import { DetailContext, useCompact, useSimple } from "./simple";
import { useForeignScope } from "./ForeignFields";
import { ParamRule, plainRule, useParamList } from "./ParamRule";
import { BandRule, bandOf, defaultBand, parseBand, simpleOf } from "./BandRule";
import { defaultRhs, round2, seriesRange } from "./scale-check";
import "./tree.css";

export interface TreeCaps { leaves: boolean; nodes: boolean; top: boolean }
type Obj = Record<string, unknown>;
type AddKind = Parameters<typeof newNode>[0];
const ADD_KINDS = Object.keys(COPY.addOptions) as AddKind[];
const GROUP_TITLES: Record<string, string> = { all: "All of", any: "Any of", not: "Not", at_least: "At least", then: "Sequence" };

export function RuleList({ list, onChange, listPath, title, tag, catalogue, caps, short = listPath.startsWith("short_side."), filters = false, notice, footer }: {
  list: RuleNode[]; onChange(list: RuleNode[]): void; listPath: ListPath; title: string; tag: string; catalogue: Catalogue; caps: TreeCaps;
  /** The list belongs to a short position (ready-made exits are mirrored). */
  short?: boolean;
  /** The Filters tab: entry rules marked `filter` (rules.rs), each with its `filter_exit` box. */
  filters?: boolean;
  /** Shown above the rows: the first-rule suggestions of an empty buy list, or the default exit a sell list falls back to. */
  notice?: ReactNode;
  /** Shown under the rows: the order type of a buy list, the stops and targets of a sell list. */
  footer?: ReactNode;
}) {
  const side: Side = listPath.endsWith("exit.any") ? "exit" : "entry";
  const put = (i: number, n: RuleNode) => onChange(list.map((v, j) => j === i ? n : v));
  const simple = useSimple(), params = useParamList(), [fresh, setFresh] = useState(-1); // Simple: a row added by hand opens in its editor
  return <div className="lab-list" data-lab-list={filters ? `${listPath}.filters` : listPath}>
    <h3>{title}</h3>
    {!simple && !params && <p className="cap lab-list-help">{filters ? COPY.filterListHelp : side === "exit" ? COPY.exitListHelp : COPY.entryListHelp}</p>}
    {notice}
    {list.map((node, i) => {
      // the filter marks stay on the row, outside the editor: a row wrapped in a group must not carry them inside
      const x = node as Obj, bare = { ...x };
      delete bare.filter; delete bare.filter_exit;
      const mark = (n: RuleNode, exit = x.filter_exit === true) => ({ ...n, filter: true, ...(exit ? { filter_exit: true } : {}) }) as RuleNode;
      const remove = () => onChange(list.filter((_, j) => j !== i));
      const editor = <NodeEditor node={filters ? bare as RuleNode : node} onChange={(n) => put(i, filters ? mark(n) : n)}
          onRemove={remove} onMove={(d) => onChange(move(list, i, d))} canUp={i > 0} canDown={i < list.length - 1}
          catalogue={catalogue} side={side} top caps={caps} label={`${tag} ${i + 1}`} />;
      return <Fragment key={i}>
        {simple && !(params && (plainRule(node, catalogue) || nodeKind(node) === "setup")) ? <SentenceRow node={node} catalogue={catalogue} label={`${tag} ${i + 1}`} onChange={(n) => put(i, n)} onRemove={remove} editor={editor} open={i === fresh} /> : editor}
        {filters && <label className="lab-check lab-filter-exit" title="As the AFLs' filter exits (ST Exit, Base exit): the trade closes on any candle where this filter is false">
          <input type="checkbox" aria-label={`${tag} ${i + 1}: ${COPY.filterExit}`} checked={x.filter_exit === true} onChange={(e) => put(i, mark(bare as RuleNode, e.target.checked))} />{COPY.filterExit}</label>}
      </Fragment>;
    })}
    <div className="lab-list-actions">
    <AddMenu title={title} caps={caps} top onAdd={(n) => { setFresh(list.length); onChange([...list, filters ? newFilter() : n]); }} />
    {side === "exit" && <select data-lab-exit-presets aria-label={`Add a ready-made exit to ${title}`} value=""
      title="Exits measured from the entry: the Nth candle of a kind, a trail on an indicator or the close, or a stop / target on the close"
      onChange={(e) => { const p = EXIT_PRESETS.find((x) => x.id === e.target.value); if (p && !(caps.leaves || caps.nodes || caps.top)) onChange([...list, p.node(short)]); }}>
      <option value="">+ Ready-made exit…</option>
      {EXIT_PRESETS.map((p) => <option key={p.id} value={p.id} disabled={caps.leaves || caps.nodes || caps.top}>{p.label(short)}</option>)}
    </select>}
    </div>
    {footer}
  </div>;
}

/** Simple view: a rule as one sentence with its numbers editable in place; Edit opens the full row, × removes it. */
function SentenceRow({ node, catalogue, label, onChange, onRemove, editor, open = false }: {
  node: RuleNode; catalogue: Catalogue; label: string; onChange(n: RuleNode): void; onRemove(): void; editor: ReactNode; open?: boolean;
}) {
  const [editing, setEditing] = useState(open);
  const x = node as Obj, off = x.off === true, phrase = rulePhrase(node, catalogue)?.phrase;
  if (editing) return <div className="lab-sentence-edit" data-lab-sentence-edit>{editor}
    <button type="button" className="dbtn sm primary" onClick={() => setEditing(false)}>Done</button></div>;
  return <div className={`lab-sentence${off ? " lab-off" : ""}`} data-lab-sentence data-lab-off={off || undefined}>
    <span className="lab-sentence-text">{off && <b>Off: </b>}{phrase ? phrase.map((part, i) => typeof part === "string" ? part
      : <NumField key={i} hideLabel label={`${label}: ${part.label}`} value={part.value} min={part.min} max={part.max} step={part.step}
        onChange={(n) => n !== undefined && (!part.integer || Number.isInteger(n)) && onChange(setAt(node, part.path, n))} />)
      : describeNode(node, catalogue)}</span>
    {off && <button type="button" className="dbtn sm" onClick={() => { const on = { ...x }; delete on.off; onChange(on as RuleNode); }}>Switch on</button>}
    <button type="button" className="dbtn sm ghost" aria-label={`Edit ${label}`} onClick={() => setEditing(true)}>Edit</button>
    <button type="button" className="dbtn sm ghost" aria-label={`Remove ${label}`} title="Remove this rule" onClick={onRemove}>×</button>
  </div>;
}

function AddMenu({ title, caps, top = false, sequence = false, full = false, onAdd }: {
  title: string; caps: TreeCaps; top?: boolean; sequence?: boolean; full?: boolean; onAdd(n: RuleNode): void;
}) {
  const blocked = caps.leaves || caps.nodes || (top && caps.top) || full;
  const simple = useSimple(), compact = useCompact();
  if (compact && !simple) return <span className="lab-add-row">
    <button type="button" className="dbtn sm" data-lab-add-cond aria-label={`Add a condition to ${title}`} disabled={blocked} onClick={() => onAdd(newNode("cond"))}>+ Add condition</button>
    <select data-lab-add aria-label={`Add a group to ${title}`} title="A group combines conditions: all must be true, any one is enough, not, a count over the last bars, or a sequence" value="" onChange={(e) => {
      const k = e.target.value as AddKind;
      if (!blocked && k && k !== "cond" && (sequence ? ADD_KINDS.filter((x) => x !== "at_least" && x !== "then") : ADD_KINDS).includes(k)) onAdd(newNode(k));
    }}>
      <option value="">+ Add group…</option>
      {(sequence ? ADD_KINDS.filter((k) => k !== "at_least" && k !== "then") : ADD_KINDS).filter((k) => k !== "cond").map((k) => <option key={k} value={k} disabled={blocked}>{COPY.addOptions[k]}</option>)}
    </select>
  </span>;
  if (simple) return <button type="button" className="dbtn sm" data-lab-add aria-label={`Add to ${title}`} disabled={blocked} onClick={() => onAdd(newNode("cond"))}>+ Add condition</button>;
  const options = sequence ? ADD_KINDS.filter((k) => k !== "at_least" && k !== "then") : ADD_KINDS;
  return <select data-lab-add aria-label={`Add to ${title}`} title={COPY.addTip} value="" onChange={(e) => {
    const k = e.target.value as AddKind;
    if (!blocked && options.includes(k)) onAdd(newNode(k));
  }}>
    <option value="">{COPY.addMenu}</option>
    {options.map((k) => <option key={k} value={k} disabled={blocked}>{COPY.addOptions[k]}</option>)}
  </select>;
}

export function NodeEditor({ node, onChange, onRemove, onMove, canUp = false, canDown = false, catalogue, side, top, caps, label, switchable = top }: {
  node: RuleNode; onChange(n: RuleNode): void; onRemove?(): void; onMove?(d: -1 | 1): void; canUp?: boolean; canDown?: boolean;
  catalogue: Catalogue; side: Side; top: boolean; caps: TreeCaps; label: string;
  /** An on/off switch (rules.rs `off`): top-level rules and the conditions of an all/any group. */
  switchable?: boolean;
}) {
  const [open, setOpen] = useState(true);
  const compact = useCompact(), simple = useSimple(), params = useParamList();
  const [more, setMore] = useState(!compact); // the page keeps a row's timing, source and group/move behind "More"
  const [parts, setParts] = useState(false); // an "Nth up close after entry" exit opened as its full condition
  const kind = nodeKind(node), x = node as Obj;
  const set = (patch: Obj) => onChange({ ...x, ...patch } as RuleNode);
  const scope = useForeignScope(), latest = useRef({ node, onChange });
  latest.current = { node, onChange };
  // picking an indicator on the left compares it with what that indicator is read against (scale-check defaultRhs); another indicator is the user's explicit choice
  const setLhs = (o: Operand) => {
    const before = x.lhs as Operand;
    // a price compared with an oscillator's threshold (RSI's 30) is meaningless: switching to a price compares it with an EMA
    if (kind === "cmp" && isSeries(o) && catalogue.price_sources.includes(o.s) && isValue(x.rhs) && !(isSeries(before) && catalogue.price_sources.includes(before.s)))
      return set({ lhs: o, rhs: { s: "ema", p: { length: 20 }, ...(o.tf ? { tf: o.tf } : {}) } });
    if (kind !== "cmp" || !isSeries(o) || (isSeries(before) && before.s === o.s) || !catalogue.indicators.some((i) => i.id === o.s)) return set({ lhs: o });
    const rhs = defaultRhs(o, catalogue);
    set({ lhs: o, rhs: rhs ?? { value: 0 } });
    if (rhs || !scope?.ticker) return;
    void seriesRange(scope.market, scope.ticker, o).then((r) => {
      const now = latest.current.node as Obj;
      if (r && JSON.stringify(now.lhs) === JSON.stringify(o) && isValue(now.rhs) && now.rhs.value === 0) latest.current.onChange({ ...now, rhs: { value: round2(r.typical) } } as RuleNode);
    }, () => undefined);
  };
  const remove = onRemove && <button type="button" className="dbtn sm ghost" aria-label={`Remove ${label}`} title="Remove this condition" onClick={onRemove}>×</button>;
  const off = x.off === true, offClass = off ? " lab-off" : "";
  const onOff = switchable && <label className="lab-check lab-onoff" title="Switch this condition off without deleting it; the backtest skips it">
    <input type="checkbox" aria-label={`${label} on`} checked={!off} onChange={(e) => set({ off: e.target.checked ? undefined : true })} />On</label>;
  const menu = simple || (isLeaf(kind) && !more) ? null : <NodeMenu node={node} onChange={onChange} onMove={onMove} canUp={canUp} canDown={canDown} caps={caps} label={label} />;
  const renderNode = (n: RuleNode, update: (n: RuleNode) => void, name: string) => <NodeEditor node={n} onChange={update} catalogue={catalogue}
    caps={caps} side={side} top={false} label={name} />;
  const operand = (field: string, name: string) => <OperandEditor operand={x[field] as Operand} side={side} catalogue={catalogue}
    onChange={(o) => field === "lhs" ? setLhs(o) : set({ [field]: o })} label={`${label} ${name}`} heldOk={kind === "cmp" && top && side === "exit"}
    renderNode={renderNode} nodesFull={caps.nodes} />;
  const number = (field: string, name: string) => <NumField label={name} value={x[field] as number} min={1} max={MAX_WHOLE} step={1}
    onChange={(n) => n !== undefined && Number.isInteger(n) && n >= 1 && n <= MAX_WHOLE && set({ [field]: n })} />;
  // a small caption over each part of a top-level row (both views), so a row reads "what to check · condition · compared with"
  const part = (caption: string, el: ReactNode) => top ? <span className="lab-part"><small className="lab-part-cap">{caption}</small><span className="lab-part-body">{el}</span></span> : el;
  // a band setup (close outside a band, then back inside) as one parameter list; "Simple compare" turns it back into a crossing
  if (kind === "setup") return <BandRule node={node} catalogue={catalogue} label={label} exit={side === "exit"} onChange={onChange} onOff={onOff} remove={remove}
    renderNode={renderNode} onSimple={() => { const b = parseBand(node, catalogue); onChange(b ? { ...simpleOf(b), ...(off ? { off: true } : {}) } as RuleNode : x.setup as RuleNode); }} />;
  if (kind === "unknown") return <div className="lab-cond lab-node" data-lab-node data-lab-kind="unknown">
    <span className="lab-kept">{COPY.keptAsIs} <code>{JSON.stringify(node)}</code></span>{remove}
  </div>;
  // the ready-made and default exits read as one sentence; "Edit as condition" shows every part
  const candle = top && side === "exit" && !parts ? candleCount(node) : undefined;
  if (candle) return <div className={`lab-cond lab-node lab-candle${offClass}`} data-lab-node data-lab-kind="candle" data-lab-off={off || undefined}>
    {onOff}
    <span className="lab-word">Exit at the</span>
    <NumField label={`${label}: which one`} hideLabel value={candle.n} min={1} max={MAX_WHOLE} step={1}
      onChange={(n) => n !== undefined && Number.isInteger(n) && n >= 1 && n <= MAX_WHOLE && set({ rhs: { value: n } })} />
    <span className="lab-word lab-ord">{ordinal(candle.n).replace(/^\d+/, "")}</span>
    <select aria-label={`${label}: candle`} value={candle.kind} onChange={(e) => onChange({ ...candleCountNode(e.target.value as CandleKind, candle.n), ...(off ? { off: true } : {}) } as RuleNode)}>
      {(Object.keys(CANDLES) as CandleKind[]).map((k) => <option key={k} value={k}>{CANDLES[k].word}</option>)}
    </select>
    <span className="lab-word">after entry</span>
    {!simple && <button type="button" className="dbtn sm ghost lab-more-btn" title="Show this exit as a full condition you can change part by part" onClick={() => setParts(true)}>Edit as condition</button>}
    {remove}
  </div>;
  const listed = params && top && plainRule(node, catalogue);
  const band = kind === "cmp" ? bandOf(x.lhs as Operand, catalogue) ?? bandOf(x.rhs as Operand, catalogue) : undefined;
  if (listed) return <ParamRule node={node} catalogue={catalogue} label={label} onChange={onChange} onLhs={setLhs} onOff={onOff} remove={remove}
    onBand={band && (() => onChange({ ...defaultBand(band, side === "exit"), ...(off ? { off: true } : {}) } as RuleNode))} />;
  if (isLeaf(kind)) return <div className={`lab-cond lab-node${offClass}`} data-lab-node data-lab-kind={kind} data-lab-off={off || undefined}>
    {onOff}
    <DetailContext.Provider value={more}>
    {part("What to check", operand("lhs", "left"))}
    {part("Condition", <select aria-label="Operator" title="How the left side is compared with the right side" value={x.op as string} onChange={(e) => onChange(withOp(node, e.target.value))}>
      {[...new Set([...catalogue.operators, x.op as string])].map((op) => <option key={op} value={op}>{OPERATOR_LABELS[op] ?? op}</option>)}
    </select>)}
    {kind === "cmp" ? part("Compared with", operand("rhs", "right")) : kind === "range" ? <>{part("Lower limit", operand("lo", "low"))}<span className="lab-word">and</span>{part("Upper limit", operand("hi", "high"))}</>
      : part("For how long", <>{number("bars", "bars")}<span className="lab-word">bars</span></>)}
    </DetailContext.Provider>
    {compact && !simple && <button type="button" className="dbtn sm ghost lab-more-btn" aria-expanded={more} title="Timeframe, bars ago, another symbol, group or move" onClick={() => setMore(!more)}>{more ? "Less ▴" : "More ▾"}</button>}
    {menu}{remove}
  </div>;
  const key = kind === "at_least" ? "when" : kind;
  const kids = kind === "not" || kind === "at_least" ? [x[key] as RuleNode] : x[key] as RuleNode[];
  const single = kind === "not" || kind === "at_least";
  const setKids = (next: RuleNode[]) => set({ [key]: single ? next[0] : next });
  const title = GROUP_TITLES[kind];
  return <div className={`lab-group lab-node${offClass}`} data-lab-node data-lab-kind={kind} data-lab-off={off || undefined}>
    <div className="lab-group-h">
      {onOff}
      <button type="button" className="dbtn sm ghost" aria-expanded={open} onClick={() => setOpen(!open)}>{title}</button>
      {(kind === "all" || kind === "any") && <select aria-label="Group type" value={kind} onChange={(e) => {
        const next = e.target.value;
        if ((next === "all" || next === "any") && next !== kind) {
          const rest = { ...x }; delete rest[kind]; onChange({ ...rest, [next]: kids } as RuleNode);
        }
      }}><option value="all">All of</option><option value="any">Any of</option></select>}
      {kind === "at_least" && <>{number("at_least", "at least")}<span className="lab-word">of the last</span>{number("of", "of (bars)")}<span className="lab-word">bars</span></>}
      {kind === "then" && <><span className="lab-word">within</span>{number("within", "within (bars)")}<span className="lab-word">bars</span></>}
      {menu}{remove}
    </div>
    {open ? <div className="lab-group-body">
      {kids.map((child, i) => <Fragment key={i}>
        {kind === "then" && i > 0 && <span className="lab-word">then</span>}
        <NodeEditor node={child} onChange={(n) => setKids(kids.map((v, j) => j === i ? n : v))}
          onRemove={single ? undefined : () => setKids(kids.filter((_, j) => j !== i))}
          onMove={single ? undefined : (d) => setKids(move(kids, i, d))} canUp={i > 0} canDown={i < kids.length - 1}
          catalogue={catalogue} side={side} top={false} caps={caps} switchable={kind === "all" || kind === "any"}
          label={single ? `${label} › ${key}` : `${label} › ${kind === "then" ? "step" : kind} ${i + 1}`} />
      </Fragment>)}
      {!single && <AddMenu title={label} caps={caps} sequence={kind === "then"} full={kind === "then" && kids.length >= MAX_STEPS}
        onAdd={(n) => setKids([...kids, n])} />}
    </div> : <span className="cap">{describeNode(node, catalogue)}</span>}
  </div>;
}

function NodeMenu({ node, onChange, onMove, canUp, canDown, caps, label }: {
  node: RuleNode; onChange(n: RuleNode): void; onMove?(d: -1 | 1): void; canUp: boolean; canDown: boolean; caps: TreeCaps; label: string;
}) {
  const unwrapped = unwrap(node);
  return <select data-lab-node-menu aria-label={`Actions for ${label}`} title={COPY.nodeMenuTip} value="" onChange={(e) => {
    const action = e.target.value;
    if (!caps.nodes && ["wrap_all", "wrap_any", "wrap_not"].includes(action)) onChange(wrap(node, action.slice(5) as "all" | "any" | "not"));
    else if (action === "unwrap" && unwrapped !== undefined) onChange(unwrapped);
    else if (action === "up" && canUp) onMove?.(-1);
    else if (action === "down" && canDown) onMove?.(1);
  }}>
    <option value="">{COPY.nodeMenuTitle}</option>
    {(["wrap_all", "wrap_any", "wrap_not"] as const).map((k) => <option key={k} value={k} disabled={caps.nodes}>{COPY.nodeMenu[k]}</option>)}
    {unwrapped !== undefined && <option value="unwrap">{COPY.nodeMenu.unwrap}</option>}
    {onMove && <><option value="up" disabled={!canUp}>{COPY.nodeMenu.up}</option><option value="down" disabled={!canDown}>{COPY.nodeMenu.down}</option></>}
  </select>;
}
