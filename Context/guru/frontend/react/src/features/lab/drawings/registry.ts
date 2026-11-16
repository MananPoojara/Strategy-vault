import type { ToolDef } from "./model";
import { LINES } from "./lines";
import { FIB } from "./fib";
import { PATTERNS } from "./patterns";
import { FORECAST } from "./forecast";
import { SHAPES } from "./shapes";
import { TEXT } from "./text";

/** TradingView's drawing groups in its left-toolbar order (the cursors come first, built by the rail). */
export const GROUPS: { id: string; name: string; tools: ToolDef[] }[] = [
  { id: "lines", name: "Lines", tools: LINES },
  { id: "fib", name: "Gann and Fibonacci", tools: FIB },
  { id: "patterns", name: "Patterns", tools: PATTERNS },
  { id: "forecast", name: "Forecasting and measurement", tools: FORECAST },
  { id: "shapes", name: "Geometric shapes", tools: SHAPES },
  { id: "text", name: "Annotations", tools: TEXT },
];
export const TOOLS: Record<string, ToolDef> = Object.fromEntries(GROUPS.flatMap((g) => g.tools.map((t) => [t.id, t])));
