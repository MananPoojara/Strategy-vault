import {expect, it} from 'vitest';
import {engineCalls} from '../src/services/engine-calls';

const flush = () => new Promise(resolve => setTimeout(resolve));
function held() {
  const open: {group: string; signal: AbortSignal; finish(): void}[] = [];
  const task = (group: string) => (signal: AbortSignal) => new Promise<string>((resolve, reject) => {
    open.push({group, signal, finish: () => resolve(group)});
    signal.addEventListener('abort', () => reject(new DOMException('left', 'AbortError')));
  });
  return {open, task};
}

it('never runs more than the limit at once and starts the next call as a slot frees', async () => {
  const calls = engineCalls(2), {open, task} = held();
  const results = ['a', 'b', 'c'].map(group => calls.run(group, task(group)));
  expect(open.map(call => call.group)).toEqual(['a', 'b']);
  open[0].finish();
  await flush();
  expect(open.map(call => call.group)).toEqual(['a', 'b', 'c']);
  open[1].finish(); open[2].finish();
  expect(await Promise.all(results)).toEqual(['a', 'b', 'c']);
});

it('aborts running calls, drops queued ones, and keeps what the next page also wants', async () => {
  const calls = engineCalls(2), {open, task} = held();
  const [keep, gone, queuedGone, queuedKeep] = ['keep', 'gone', 'queuedGone', 'queuedKeep'].map(group => calls.run(group, task(group)));
  const settled = Promise.allSettled([gone, queuedGone]);
  calls.keepOnly(new Set(['keep', 'queuedKeep']));
  expect(open[0].signal.aborted).toBe(false);
  expect(open[1].signal.aborted).toBe(true);
  expect((await settled).map(result => result.status === 'rejected' && result.reason.name)).toEqual(['AbortError', 'AbortError']);
  await flush();
  expect(open.map(call => call.group)).toEqual(['keep', 'gone', 'queuedKeep']);
  open[0].finish(); open[2].finish();
  expect(await keep).toBe('keep');
  expect(await queuedKeep).toBe('queuedKeep');
});

it('frees the slot when a task fails', async () => {
  const calls = engineCalls(1), order: string[] = [];
  const failing = calls.run('a', async () => {order.push('a'); throw new Error('boom');});
  const next = calls.run('b', async () => {order.push('b'); return 'b';});
  await expect(failing).rejects.toThrow('boom');
  expect(await next).toBe('b');
  expect(order).toEqual(['a', 'b']);
});
