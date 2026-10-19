import {Element, htmlToDOM, type DOMNode} from "html-react-parser";

export const REPORT_TABS = [
  {id: "equity", label: "Equity Curve", anchor: "c-eq"},
  {id: "drawdown", label: "Drawdown", anchor: "c-dd"},
  {id: "monthly", label: "Monthly Returns", anchor: "heatbox"},
  {id: "yearly", label: "Yearly Returns", anchor: "c-yr"},
  {id: "signals", label: "Price & Signals", anchor: "c-px"},
  {id: "positions", label: "Positions", anchor: null},
] as const;
export type ReportTabId = typeof REPORT_TABS[number]["id"];
export interface ReportMarkup {nodes: DOMNode[]; tabs: ReportTabId[]}

// Parser bundles can use separate domhandler constructors; identify their shared
// node format rather than depending on cross-bundle instanceof identity.
export function isMarkupElement(node: {type: string} | null): node is Element {
  return !!node && ["tag", "script", "style"].includes(node.type);
}

function find(nodes: DOMNode[], predicate: (node: Element) => boolean): Element | undefined {
  for (const node of nodes) if (isMarkupElement(node)) {
    if (predicate(node)) return node;
    const child = find(node.children as DOMNode[], predicate);
    if (child) return child;
  }
}
function section(node: Element | undefined): Element | undefined {
  for (let current = node; current; current = isMarkupElement(current.parent) ? current.parent : undefined)
    if (current.name === "section") return current;
}
function children(node: Element, nodes: DOMNode[]) {
  node.children = nodes as typeof node.children;
  for (const child of node.children) child.parent = node;
}

/** Regroup the existing presenters; prices, results and widget containers are untouched. */
export function prepareStrategyReport(html: string): ReportMarkup {
  const nodes = htmlToDOM(html);
  const panels = REPORT_TABS.flatMap(tab => {
    const node = section(find(nodes, element => tab.anchor
      ? element.attribs.id === tab.anchor
      : Object.hasOwn(element.attribs, "data-tradesheet")));
    return node ? [{id: tab.id, node}] : [];
  });
  const equity = panels.find(panel => panel.id === "equity")?.node;
  if (!equity) return {nodes, tabs: []};
  const wrappers = new Set<Element>();
  const detach = (node: Element) => {
    const siblings = nodes.includes(node) ? nodes : node.parent?.children;
    const index = siblings?.indexOf(node) ?? -1;
    if (index >= 0) siblings!.splice(index, 1);
  };
  const content = panels.map(({id, node}) => {
    if (node !== equity) {
      if (isMarkupElement(node.parent) && node.parent.attribs.class?.split(/\s+/).includes("two"))
        wrappers.add(node.parent);
      detach(node);
    }
    const panel = new Element("div", {
      ...node.attribs,
      class: `report-panel ${(node.attribs.class || "").replace(/\bbox\b/g, "").trim()}`.trim(),
      id: `report-panel-${id}`, role: "tabpanel", tabindex: "0",
      "data-report-panel": id, "aria-labelledby": `report-tab-${id}`,
    }, []);
    children(panel, [...node.children] as DOMNode[]);
    return panel;
  });
  for (const wrapper of wrappers) if (!wrapper.children.some(node => isMarkupElement(node)
    || (node.type === "text" && node.data.trim()))) detach(wrapper);
  equity.attribs.class = `${equity.attribs.class || "box"} strategy-report`;
  equity.attribs["data-strategy-report"] = "";
  children(equity, [new Element("div", {"data-react-slot": "report-tabs"}, []), ...content]);
  return {nodes, tabs: panels.map(panel => panel.id)};
}
