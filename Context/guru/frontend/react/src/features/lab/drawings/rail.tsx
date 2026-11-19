import { Fragment, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { GROUPS, TOOLS } from "./registry";

export type Cursor = "cross" | "dot" | "arrow" | "demo" | "eraser";
type Hide = "drawings" | "indicators" | "positions";
export type DrawPrefs = { cursor: Cursor; magnet: "weak" | "strong" | null; stay: boolean; lock: boolean; hide: Record<Hide, boolean>;
  /** Each fly-out's last choice: a group's tool, the magnet mode, the hide option. */
  last: Record<string, string> };
const PREFS_KEY = "lab.drawPrefs";
const DEFAULTS: DrawPrefs = { cursor: "cross", magnet: null, stay: false, lock: false, hide: { drawings: false, indicators: false, positions: false }, last: {} };

/** The rail's choices, kept in this browser. */
export function useDrawPrefs() {
  const [prefs, set] = useState<DrawPrefs>(() => { try { return { ...DEFAULTS, ...JSON.parse(localStorage.getItem(PREFS_KEY) ?? "{}") }; } catch { return DEFAULTS; } });
  const update = (change: Partial<DrawPrefs>) => set((old) => {
    const next = { ...old, ...change };
    try { localStorage.setItem(PREFS_KEY, JSON.stringify(next)); } catch { /* kept for this visit */ }
    return next;
  });
  return [prefs, update] as const;
}

const CURSORS: [Cursor, string, ReactNode][] = [
  ["cross", "Cross", <path d="M12 3v18M3 12h18" />],
  ["dot", "Dot", <circle cx="12" cy="12" r="3" fill="currentColor" />],
  ["arrow", "Arrow", <path d="M7 3v15l4-4 3 7 2.5-1-3-7h5.5z" />],
  ["demo", "Demonstration", <><circle cx="12" cy="12" r="8" /><circle cx="12" cy="12" r="2.5" fill="currentColor" /></>],
  ["eraser", "Eraser", <path d="m14 4 6 6-9 9H6l-3-3zM9 9l6 6M11 19h10" />],
];
const HIDES: [Hide | "all", string][] = [["drawings", "Hide drawings"], ["indicators", "Hide indicators"], ["positions", "Hide positions & orders"], ["all", "Hide all"]];
const ICON = {
  measure: <path d="M3 17 17 3l4 4L7 21zM7 13l2 2M10 10l2 2M13 7l2 2" />,
  zoom: <><circle cx="10" cy="10" r="6.5" /><path d="m15 15 6 6M7 10h6M10 7v6" /></>,
  zoomOut: <><circle cx="10" cy="10" r="6.5" /><path d="m15 15 6 6M7 10h6" /></>,
  magnet: <path d="M5 4h4v8a3 3 0 0 0 6 0V4h4v8a7 7 0 0 1-14 0zM5 8h4M15 8h4" />,
  stay: <><path d="m14 5 5 5-10 10H4v-5z" /><path d="M17 21a3 3 0 1 0 0-6" /></>,
  lock: <><rect x="5" y="11" width="14" height="10" rx="1.5" /><path d="M8 11V8a4 4 0 0 1 8 0v3" /></>,
  eye: <><path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6S2 12 2 12z" /><circle cx="12" cy="12" r="3" /></>,
  eyeOff: <><path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6S2 12 2 12z" /><path d="M4 4l16 16" /></>,
  trash: <path d="M4 6h16M9 6V3h6v3M6 6l1 15h10l1-15M10 10v7m4-7v7" />,
  undo: <path d="m8 5-5 5 5 5M3 10h11a6 6 0 0 1 0 12" />,
};
type Item = { key: string; name: string; icon?: ReactNode; section?: string; shortcut?: string; checked?: boolean; attrs?: Record<string, string>; pick(): void };
const plural = (n: number, one: string) => `${n} ${one}${n === 1 ? "" : "s"}`;
const typing = (t: EventTarget | null) => t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement || t instanceof HTMLSelectElement;
/** "Alt+Shift+R" against a key press (the key's code: Alt changes the typed letter on a Mac). */
const matches = (shortcut: string, e: KeyboardEvent) => {
  const parts = shortcut.split("+"), key = parts[parts.length - 1];
  return e.altKey === parts.includes("Alt") && e.shiftKey === parts.includes("Shift") && (e.ctrlKey || e.metaKey) === parts.includes("Ctrl")
    && (e.code === `Key${key}` || e.key.toUpperCase() === key);
};

/** TradingView's left toolbar: cursors, the seven drawing groups (each shows its last tool, its arrow or a long press opens the
 *  fly-out), Measure, Zoom in, Magnet, Stay in drawing mode, Lock, Hide and Remove, then Undo. */
export function DrawRail({ hidden, tool, setTool, prefs, setPrefs, drawings, indicators, zoomed, onZoomOut, onUndo, onRemoveDrawings, onRemoveIndicators }: {
  hidden: boolean; tool: string | null; setTool(tool: string | null): void; prefs: DrawPrefs; setPrefs(change: Partial<DrawPrefs>): void;
  drawings: number; indicators: number; zoomed: boolean; onZoomOut(): void; onUndo(): void; onRemoveDrawings(): void; onRemoveIndicators(): void;
}) {
  const [menu, setMenu] = useState<{ id: string; x: number; y: number } | null>(null);
  const held = useRef<{ timer?: number; opened?: boolean }>({});
  const last = (id: string, fallback: string) => prefs.last[id] ?? fallback;
  const choose = (group: string, id: string) => { setPrefs({ last: { ...prefs.last, [group]: id } }); setTool(id); };
  // fly-outs close on Esc, a click outside, or a choice; TradingView's shortcuts pick their tool
  useEffect(() => {
    const away = (e: PointerEvent) => { if (menu && !(e.target instanceof Element && e.target.closest(`[data-lab-slot="${menu.id}"]`))) setMenu(null); };
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") { setMenu(null); return; }
      if (hidden || typing(e.target) || !(e.altKey || e.ctrlKey || e.metaKey)) return;
      const group = GROUPS.find((g) => g.tools.some((t) => t.shortcut && matches(t.shortcut, e)));
      const def = group?.tools.find((t) => t.shortcut && matches(t.shortcut, e));
      if (group && def) { e.preventDefault(); choose(group.id, def.id); }
    };
    window.addEventListener("pointerdown", away); window.addEventListener("keydown", key);
    return () => { window.removeEventListener("pointerdown", away); window.removeEventListener("keydown", key); };
  });
  const open = (id: string, el: Element) => {
    if (menu?.id === id) { setMenu(null); return; }
    const r = el.closest("[data-lab-slot]")!.getBoundingClientRect(), rail = el.closest(".lab-draw-rail")?.getBoundingClientRect() ?? r;
    setMenu({ id, x: rail.right + 6, y: r.top });
  };
  // TradingView keeps the whole menu on screen: one taller than the space below the button opens higher up
  useLayoutEffect(() => {
    const fly = menu && document.querySelector<HTMLElement>(`.lab-flyout[data-menu="${menu.id}"]`);
    if (!fly) return;
    const top = Math.max(8, Math.min(menu.y, window.innerHeight - fly.offsetHeight - 8));
    if (top !== menu.y) setMenu({ ...menu, y: top });
  }, [menu]);
  const button = (label: string, icon: ReactNode, onClick: () => void, attrs: Record<string, unknown> = {}) =>
    <button type="button" className="dbtn sm" title={label} aria-label={label} onClick={onClick} {...attrs}><svg viewBox="0 0 24 24" aria-hidden="true">{icon}</svg></button>;
  /** A rail button with a fly-out: `main` runs on a click, a long press or the arrow opens the menu. */
  const slot = (id: string, label: string, icon: ReactNode, main: (() => void) | null, pressed: boolean, items: Item[], attrs: Record<string, unknown> = {}) => {
    let section: string | undefined;
    return <div key={id} className="lab-draw-slot" data-lab-slot={id}>
      {button(label, icon, () => { if (held.current.opened) held.current.opened = false; else if (main) main(); else open(id, document.querySelector(`[data-lab-slot="${id}"]`)!); }, {
        "aria-pressed": pressed, "data-lab-draw-group": id, ...attrs,
        onPointerDown: (e: { currentTarget: Element }) => { const el = e.currentTarget; held.current.timer = window.setTimeout(() => { held.current.opened = true; open(id, el); }, 450); },
        onPointerUp: () => clearTimeout(held.current.timer), onPointerLeave: () => clearTimeout(held.current.timer),
      })}
      {main && <button type="button" className="lab-draw-more" aria-label={`More ${label.toLowerCase()} options`} aria-haspopup="menu" aria-expanded={menu?.id === id}
        onClick={(e) => open(id, e.currentTarget)}><svg viewBox="0 0 6 10" aria-hidden="true"><path d="m1 1 4 4-4 4" /></svg></button>}
      <div className="lab-flyout" data-menu={id} role="menu" aria-label={label} hidden={menu?.id !== id} style={menu?.id === id ? { left: menu.x, top: menu.y } : undefined}>
        {items.length ? items.map((it) => {
          const head = it.section && it.section !== section ? <div className="lab-flyout-head" role="presentation">{(section = it.section)}</div> : null;
          return <Fragment key={it.key}>{head}
            <button type="button" role={it.checked === undefined ? "menuitem" : "menuitemcheckbox"} aria-checked={it.checked} {...it.attrs}
              onClick={() => { it.pick(); setMenu(null); }}>
              {it.icon && <svg viewBox="0 0 24 24" aria-hidden="true">{it.icon}</svg>}<span>{it.name}</span>{it.shortcut && <kbd>{it.shortcut}</kbd>}
            </button></Fragment>;
        }) : <p className="lab-flyout-head">Nothing to remove</p>}
      </div>
    </div>;
  };
  const cursor = CURSORS.find(([c]) => c === prefs.cursor) ?? CURSORS[0];
  const magnetMode = last("magnet", "weak") as "weak" | "strong", hideMode = last("hide", "drawings") as Hide | "all";
  const anyHidden = Object.values(prefs.hide).some(Boolean);
  const toggleHide = (h: Hide | "all") => {
    const all = Object.values(prefs.hide).every(Boolean);
    setPrefs({ hide: h === "all" ? { drawings: !all, indicators: !all, positions: !all } : { ...prefs.hide, [h]: !prefs.hide[h] }, last: { ...prefs.last, hide: h } });
  };
  return <div className="lab-draw lab-draw-rail" role="toolbar" aria-label="Drawing tools" hidden={hidden}>
    {slot("cursors", cursor[1], cursor[2], () => setTool(null), tool === null, CURSORS.map(([id, name, icon]) => ({ key: id, name, icon, checked: prefs.cursor === id,
      attrs: { "data-lab-cursor": id }, pick: () => { setPrefs({ cursor: id }); setTool(null); } })))}
    {GROUPS.filter((g) => g.tools.length).map((g) => {
      const shown = TOOLS[last(g.id, g.tools[0].id)] ?? g.tools[0];
      return slot(g.id, shown.name, shown.icon, () => (tool === shown.id ? setTool(null) : choose(g.id, shown.id)), g.tools.some((t) => t.id === tool),
        g.tools.map((t) => ({ key: t.id, name: t.name, icon: t.icon, section: t.section, shortcut: t.shortcut, attrs: { "data-lab-draw": t.id }, pick: () => choose(g.id, t.id) })));
    })}
    <hr />
    {button("Measure", ICON.measure, () => setTool(tool === "measure" ? null : "measure"), { "aria-pressed": tool === "measure", "data-lab-draw-measure": "" })}
    {button("Zoom in", ICON.zoom, () => setTool(tool === "zoom" ? null : "zoom"), { "aria-pressed": tool === "zoom", "data-lab-draw-zoom": "" })}
    {zoomed && button("Zoom out", ICON.zoomOut, onZoomOut, { "data-lab-draw-zoomout": "" })}
    <hr />
    {slot("magnet", magnetMode === "strong" ? "Strong magnet" : "Weak magnet", ICON.magnet, () => setPrefs({ magnet: prefs.magnet ? null : magnetMode }), !!prefs.magnet,
      (["weak", "strong"] as const).map((m) => ({ key: m, name: m === "weak" ? "Weak magnet" : "Strong magnet", icon: ICON.magnet, checked: prefs.magnet === m,
        attrs: { "data-lab-magnet": m }, pick: () => setPrefs({ magnet: prefs.magnet === m ? null : m, last: { ...prefs.last, magnet: m } }) })))}
    {button("Stay in drawing mode", ICON.stay, () => setPrefs({ stay: !prefs.stay }), { "aria-pressed": prefs.stay, "data-lab-draw-stay": "" })}
    {button(prefs.lock ? "Unlock all drawings" : "Lock all drawings", ICON.lock, () => setPrefs({ lock: !prefs.lock }), { "aria-pressed": prefs.lock, "data-lab-draw-lock": "" })}
    {slot("hide", HIDES.find(([h]) => h === hideMode)![1], anyHidden ? ICON.eyeOff : ICON.eye, () => toggleHide(hideMode), anyHidden,
      HIDES.map(([h, name]) => ({ key: h, name, checked: h === "all" ? Object.values(prefs.hide).every(Boolean) : prefs.hide[h], attrs: { "data-lab-hide": h }, pick: () => toggleHide(h) })))}
    <hr />
    {slot("remove", "Remove objects", ICON.trash, null, false, [
      ...(drawings ? [{ key: "d", name: `Remove ${plural(drawings, "drawing")}`, attrs: { "data-lab-draw-clear": "" }, pick: onRemoveDrawings }] : []),
      ...(indicators ? [{ key: "i", name: `Remove ${plural(indicators, "indicator")}`, attrs: { "data-lab-remove-indicators": "" }, pick: onRemoveIndicators }] : []),
      ...(drawings && indicators ? [{ key: "a", name: `Remove ${plural(drawings, "drawing")} & ${plural(indicators, "indicator")}`, attrs: { "data-lab-remove-all": "" },
        pick: () => { onRemoveDrawings(); onRemoveIndicators(); } }] : []),
    ])}
    {drawings > 0 && button("Undo drawing", ICON.undo, onUndo, { className: "dbtn sm ghost", "data-lab-draw-undo": "" })}
  </div>;
}
