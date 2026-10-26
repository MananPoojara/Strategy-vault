// Author mastery summaries shown on author pages (researched from cited public sources; see author-profiles-data.ts).
import {AUTHOR_PROFILE_DATA} from './author-profiles-data';

export type AuthorProfile = {kind: 'person' | 'collection'; mastery: string[]; summary: string; known_for?: string[]; sources?: string[]; confidence?: string};

const esc = (s: string) => s.replace(/[&<>"']/g, c => ({'&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'}[c]!));
const safeUrl = (u: string) => { try { const x = new URL(u); return /^https?:$/.test(x.protocol) ? x : null; } catch { return null; } };

export function authorProfileHtml(id: string): string {
  const p = (AUTHOR_PROFILE_DATA as Record<string, AuthorProfile>)[id];
  if (!p) return '';
  const links = (p.sources || []).map(safeUrl).filter((x): x is URL => !!x)
    .map(u => `<a href="${esc(u.href)}" target="_blank" rel="noopener noreferrer">${esc(u.hostname.replace(/^www\./, ''))}</a>`);
  return `<div class="gprofile" data-author-profile>`
    + (p.mastery.length ? `<div class="tags" aria-label="Known for">${p.mastery.map(t => `<span class="tag">${esc(t)}</span>`).join('')}</div>` : '')
    + `<p class="gbook">${esc(p.summary)}</p>`
    + (p.known_for?.length ? `<p class="gbook"><b>Known for:</b> ${esc(p.known_for.join(' · '))}</p>` : '')
    + (links.length ? `<p class="gbook gsrc">Sources: ${links.join(' · ')}</p>` : '')
    + `</div>`;
}
