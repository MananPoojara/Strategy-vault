// From claude-continuation-20261020/author-profiles/profiles.json (researched 22 Oct 2026; sources cited per author).
// Each author's profile is `profile` in src/catalogue/authors/<id>.ts.
import {AUTHOR_FILES} from '../catalogue/author-files';
export const AUTHOR_PROFILE_DATA = Object.fromEntries(AUTHOR_FILES.filter(f => f.profile).map(f => [f.author.id, f.profile!]));
