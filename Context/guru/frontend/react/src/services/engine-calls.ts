/** Expensive engine calls of the open page: at most `limit` run at once, the rest wait their turn,
 * and a call the page no longer wants is aborted or dropped before it starts. Groups name a result
 * (one summary may be several owner calls), so the next page can keep a calculation it also needs. */
interface Call {group: string; controller: AbortController; start(): void; drop(): void}
export function engineCalls(limit = 2) {
  const queued: Call[] = [], running = new Set<Call>();
  const pump = () => {
    while (running.size < limit && queued.length) {
      const call = queued.shift()!;
      running.add(call);
      call.start();
    }
  };
  return {
    run<T>(group: string, task: (signal: AbortSignal) => Promise<T>): Promise<T> {
      return new Promise<T>((resolve, reject) => {
        const controller = new AbortController();
        queued.push({group, controller,
          start() {
            const done = () => {running.delete(this); pump();};
            task(controller.signal).then(value => {done(); resolve(value);}, error => {done(); reject(error);});
          },
          drop() {reject(new DOMException('The page no longer needs this result.', 'AbortError'));},
        });
        pump();
      });
    },
    keepOnly(wanted: ReadonlySet<string>) {
      for (let i = queued.length - 1; i >= 0; i--) if (!wanted.has(queued[i].group)) queued.splice(i, 1)[0].drop();
      for (const call of running) if (!wanted.has(call.group)) call.controller.abort();
    },
  };
}
