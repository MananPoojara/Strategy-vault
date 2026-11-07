import {afterEach, describe, expect, it, vi} from 'vitest';

const made: {opts: unknown; stop: ReturnType<typeof vi.fn>; start: ReturnType<typeof vi.fn>}[] = [];
vi.mock('lenis', () => ({default: class { stop = vi.fn(); start = vi.fn(); constructor(public opts: unknown) { made.push(this as never); } }}));
vi.mock('lenis/dist/lenis.css', () => ({}));
const media = (reduce: boolean) => { window.matchMedia = ((q: string) => ({matches: reduce && q.includes('reduce')})) as never; };

describe('smooth scroll', () => {
  afterEach(() => { made.length = 0; document.body.style.overflow = ''; });
  it('starts Lenis with the lab settings and pauses while a popup locks the page', async () => {
    media(false);
    const {startSmoothScroll} = await import('../src/services/smooth-scroll');
    expect(startSmoothScroll()).not.toBeNull();
    expect(made[0].opts).toEqual({autoRaf: true, allowNestedScroll: true, anchors: true});
    expect(made[0].start).toHaveBeenCalledTimes(1);
    document.body.style.overflow = 'hidden';
    await new Promise(r => setTimeout(r, 0));
    expect(made[0].stop).toHaveBeenCalled();
  });
  it('stays off for reduced motion', async () => {
    media(true);
    const {startSmoothScroll} = await import('../src/services/smooth-scroll');
    expect(startSmoothScroll()).toBeNull();
    expect(made).toHaveLength(0);
  });
});
