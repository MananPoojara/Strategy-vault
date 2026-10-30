import {expect, it} from 'vitest';
import {createPaint, NOT_READY} from '../src/services/first-paint';

function later<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(ok => {resolve = ok;});
  return {promise, resolve};
}

it('paints at once, then repaints with the result, without asking the engine twice', async () => {
  const paint = createPaint(0), cache = {}, engine = later<{a: number}>();
  let calls = 0;
  const ask = async () => paint.ask(cache, 'k', 'sum:k', () => {calls++; return engine.promise;});
  const view = async () => {const value = await ask(); return value === NOT_READY ? 'calculating' : `a=${value.a}`;};
  const first = await paint.run(view);
  expect(first.value).toBe('calculating');
  expect(first.pending.size).toBe(1);
  expect([...first.wanted]).toEqual(['sum:k']);
  const painted: string[] = [first.value];
  const done = paint.until(first.pending, () => false, async () => {
    const next = await paint.run(view);
    painted.push(next.value);
    return next;
  });
  engine.resolve({a: 7});
  await done;
  expect(painted).toEqual(['calculating', 'a=7']);
  expect(calls).toBe(1);
});

it('a count badge never starts a calculation but reads one already here', async () => {
  const paint = createPaint(0), cache: Record<string, Promise<any>> = {};
  let calls = 0;
  const start = () => {calls++; return Promise.resolve({n: 1});};
  const pass = await paint.run(() => paint.peek(async () => paint.ask(cache, 'other', 'sum:other', start)));
  expect(pass.value).toBe(NOT_READY);
  expect(calls).toBe(0);
  expect(pass.pending.size).toBe(0);
  expect(pass.wanted.size).toBe(0);
  await paint.run(async () => paint.ask(cache, 'here', 'sum:here', start));
  await cache.here;
  const read = await paint.run(() => paint.peek(async () => paint.ask(cache, 'here', 'sum:here', start)));
  expect(read.value).toEqual({n: 1});
  expect(calls).toBe(1);
});

it('outside a pass it waits as before, and stops repainting once the page has moved on', async () => {
  const paint = createPaint(0), cache = {};
  expect(await paint.ask(cache, 'k', 'g', async () => 5)).toBe(5);
  const engine = later<number>();
  const pending = new Set<Promise<unknown>>([engine.promise]);
  let repaints = 0;
  const done = paint.until(pending, () => true, async () => {repaints++; return {pending: new Set()};});
  engine.resolve(1);
  await done;
  expect(repaints).toBe(0);
});

it('holds placeholder rows only for the real pass, and a failure still ends the wait', async () => {
  const paint = createPaint(0);
  const pass = await paint.run(async () => {
    await paint.peek(async () => paint.hold(['from a badge']));
    paint.hold(['row']);
    return paint.held();
  });
  expect(pass.value).toEqual(['row']);
  const failing = Promise.reject(new Error('boom'));
  let repaints = 0;
  await paint.until(new Set([failing]), () => false, async () => {repaints++; return {pending: new Set()};});
  expect(repaints).toBe(1);
});
