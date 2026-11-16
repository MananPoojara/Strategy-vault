# Drawing tools

Each TradingView tool group has one file here, and each file exports an array of `ToolDef`s:
`lines.tsx` LINES, `fib.tsx` FIB, `patterns.tsx` PATTERNS, `forecast.tsx` FORECAST, `shapes.tsx` SHAPES, `text.tsx` TEXT.
`registry.ts` collects them into `TOOLS` (by id) and `GROUPS` (rail order). The rail (`rail.tsx`) and the layer
(`../DrawingLayer.tsx`) need no change when you add a tool: they read the registry.
Only edit your own group file and your own test file.

## ToolDef (model.ts)

| field | meaning |
|---|---|
| `id` | unique tool id, also the stored `kind` (the layer looks the renderer up by `kind`) |
| `name` | TradingView's name, e.g. "Parallel channel" |
| `section` | the fly-out header it is listed under, e.g. "CHANNELS" (keep TradingView's order in the array) |
| `icon` | the contents of a 24×24 SVG, stroked (`fill:none; stroke:currentColor` is set by the rail) |
| `points` | clicks to place it (`1` = one click); `"free"` = the pointer path while pressed (brush); `"multi"` = click points, double-click or Enter finishes |
| `make?(pts, ctx)` | the stored drawing, default `{ kind: id, pts }`; `ctx` is the draft's RenderCtx (layer size, x/y). Also called with the points so far while drawing. Only Lines/Fib/Forecast legacy tools return the old `{a,b}` shapes; new tools keep the default |
| `render(d, ctx)` | the SVG, inside `<g data-lab-drawing={kind}>`; return `null` to draw nothing. `d.pts` may hold fewer points than `points` while it is being drawn |
| `text?` | opens the inline text editor after placing; a double-click edits `d.text` |
| `shortcut?` | TradingView's shortcut, e.g. `"Alt+V"`, `"Alt+Shift+R"`; it works as soon as it is set |
| `hint?` | the hint under the chart while placing (a default follows from `points`) |

Extra settings go in `d.props` (numbers, strings, booleans). Points are `[bar date, price]`, snapped to bars (and by the magnet).

## RenderCtx

- `x(date)`, `xAt(barIndex)` (works past the last bar), `y(price)`: pixels, or `null` when off the data; `index(date)`: bar index (−1 if unknown).
- `bars`: `{ d, o, h, l, c, v }` by bar index (`v` may be empty or hold nulls).
- `plot` (width up to the price axis), `axis` (price-axis width), `height`, `color` (default blue), `live` (selected / moving / being drawn), `i` (index, `null` for the draft).
- `hit(x1,y1,x2,y2)`, `hitArea(x,y,w,h)`, `hitPath(d, fill?)`: invisible targets that make the drawing selectable and movable (nothing for the draft).
- `handles(xy?)`: point handles `data-part="p0".."pN"`, at `d.pts` by default; pass pixel positions `[x,y] | null` per point to put them elsewhere. Dragging handle k moves `pts[k]`.
- `label(lines, x, y, fill, below = true)`: TradingView's read-out box (white text), centred on `x`, kept in the plot, its top at `y` (or its bottom when `below` is false).
- `fmt(price)` "25,281.85", `priceChange(a, b)` "120.50 (0.48%)", `dateSpan(bars, from, to)` "25 bars, 35d".

Use TradingView's default colours (e.g. `#2962FF` blue, `#F23645` red, `#089981` green, `#787B86` grey) and `ctx.color` for the plain line colour.

## Example: Vertical line (lines.tsx)

```tsx
{ id: "vline", name: "Vertical line", section: "LINES", points: 1, shortcut: "Alt+V",
  render: (d, c) => {
    const x = c.x(d.pts[0][0]);
    if (x === null) return null;
    return <>
      <line x1={x} x2={x} y1={0} y2={c.height} stroke={c.color} strokeWidth={c.live ? 2 : 1.5} />
      {c.hit(x, 0, x, c.height)}
      {c.handles([[x, c.height / 2]])}
    </>;
  },
  icon: <><path d="M12 2v20" /><circle cx="12" cy="12" r="2" /></> },
```

## Test pattern

See `tests/lab-drawing-tools.test.tsx` (copy its `layer()` helper): a mock chart with one bar every 10 px and price `p` at `y = p / 10`,
`SVGElement.clientWidth` patched to 500 (plot 450 with a 50 px axis). Mount the layer with `tool: "<your id>"`, `click(x, y)` each point
(or `ptr("pointerdown"| "pointermove" | "pointerup" | "dblclick", x, y)`), then check the saved drawing (`last()`) and the rendered SVG
(`[data-lab-drawing="<id>"]`). Put each group's tests in its own file, e.g. `tests/lab-draw-lines.test.tsx`.
Run `npx tsc --noEmit -p .` and `npx vitest run <your test file>`.
