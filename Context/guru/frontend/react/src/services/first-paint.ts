/** Draw a page from what is already here, then again as each backtest arrives. Inside a pass, a calculation
 * that has not answered yet resolves at once as NOT_READY, so no page ever waits on the engine for its first paint.
 * Views must show NOT_READY as "still calculating", never as an empty or zero result. */
export const NOT_READY: Readonly<Record<string, never>> = Object.freeze({});
type Cache = Record<string, Promise<any> | undefined>;
interface Pass {pending: Set<Promise<unknown>>; wanted: Set<string>; held: unknown[]}

export function createPaint(settleMs = 250) {
  let pass: Pass | null = null, peeking = false;
  const arrived = new WeakSet<Promise<unknown>>();
  const track = <T,>(promise: Promise<T>) => {
    const mark = () => {arrived.add(promise);};
    promise.then(mark, mark);
    return promise;
  };
  return {
    /** The cached or newly started calculation. `group` names it to the engine-call limiter. */
    ask(cache: Cache, key: string, group: string, start: () => Promise<any>): any {
      if (peeking && !cache[key]) return NOT_READY;
      const promise = cache[key] || (cache[key] = track(start()));
      if (!pass) return promise;
      if (!peeking) pass.wanted.add(group);
      if (arrived.has(promise)) return promise;
      if (!peeking) pass.pending.add(promise);
      return NOT_READY;
    },
    /** Inside `view`, use only results already here: a count badge never starts a calculation. */
    async peek<T>(view: () => Promise<T>): Promise<T> {
      peeking = true;
      try {return await view();} finally {peeking = false;}
    },
    async run<T>(view: () => Promise<T>) {
      const current: Pass = pass = {pending: new Set(), wanted: new Set(), held: []};
      try {
        const value = await view();
        return {value, pending: current.pending, wanted: current.wanted};
      } finally {if (pass === current) pass = null;}
    },
    waiting: () => !!pass && pass.pending.size > 0,
    /** Rows to list as "still calculating" while their result is on its way. */
    hold(rows: unknown[]) {if (pass && !peeking) pass.held.push(...rows);},
    held: () => pass?.held || [],
    /** Re-run the view whenever something arrives, until nothing is pending or the page has moved on. */
    async until(first: Set<Promise<unknown>>, stale: () => boolean, again: () => Promise<{pending: Set<Promise<unknown>>}>) {
      let pending = first;
      while (pending.size) {
        await Promise.race([...pending].map(promise => promise.catch(() => undefined)));
        await new Promise(resolve => setTimeout(resolve, settleMs)); // several answers landing together make one redraw
        if (stale()) return;
        ({pending} = await again());
        if (stale()) return;
      }
    },
  };
}
