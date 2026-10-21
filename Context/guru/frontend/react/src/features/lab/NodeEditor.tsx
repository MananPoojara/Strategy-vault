import { Fragment, useState } from "react";
import { COPY } from "./labels";
import { NumField, OperandEditor } from "./OperandEditor";
import { OPERATOR_LABELS, type Side } from "./spec-model";
import { MAX_STEPS, MAX_WHOLE, describeNode, isLeaf, move, newNode, nodeKind, unwrap, withOp, wrap, type ListPath } from "./rule-tree";
import type { Catalogue, Operand, RuleNode } from "./types";
import "./tree.css";

export interface TreeCaps { leaves: boolean; nodes: boolean; top: boolean }
type Obj = Record<string, unknown>;
type AddKind = Parameters<typeof newNode>[0];
const ADD_KINDS = Object.keys(COPY.addOptions) as AddKind[];
const GROUP_TITLES: Record<string, string> = { all: "All of", any: "Any of", not: "Not", at_least: "At least", then: "Sequence" };

export function RuleList({ list, onChange, listPath, title, tag, catalogue, caps }: {
  list: RuleNode[]; onChange(list: RuleNode[]): void; listPath: ListPath; title: string; tag: string; catalogue: Catalogue; caps: TreeCaps;
}) {
  const side: Side = listPath.endsWith("exit.any") ? "exit" : "entry";
  return <div className="lab-list" data-lab-list={listPath}>
    <h3>{title}</h3>
    {list.map((node, i) => <NodeEditor key={i} node={node} onChange={(n) => onChange(list.map((v, j) => j === i ? n : v))}
      onRemove={() => onChange(list.filter((_, j) => j !== i))} onMove={(d) => onChange(move(list, i, d))} canUp={i > 0} canDown={i < list.length - 1}
      catalogue={catalogue} side={side} top caps={caps} label={`${tag} ${i + 1}`} />)}
    <AddMenu title={title} caps={caps} top onAdd={(n) => onChange([...list, n])} />
  </div>;
}

function AddMenu({ title, caps, top = false, sequence = false, full = false, onAdd }: {
  title: string; caps: TreeCaps; top?: boolean; sequence?: boolean; full?: boolean; onAdd(n: RuleNode): void;
}) {
  const blocked = caps.leaves || caps.nodes || (top && caps.top) || full;
  const options = sequence ? ADD_KINDS.filter((k) => k !== "at_least" && k !== "then") : ADD_KINDS;
  return <select data-lab-add aria-label={`Add to ${title}`} title={COPY.addTip} value="" onChange={(e) => {
    const k = e.target.value as AddKind;
    if (!blocked && options.includes(k)) onAdd(newNode(k));
  }}>
    <option value="">{COPY.addMenu}</option>
    {options.map((k) => <option key={k} value={k} disabled={blocked}>{COPY.addOptions[k]}</option>)}
  </select>;
}

export function NodeEditor({ node, onChange, onRemove, onMove, canUp = false, canDown = false, catalogue, side, top, caps, label }: {
  node: RuleNode; onChange(n: RuleNode): void; onRemove?(): void; onMove?(d: -1 | 1): void; canUp?: boolean; canDown?: boolean;
  catalogue: Catalogue; side: Side; top: boolean; caps: TreeCaps; label: string;
}) {
  const [open, setOpen] = useState(true);
  const kind = nodeKind(node), x = node as Obj;
  const set = (patch: Obj) => onChange({ ...x, ...patch } as RuleNode);
  const remove = onRemove && <button type="button" className="dbtn sm ghost" aria-label={`Remove ${label}`} title="Remove this condition" onClick={onRemove}>×</button>;
  const menu = <NodeMenu node={node} onChange={onChange} onMove={onMove} canUp={canUp} canDown={canDown} caps={caps} label={label} />;
  const renderNode = (n: RuleNode, update: (n: RuleNode) => void, name: string) => <NodeEditor node={n} onChange={update} catalogue={catalogue}
    caps={caps} side={side} top={false} label={name} />;
  const operand = (field: string, name: string) => <OperandEditor operand={x[field] as Operand} side={side} catalogue={catalogue}
    onChange={(o) => set({ [field]: o })} label={`${label} ${name}`} heldOk={kind === "cmp" && top && side === "exit"}
    renderNode={renderNode} nodesFull={caps.nodes} />;
  const number = (field: string, name: string) => <NumField label={name} value={x[field] as number} min={1} max={MAX_WHOLE} step={1}
    onChange={(n) => n !== undefined && Number.isInteger(n) && n >= 1 && n <= MAX_WHOLE && set({ [field]: n })} />;
  if (kind === "unknown") return <div className="lab-cond lab-node" data-lab-node data-lab-kind="unknown">
    <span className="lab-kept">{COPY.keptAsIs} <code>{JSON.stringify(node)}</code></span>{remove}
  </div>;
  if (isLeaf(kind)) return <div className="lab-cond lab-node" data-lab-node data-lab-kind={kind}>
    {operand("lhs", "left")}
    <select aria-label="Operator" value={x.op as string} onChange={(e) => onChange(withOp(node, e.target.value))}>
      {[...new Set([...catalogue.operators, x.op as string])].map((op) => <option key={op} value={op}>{OPERATOR_LABELS[op] ?? op}</option>)}
    </select>
    {kind === "cmp" ? operand("rhs", "right") : kind === "range" ? <>{operand("lo", "low")}<span className="lab-word">and</span>{operand("hi", "high")}</>
      : <>{number("bars", "bars")}<span className="lab-word">bars</span></>}
    {menu}{remove}
  </div>;
  const key = kind === "at_least" ? "when" : kind;
  const kids = kind === "not" || kind === "at_least" ? [x[key] as RuleNode] : x[key] as RuleNode[];
  const single = kind === "not" || kind === "at_least";
  const setKids = (next: RuleNode[]) => set({ [key]: single ? next[0] : next });
  const title = GROUP_TITLES[kind];
  return <div className="lab-group lab-node" data-lab-node data-lab-kind={kind}>
    <div className="lab-group-h">
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
          catalogue={catalogue} side={side} top={false} caps={caps}
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
