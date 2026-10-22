import {describe, expect, it, vi} from 'vitest';

vi.mock('../src/services/author-profiles-data', () => ({AUTHOR_PROFILE_DATA: {
  x: {kind: 'person', mastery: ['Mean reversion'], summary: 'Buys <dips>', known_for: ['RSI(2)'], sources: ['https://en.wikipedia.org/wiki/X', 'javascript:alert(1)']},
}}));
const {authorProfileHtml} = await import('../src/services/author-profiles');

describe('author profile', () => {
  it('renders tags, escaped summary and only http(s) sources', () => {
    const h = authorProfileHtml('x');
    expect(h).toContain('<span class="tag">Mean reversion</span>');
    expect(h).toContain('Buys &lt;dips&gt;');
    expect(h).toContain('en.wikipedia.org');
    expect(h).not.toContain('javascript:');
  });
  it('renders nothing for an author without a profile', () => expect(authorProfileHtml('none')).toBe(''));
});
