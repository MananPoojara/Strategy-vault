# Indicators Lab — Phase 4 backend (saved strategies, review, YT Strategies) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a researcher save indicator-lab rule specs to their account, edit them safely, submit them for YT Strategies, and let an administrator publish (freezing an immutable snapshot) or reject them — through `/api/app/lab/strategies` on the existing auth/app service, with global/crypto strategies refused for YT until licensed data exists.

**Architecture:** One new SQL file `Context/guru/webapp/db/002-lab-strategies.sql` (tables `lab_strategy` and `lab_strategy_version`), applied by one added line in `bootstrap.mjs` next to the existing `001-app.sql` line. Pure validators and the status/permission decision `labAction()` are added to `contracts.mjs` (no DB, unit-tested). A new module `auth/src/lab-strategies.mjs` exports `registerLabStrategies(app, pool)` — the same shape as `registerBookLibrary(app, library)` (`book-library.mjs:174`) — and `server.mjs` gains two lines (import + registration right after `registerBookLibrary`, so the existing `/api/app/*` session and origin middleware covers every route). Every change to an existing row runs inside a transaction that locks the row (`SELECT … FOR UPDATE`), decides with `labAction()` and then writes, so a stale version is a 409 and two simultaneous saves cannot both win. No gateway, engine or frontend change.

**Tech Stack:** Node 22+ ESM; the auth service's existing dependencies only (`hono` 4.13.13, `@hono/node-server`, `better-auth` 1.7.7, `pg` 8.16.3); PostgreSQL 18 (compose); `node --test`.

**Spec:** `docs/superpowers/specs/2026-10-14-indicators-lab-design.md` (§1 Saving, Markets; §2/§2a constraints; §3 Auth/app bullet; §7 errors; §8 "contract tests for lab strategy routes and roles"; §9 phase 4). Format: `docs/superpowers/plans/2026-10-15-indicators-lab-phase2-rules-engine.md`. Phase 4's frontend half (Indicators page save/submit UI, YT Strategies tab, results/optimize presenter reuse) is a separate plan and consumes the API defined here.

**Planning evidence:** every code block below was run against a scratch copy of the auth service, `db/` and `tests/` taken from `Nandini` at `9fe0f21` (outside the repository, `npm ci` from the committed lockfile): `contracts.test.mjs` 17/17 (9 existing + 8 new), `lab-strategies.test.mjs` 5/5, `book-library.test.mjs` unchanged and passing (32 in-process tests in total). The DB-backed `lab-strategies.integration.test.mjs` (8 subtests) passed twice against a throwaway `postgres:17-alpine` container plus `node src/server.mjs` (no Compose project, no gateway, not the live deployment), including a server restart to prove the migration re-runs cleanly, the fixture cleanup leaving 0 rows, the DB `CHECK` refusing a crypto snapshot insert, and the test refusing to start with `VAULT_COMPOSE_PROJECT=strategy-vault`. Production runs PostgreSQL 18; nothing here is version-specific (identity columns, `jsonb_typeof`, partial index, `FOR UPDATE OF`).

## Global Constraints

- Branch Nandini; never main; no force-push; no git worktrees (user preference); commits without any Co-Authored-By trailer; never push (the user pushes).
- Book results untouched: protected files `connors.rs`, `books/**`, `readiness.rs`, `gate.rs`, `instrument_policy.rs`, `baskets.rs`, `accounting.rs`, `trade_validation.rs`, frontend `compat/*`, `ResearchPages.tsx` — no edits; `main.rs`, gateway `policy.rs` and `custom-pages.ts` additive only. **This phase changes no Rust at all** (engine and gateway untouched) and no frontend file.
- Auth service: existing npm dependencies only (no new packages, `package.json`/`package-lock.json` unchanged); follow the existing style (ESM, `pool.query` with `$n` parameters, `BEGIN/COMMIT/ROLLBACK` as in `server.mjs:63-70`, `vault_audit` rows for writes). Parameterized SQL only — the only interpolated SQL fragments are module constants (`COLUMNS`, `FROM`, `TRANSITIONS[action]` with `action` restricted by the route pattern).
- Migrations: new file `002-lab-strategies.sql` only; `001-app.sql` is never edited. Every statement is re-runnable (`IF NOT EXISTS`) because `bootstrap()` runs on every service start (`bootstrap.mjs:9`).
- Never start/recreate the shared live Compose project (`name: strategy-vault`, `compose.yaml:1`). DB-backed testing uses an isolated dev stack (`docs/DEVELOPMENT.md` "Two developers on one machine") and is a **manual gate item for the user**; automated tasks never start Docker.
- No secrets in code; no data files, `.env` or generated results committed.

## Review Focus

1. **Editing (or deleting, withdrawing) a published strategy** changes what YT Strategies shows — `published lab strategies are frozen and statuses move only along draft → submitted → published | rejected` (Task 1), `editing a published strategy or saving a stale version is a 409 and rolls back` (Task 3), integration subtest `publish freezes the reviewed version for every signed-in role` (Task 4).
2. **Another user (or a viewer) reading or changing someone's draft** — `another user cannot read or change a draft; published strategies are readable by every role` (Task 1), `another user reading or editing a draft gets 404 and nothing is written` (Task 3), integration `another user and a viewer cannot see the draft` (Task 4).
3. **A global/crypto (provisional TradingView) strategy reaching YT Strategies** — `global and crypto strategies cannot be submitted or published to YT Strategies` (Task 1), `lab migration is additive, re-runnable and keeps provisional markets out of published snapshots` (Task 2, DB `CHECK` backstop), `a crypto strategy cannot be submitted to YT Strategies` (Task 3), integration `crypto strategies save but cannot be submitted` (Task 4).
4. **Lost update** — an older tab or a simultaneous save silently overwrites newer work, or an admin publishes a version they did not review — `a stale version is a conflict, never a lost update` (Task 1), the 409 half of the Task 3 test, integration `a stale version is refused instead of overwriting`, `two simultaneous saves of one version: one wins, one conflicts` and the stale-version publish in `publish freezes …` (Task 4).
5. **Oversized or malformed spec stored** (then failing in the engine or bloating lists) — `lab specs over 8 KB are refused before storage`, `lab strategies accept a trimmed name, a v1 spec object, …` (Task 1), `viewers cannot create and oversized specs are refused before any query` (Task 3).

---

## Existing code this phase relies on (read-only unless stated)

| Item | Where | Used for |
|---|---|---|
| `roleAllows(role, action)` — `read` all roles; `write`/`optimize` admin + researcher; `admin` admin only | `contracts.mjs:11` | every permission decision |
| `ROLES` | `contracts.mjs:2` | unchanged; roles are `admin`, `researcher`, `viewer` |
| `/api/app/*` middleware: session required (`401 Please sign in.`), `c.set('user', session.user)`, non-GET/HEAD requests need an `Origin` in `allowedOrigins` (`403 Invalid request origin`) | `server.mjs:30-38` | sign-in + CSRF/origin protection for all lab routes (registered after it) |
| `registerBookLibrary(app, await createBookLibrary())` | `server.mjs:39` | registration point; lab routes registered on the next line |
| transaction pattern `pool.connect()` / `BEGIN` / `COMMIT` / `ROLLBACK` / `release()` + `vault_audit` insert | `server.mjs:63-70`, `:84-89` | lab writes |
| `app.onError` → `SyntaxError` (bad JSON body) becomes `400 Invalid request`; other errors `500` | `server.mjs:108-112` | unchanged; lab validation errors are answered inside the lab module (`LabRequestError`) so the user gets the specific message |
| `bodyLimit` 1 MB | `server.mjs:12` | outer bound; the 8 KB spec bound is in `validateLabStrategy` |
| `allowedOrigins`, `pool` (max 5 connections) | `config.mjs:8-9` | origin check, DB |
| Better Auth user table `"user"(id text, name, email, role)` | created by `getMigrations(auth.options)` in `bootstrap.mjs:7-8`; FKs `REFERENCES "user"(id)` in `001-app.sql` | `owner`, `reviewed_by`, `published_by` |
| `001-app.sql` applied on every start | `bootstrap.mjs:9` | the `002` line is a copy of it with the new file name |
| `COPY webapp/db /app/db` | `infra/auth.Dockerfile:6` | the new SQL file ships in the image with no Dockerfile change |
| gateway `.route("/api/app/{*path}", any(app_proxy))`; `app_proxy` forwards every method, cookie, content-type and origin; 1 MB body | `gateway/src/main.rs:441`, `:205-217`, `:83-100` | PUT/DELETE/POST reach the auth service unchanged; **no gateway change** |
| in-process Hono route test pattern (`createRequire(…auth/package.json)('hono')`, test middleware sets `user`) | `tests/book-library.test.mjs:9`, `:166-178` | `tests/lab-strategies.test.mjs` |
| full-stack test pattern (env file, sign-in, admin creates fixture users, cleanup via `docker compose … exec -T postgres psql`) | `tests/integration.test.mjs:10-17`, `:52-55` | `tests/lab-strategies.integration.test.mjs` (adds `-p <isolated project>` and refuses the live one) |
| `scripts/verify.sh:4` in-process test list | `Context/guru/webapp/scripts/verify.sh` | gains `tests/lab-strategies.test.mjs` |
| engine spec contract: `/api/{in\|us}/lab/run\|optimize?spec=<url-encoded JSON>`, spec ≤ 8 KB, ≤ 12 conditions, depth ≤ 16, errors `{status, error, label}` | Phase 2 | the saved `spec` is sent unchanged by the frontend; full validation stays in the engine |

## Decisions (spec ambiguities resolved)

1. **Roles.** Create, update, delete, duplicate, submit, withdraw need `write` (researcher, admin) — the same rule as notes and annotations (`server.mjs:60`, `:80`). Review (list submitted, publish, reject) needs `admin` (spec §3 "publish is admin-only"; §10 assumes admin approves). Viewers are read-only: they see the YT Strategies list and published strategies, and their own list (always empty because they cannot create). Admins are reviewers, not co-editors: they can **read** any strategy (needed to review) but only the owner can change it (`403 Only the owner can change this strategy.`).
2. **Visibility.** A strategy is readable by its owner, any admin, or — once published — every signed-in role. Anything else is `404 Strategy not found.` (not 403), so draft ids do not leak. `GET …/published` (YT Strategies) is readable by every signed-in role.
3. **Status machine** (`labAction`, Task 1): `draft|rejected --update--> draft (version+1)`; `draft|rejected --submit--> submitted`; `submitted --withdraw--> draft`; `submitted --publish(admin)--> published`; `submitted --reject(admin, note)--> rejected`; delete only from `draft|rejected`. **Published is terminal and frozen**: no update, delete, withdraw or re-review; to iterate, duplicate it (any write-role user may duplicate any strategy they can read; the copy is a new `draft` owned by them, version 1, name `Copy of …`). Editing a rejected strategy returns it to `draft`; the last review note stays visible until the next review.
4. **Versions and concurrency.** `version` counts content revisions (starts at 1, +1 per update; transitions do not bump it). `PUT`, `submit`, `withdraw`, `publish`, `reject` require the client's `version` in the body; a mismatch is `409 This strategy changed since you opened it (now version N). Reload before saving.` Publishing therefore freezes exactly the version the admin reviewed. The row is locked with `SELECT … FOR UPDATE OF s` inside the transaction before deciding, so concurrent requests serialize (proved by the integration race subtest). Delete and duplicate need no version (delete is refused unless the row is still `draft|rejected`).
5. **Publishable markets.** `LAB_MARKETS = in, us, gl, crypto`; `LAB_PUBLISHABLE_MARKETS = in, us`. Submit and publish on `gl`/`crypto` are `400` with "Global and crypto data are provisional TradingView exports; strategies on them cannot go to YT Strategies until licensed data replaces them." Saving, editing and duplicating them is allowed (internal research). Responses carry a derived `publishable` boolean so the frontend never re-implements the rule. The snapshot table's `CHECK (market IN ('in','us'))` is the DB backstop; widening it when licensed data arrives (Phase 5 or later) is a new migration plus a change of `LAB_PUBLISHABLE_MARKETS`, both covered by the Task 2 test.
6. **Snapshots.** Publishing inserts one immutable row into `lab_strategy_version` (`UNIQUE (strategy_id, version)`); no code path updates or deletes it. YT Strategies reads snapshots, not working rows. Snapshots outlive the author's account (`owner`, `published_by` `ON DELETE SET NULL`; `strategy_id` `ON DELETE SET NULL`). The private working rows go with the account (`lab_strategy.owner … ON DELETE CASCADE`, like `vault_settings`).
7. **Spec check is structural only:** a JSON object, ≤ 8192 bytes as `JSON.stringify` UTF-8 (the same text the frontend URL-encodes for the engine), `v === 1`, `entry` and `exit` are objects. Condition counts, operators, indicator ids and parameters stay the engine's job (Phase 2). Drafts may hold empty `entry.all`/`exit.any` while being built; the engine refuses them at run time. Stored as `jsonb` (key order/whitespace normalised; the engine's rules do not depend on key order).
8. **Field bounds.** `name` trimmed 1–120, no control characters; `instrument` trimmed 1–64, no control characters (free text, e.g. `NIFTY 50`, `basket:…`; existence is checked by the engine when run); `note` trimmed 1–1000, newlines/tabs allowed, other control characters refused; required to reject, optional to publish. `?status=` filter must be one of the four statuses.
9. **IDs.** `integer GENERATED ALWAYS AS IDENTITY` (pg returns a JS number; `bigserial` would come back as a string). Routes accept `[1-9][0-9]{0,8}` so a path id can never overflow `integer`; anything else is Hono's plain 404.
10. **Errors.** Lab errors are `{ "error": "<message>" }` with 400/403/404/409, matching the existing `/api/app/*` shape (`{error}`), not the engine's `{status, error, label}` (that contract is for engine results). A malformed JSON body stays the existing `400 Invalid request` (`server.mjs:109`).
11. **Lists** are capped at 200 rows and include `spec` (≤ 8 KB each). Mine: newest `updated_at` first. Review queue: oldest `submitted_at` first. YT: newest `published_at` first.
12. **Audit.** Each write adds `vault_audit(actor_id, 'lab.<create|update|delete|duplicate|submit|withdraw|publish|reject>', 'lab_strategy:<id>')` in the same transaction.
13. **Migration numbering.** `001-app.sql` is the only existing migration, so the new file is `002-lab-strategies.sql`; bootstrap gets one added line identical to the `001` line with the new name (no loop/refactor of the existing line, keeping the change additive).
14. **DB-backed tests.** No test in the repo can reach PostgreSQL without the deployed stack and private `.env` (`integration.test.mjs:10-11`, `persistence.test.mjs:9-10` read `~/.cache/strategy-vault-webapp/.env` and drive `docker compose` on the **live** project name). So automated tasks run only in-process tests (validators, transitions, routes with a recording fake pool). SQL behaviour is pinned by `tests/lab-strategies.integration.test.mjs`, which requires `VAULT_ENV_FILE` **and** `VAULT_COMPOSE_PROJECT`, refuses `strategy-vault`, and is a manual gate item (Task 5 Step 6).

## API (consumed by the Phase 4 frontend plan)

All paths are under the existing `/api/app/*` middleware: signed-in only (`401 {"error":"Please sign in."}`); non-GET requests need a trusted `Origin` (`403 {"error":"Invalid request origin"}`). Request bodies are JSON.

`Strategy` (working row):

```json
{ "id": 12, "owner": "<user id>", "ownerName": "Nandini", "name": "RSI dip", "spec": { "v": 1, "entry": {"all": []}, "exit": {"any": []} },
  "market": "in", "instrument": "NIFTY 50", "status": "draft|submitted|published|rejected", "version": 3, "publishable": true,
  "submittedAt": "2026-10-15T09:00:00.000Z" | null, "reviewedAt": null, "reviewNote": null,
  "createdAt": "2026-10-15T08:00:00.000Z", "updatedAt": "2026-10-15T08:30:00.000Z" }
```

`Snapshot` (YT Strategies): `{ "snapshotId": 4, "id": 12 | null, "version": 3, "name", "spec", "market": "in|us", "instrument", "ownerName": "Nandini" | null, "publishedAt" }`.

| Method & path | Role | Body | Success | Errors |
|---|---|---|---|---|
| `GET /api/app/lab/strategies[?status=draft\|submitted\|published\|rejected]` | any | — | `200 Strategy[]` (own only) | 400 unknown status |
| `POST /api/app/lab/strategies` | write | `{name, spec, market, instrument}` | `201 Strategy` (draft, v1) | 400 validation, 403 viewer |
| `GET /api/app/lab/strategies/published` | any | — | `200 Snapshot[]` | — |
| `GET /api/app/lab/strategies/review` | admin | — | `200 Strategy[]` (all `submitted`) | 403 |
| `GET /api/app/lab/strategies/:id` | owner, admin, or anyone if published | — | `200 Strategy` | 404 |
| `PUT /api/app/lab/strategies/:id` | write, owner | `{version, name, spec, market, instrument}` | `200 Strategy` (draft, version+1) | 400, 403, 404, 409 status/version |
| `DELETE /api/app/lab/strategies/:id` | write, owner | — | `204` | 403, 404, 409 status |
| `POST /api/app/lab/strategies/:id/duplicate` | write, can read | — | `201 Strategy` (new draft, caller owns) | 403, 404 |
| `POST /api/app/lab/strategies/:id/submit` | write, owner | `{version}` | `200 Strategy` (submitted) | 400 provisional market, 403, 404, 409 |
| `POST /api/app/lab/strategies/:id/withdraw` | write, owner | `{version}` | `200 Strategy` (draft) | 403, 404, 409 |
| `POST /api/app/lab/strategies/:id/publish` | admin | `{version, note?}` | `200 Strategy` (published) + snapshot row | 400 provisional market, 403, 404, 409 |
| `POST /api/app/lab/strategies/:id/reject` | admin | `{version, note}` | `200 Strategy` (rejected, `reviewNote`) | 400 note, 403, 404, 409 |

## File structure

```
Context/guru/webapp/db/002-lab-strategies.sql            NEW  lab_strategy, lab_strategy_version, indexes        (Task 2)
Context/guru/webapp/auth/src/bootstrap.mjs               +1 line after the 001-app.sql line                       (Task 2)
Context/guru/webapp/auth/src/contracts.mjs               + LabRequestError, LAB_* sets, validators, labCanRead, labAction (appended)  (Task 1)
Context/guru/webapp/auth/src/lab-strategies.mjs          NEW  registerLabStrategies(app, pool)                    (Task 3)
Context/guru/webapp/auth/src/server.mjs                  +2 lines (import, registration)                          (Task 3)
Context/guru/webapp/tests/contracts.test.mjs             + 8 tests appended                                       (Tasks 1–2)
Context/guru/webapp/tests/lab-strategies.test.mjs        NEW  in-process route tests (recording fake pool)        (Task 3)
Context/guru/webapp/scripts/verify.sh                    line 4 lists tests/lab-strategies.test.mjs               (Task 3)
Context/guru/webapp/tests/lab-strategies.integration.test.mjs  NEW  manual, isolated dev stack only               (Task 4)
```

Nothing else changes: no gateway, engine, frontend, compose, Dockerfile or `package.json` edit.

**Setup once per checkout (all tasks):** `npm --prefix Context/guru/webapp/auth ci` (the committed lockfile; `node_modules` is ignored by Git and by `check_upload.py`). The auth tests resolve `hono` through `auth/package.json`.

---

### Task 1: Lab validators and the status/permission decision

**Files:**
- Modify: `Context/guru/webapp/auth/src/contracts.mjs` (append after `validateAnnotation`, end of file; no existing line changes)
- Test: `Context/guru/webapp/tests/contracts.test.mjs` (append)

**Interfaces:**
- Produces: `class LabRequestError extends Error { status }` (HTTP status, default 400); `LAB_MARKETS` = `Set{in,us,gl,crypto}`; `LAB_PUBLISHABLE_MARKETS` = `Set{in,us}`; `LAB_STATUSES` = `Set{draft,submitted,published,rejected}`; `validateLabStrategy(body) -> {name, spec, market, instrument}` (trimmed; extra keys dropped); `validateLabVersion(v) -> v` (safe integer ≥ 1); `validateLabNote(note, required) -> string | null`; `labCanRead(row, user) -> boolean`; `labAction(row, user, action, version?) -> nextStatus` with `action ∈ duplicate|update|delete|submit|withdraw|publish|reject`, throwing `LabRequestError` 404 (not visible), 403 (role / not owner), 409 (status or version), 400 (provisional market). `row` needs `{owner, status, version, market}`; `user` is `{id, role}` as set by the middleware.

- [ ] **Step 1: Write the failing tests** (append to `tests/contracts.test.mjs`)

```js
const LAB_SPEC={v:1,direction:'long',entry:{all:[{lhs:{s:'rsi',p:{length:2}},op:'crosses_below',rhs:{value:10}}]},exit:{any:[{lhs:{s:'bars_held'},op:'>=',rhs:{value:10}}]}};
const LAB_ROW={id:7,owner:'u1',status:'draft',version:3,market:'in'};
const LAB_USERS={owner:{id:'u1',role:'researcher'},other:{id:'u2',role:'researcher'},viewer:{id:'u3',role:'viewer'},admin:{id:'u4',role:'admin'}};

test('lab strategies accept a trimmed name, a v1 spec object, a lab market and an instrument',async()=>{
 const {validateLabStrategy}=await import('../auth/src/contracts.mjs');
 assert.deepEqual(validateLabStrategy({name:'  RSI dip ',spec:LAB_SPEC,market:'crypto',instrument:' BTCUSD ',extra:1}),{name:'RSI dip',spec:LAB_SPEC,market:'crypto',instrument:'BTCUSD'});
 for(const body of [null,[],{...{spec:LAB_SPEC,market:'in',instrument:'X'},name:'   '},{name:'x'.repeat(121),spec:LAB_SPEC,market:'in',instrument:'X'},{name:'a\nb',spec:LAB_SPEC,market:'in',instrument:'X'},
  {name:'A',spec:'{"v":1}',market:'in',instrument:'X'},{name:'A',spec:[],market:'in',instrument:'X'},{name:'A',spec:{...LAB_SPEC,v:2},market:'in',instrument:'X'},{name:'A',spec:{v:1,entry:LAB_SPEC.entry},market:'in',instrument:'X'},{name:'A',spec:{v:1,entry:[],exit:{}},market:'in',instrument:'X'},
  {name:'A',spec:LAB_SPEC,market:'eu',instrument:'X'},{name:'A',spec:LAB_SPEC,market:'in',instrument:''},{name:'A',spec:LAB_SPEC,market:'in',instrument:'x'.repeat(65)}]) assert.throws(()=>validateLabStrategy(body),{status:400});
});

test('lab specs over 8 KB are refused before storage',async()=>{
 const {validateLabStrategy}=await import('../auth/src/contracts.mjs');
 const sized=n=>({...LAB_SPEC,pad:'x'.repeat(n-JSON.stringify({...LAB_SPEC,pad:''}).length)});
 assert.equal(Buffer.byteLength(JSON.stringify(sized(8192))),8192);
 assert.ok(validateLabStrategy({name:'A',spec:sized(8192),market:'in',instrument:'X'}));
 assert.throws(()=>validateLabStrategy({name:'A',spec:sized(8193),market:'in',instrument:'X'}),{status:400,message:/8 KB/});
 assert.throws(()=>validateLabStrategy({name:'A',spec:{...LAB_SPEC,pad:'é'.repeat(4000)},market:'in',instrument:'X'}),{message:/8 KB/});
});

test('lab versions and review notes are bounded',async()=>{
 const {validateLabVersion,validateLabNote}=await import('../auth/src/contracts.mjs');
 assert.equal(validateLabVersion(4),4);
 for(const v of [undefined,0,-1,1.5,'3',null]) assert.throws(()=>validateLabVersion(v),{status:400});
 assert.equal(validateLabNote(undefined,false),null);
 assert.equal(validateLabNote(' Needs a stop \n rule ',true),'Needs a stop \n rule');
 for(const note of [undefined,'  ','x'.repeat(1001),'a\u0000b',5]) assert.throws(()=>validateLabNote(note,true),{status:400});
});

test('another user cannot read or change a draft; published strategies are readable by every role',async()=>{
 const {labCanRead,labAction}=await import('../auth/src/contracts.mjs');
 assert.equal(labCanRead(LAB_ROW,LAB_USERS.owner),true);
 assert.equal(labCanRead(LAB_ROW,LAB_USERS.admin),true);
 for(const user of [LAB_USERS.other,LAB_USERS.viewer]){
  assert.equal(labCanRead(LAB_ROW,user),false);
  for(const action of ['update','delete','duplicate','submit']) assert.throws(()=>labAction(LAB_ROW,user,action,3),{status:404});
 }
 assert.equal(labCanRead(undefined,LAB_USERS.admin),false);
 const published={...LAB_ROW,status:'published'};
 for(const user of Object.values(LAB_USERS)) assert.equal(labCanRead(published,user),true);
 assert.equal(labAction(published,LAB_USERS.other,'duplicate'),'published');
 assert.throws(()=>labAction(published,LAB_USERS.viewer,'duplicate'),{status:403});
 assert.throws(()=>labAction(LAB_ROW,LAB_USERS.admin,'update',3),{status:403,message:/owner/});
});

test('published lab strategies are frozen and statuses move only along draft → submitted → published | rejected',async()=>{
 const {labAction}=await import('../auth/src/contracts.mjs');
 const at=status=>({...LAB_ROW,status});
 assert.equal(labAction(at('draft'),LAB_USERS.owner,'update',3),'draft');
 assert.equal(labAction(at('rejected'),LAB_USERS.owner,'update',3),'draft');
 assert.equal(labAction(at('draft'),LAB_USERS.owner,'submit',3),'submitted');
 assert.equal(labAction(at('rejected'),LAB_USERS.owner,'submit',3),'submitted');
 assert.equal(labAction(at('submitted'),LAB_USERS.owner,'withdraw',3),'draft');
 assert.equal(labAction(at('submitted'),LAB_USERS.admin,'publish',3),'published');
 assert.equal(labAction(at('submitted'),LAB_USERS.admin,'reject',3),'rejected');
 for(const action of ['update','delete','submit','withdraw']) assert.throws(()=>labAction(at('published'),LAB_USERS.owner,action,3),{status:409});
 for(const action of ['publish','reject']) assert.throws(()=>labAction(at('published'),LAB_USERS.admin,action,3),{status:409});
 for(const action of ['update','delete','submit']) assert.throws(()=>labAction(at('submitted'),LAB_USERS.owner,action,3),{status:409});
 assert.throws(()=>labAction(at('draft'),LAB_USERS.admin,'publish',3),{status:409});
 assert.throws(()=>labAction(at('submitted'),LAB_USERS.owner,'publish',3),{status:403});
 assert.throws(()=>labAction(at('submitted'),{id:'u1',role:'viewer'},'withdraw',3),{status:403});
});

test('a stale version is a conflict, never a lost update',async()=>{
 const {labAction}=await import('../auth/src/contracts.mjs');
 for(const [action,user,status] of [['update',LAB_USERS.owner,'draft'],['submit',LAB_USERS.owner,'draft'],['withdraw',LAB_USERS.owner,'submitted'],['publish',LAB_USERS.admin,'submitted'],['reject',LAB_USERS.admin,'submitted']])
  assert.throws(()=>labAction({...LAB_ROW,status},user,action,2),{status:409,message:/version 3/});
});

test('global and crypto strategies cannot be submitted or published to YT Strategies',async()=>{
 const {labAction,LAB_PUBLISHABLE_MARKETS}=await import('../auth/src/contracts.mjs');
 assert.deepEqual([...LAB_PUBLISHABLE_MARKETS],['in','us']);
 for(const market of ['gl','crypto']){
  assert.throws(()=>labAction({...LAB_ROW,market},LAB_USERS.owner,'submit',3),{status:400,message:/provisional/});
  assert.throws(()=>labAction({...LAB_ROW,market,status:'submitted'},LAB_USERS.admin,'publish',3),{status:400,message:/provisional/});
  assert.equal(labAction({...LAB_ROW,market},LAB_USERS.owner,'update',3),'draft');
 }
 assert.equal(labAction({...LAB_ROW,market:'us'},LAB_USERS.owner,'submit',3),'submitted');
});
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/webapp && node --test tests/contracts.test.mjs` → Expected: the 9 existing tests pass; the 7 new tests fail (`validateLabStrategy is not a function`, `labAction is not a function`, …).

- [ ] **Step 3: Implement** (append to `auth/src/contracts.mjs`)

```js
// Indicators lab saved strategies (spec §3). Full rule validation stays in the engine.
export class LabRequestError extends Error {
  constructor(message, status = 400) { super(message); this.status = status; }
}
export const LAB_MARKETS = new Set(['in', 'us', 'gl', 'crypto']);
// Global and crypto are provisional TradingView exports: internal research only until licensed data exists.
export const LAB_PUBLISHABLE_MARKETS = new Set(['in', 'us']);
export const LAB_STATUSES = new Set(['draft', 'submitted', 'published', 'rejected']);
const LAB_SPEC_BYTES = 8192;
const CONTROL = /[\x00-\x1f\x7f]/;
const NOTE_CONTROL = /[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/;

export function validateLabStrategy(body) {
  if (!body || typeof body !== 'object' || Array.isArray(body)) throw new LabRequestError('Provide a strategy name, rule spec, market and instrument.');
  const name = typeof body.name === 'string' ? body.name.trim() : '';
  if (!name || name.length > 120 || CONTROL.test(name)) throw new LabRequestError('Name must be 1–120 characters.');
  const spec = body.spec;
  if (!spec || typeof spec !== 'object' || Array.isArray(spec)) throw new LabRequestError('Rule spec must be a JSON object.');
  if (Buffer.byteLength(JSON.stringify(spec)) > LAB_SPEC_BYTES) throw new LabRequestError('Rule spec is larger than 8 KB.');
  if (spec.v !== 1) throw new LabRequestError('Rule spec version must be 1.');
  for (const part of ['entry', 'exit']) {
    if (!spec[part] || typeof spec[part] !== 'object' || Array.isArray(spec[part])) throw new LabRequestError(`Rule spec needs ${part} rules.`);
  }
  if (!LAB_MARKETS.has(body.market)) throw new LabRequestError('Market must be in, us, gl or crypto.');
  const instrument = typeof body.instrument === 'string' ? body.instrument.trim() : '';
  if (!instrument || instrument.length > 64 || CONTROL.test(instrument)) throw new LabRequestError('Instrument must be 1–64 characters.');
  return { name, spec, market: body.market, instrument };
}

export function validateLabVersion(version) {
  if (!Number.isSafeInteger(version) || version < 1) throw new LabRequestError('Provide the version you are changing.');
  return version;
}

export function validateLabNote(note, required) {
  if (note === undefined || note === null) {
    if (required) throw new LabRequestError('Give a reason for the rejection (1–1000 characters).');
    return null;
  }
  const text = typeof note === 'string' ? note.trim() : '';
  if (!text || text.length > 1000 || NOTE_CONTROL.test(text)) throw new LabRequestError('Review note must be 1–1000 characters.');
  return text;
}

export function labCanRead(row, user) {
  return Boolean(row) && (row.owner === user.id || row.status === 'published' || roleAllows(user.role, 'admin'));
}

const LAB_ACTIONS = {
  duplicate: { role: 'write' },
  update: { role: 'write', owner: true, from: ['draft', 'rejected'], to: 'draft', verb: 'edited' },
  delete: { role: 'write', owner: true, from: ['draft', 'rejected'], verb: 'deleted' },
  submit: { role: 'write', owner: true, from: ['draft', 'rejected'], to: 'submitted', verb: 'submitted', publishable: true },
  withdraw: { role: 'write', owner: true, from: ['submitted'], to: 'draft', verb: 'withdrawn' },
  publish: { role: 'admin', from: ['submitted'], to: 'published', verb: 'published', publishable: true },
  reject: { role: 'admin', from: ['submitted'], to: 'rejected', verb: 'rejected' },
};

// Decides one action on a locked row; returns the next status or throws with the HTTP status to send.
export function labAction(row, user, action, version) {
  const rule = LAB_ACTIONS[action];
  if (!labCanRead(row, user)) throw new LabRequestError('Strategy not found.', 404);
  if (!roleAllows(user.role, rule.role)) throw new LabRequestError(rule.role === 'admin' ? 'Administrator access required' : 'Researcher access required', 403);
  if (rule.owner && row.owner !== user.id) throw new LabRequestError('Only the owner can change this strategy.', 403);
  if (rule.from && !rule.from.includes(row.status)) throw new LabRequestError(`A ${row.status} strategy cannot be ${rule.verb}.`, 409);
  if (version !== undefined && version !== row.version) throw new LabRequestError(`This strategy changed since you opened it (now version ${row.version}). Reload before saving.`, 409);
  if (rule.publishable && !LAB_PUBLISHABLE_MARKETS.has(row.market)) throw new LabRequestError('Global and crypto data are provisional TradingView exports; strategies on them cannot go to YT Strategies until licensed data replaces them.', 400);
  return rule.to ?? row.status;
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cd Context/guru/webapp && node --test tests/contracts.test.mjs` → Expected: `# pass 16`, `# fail 0`.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/webapp/auth/src/contracts.mjs Context/guru/webapp/tests/contracts.test.mjs
git commit -m "Add lab strategy validators and status rules"
```

---

### Task 2: Migration `002-lab-strategies.sql` applied by bootstrap

**Files:**
- Create: `Context/guru/webapp/db/002-lab-strategies.sql`
- Modify: `Context/guru/webapp/auth/src/bootstrap.mjs` (one added line after line 9)
- Test: `Context/guru/webapp/tests/contracts.test.mjs` (append)

**Interfaces:**
- Produces tables (Better Auth's quoted `"user"(id text)` as in `001-app.sql`):
  - `lab_strategy(id integer identity PK, owner text NOT NULL → "user" ON DELETE CASCADE, name varchar(120), spec jsonb object, market in|us|gl|crypto, instrument varchar(64), status draft|submitted|published|rejected DEFAULT draft, version integer ≥ 1 DEFAULT 1, submitted_at, reviewed_by → "user" SET NULL, reviewed_at, review_note varchar(1000), created_at, updated_at)`; indexes `(owner, updated_at DESC)` and partial `(submitted_at) WHERE status='submitted'`.
  - `lab_strategy_version(id integer identity PK, strategy_id → lab_strategy SET NULL, version, owner → "user" SET NULL, name, spec jsonb, market in|us, instrument, published_by → "user" SET NULL, published_at, UNIQUE(strategy_id, version))`; index `(published_at DESC)`.

- [ ] **Step 1: Write the failing test** (append to `tests/contracts.test.mjs`)

```js
test('lab migration is additive, re-runnable and keeps provisional markets out of published snapshots',async()=>{
 const {readFile}=await import('node:fs/promises');const {LAB_PUBLISHABLE_MARKETS,LAB_MARKETS}=await import('../auth/src/contracts.mjs');
 const sql=await readFile(new URL('../db/002-lab-strategies.sql',import.meta.url),'utf8');
 assert.doesNotMatch(sql,/^\s*(DROP|ALTER|DELETE|UPDATE|TRUNCATE|INSERT)\b/im);
 for(const statement of sql.match(/CREATE\s+(TABLE|INDEX)\s+\S+/gi)) assert.match(statement,/IF$/i);
 const checks=[...sql.matchAll(/market IN \(([^)]*)\)/g)].map(m=>m[1].replace(/'/g,'').split(','));
 assert.deepEqual(checks,[[...LAB_MARKETS],[...LAB_PUBLISHABLE_MARKETS]]);
 const bootstrap=await readFile(new URL('../auth/src/bootstrap.mjs',import.meta.url),'utf8');
 assert.ok(bootstrap.indexOf('001-app.sql')>0&&bootstrap.indexOf('002-lab-strategies.sql')>bootstrap.indexOf('001-app.sql'));
});
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/webapp && node --test tests/contracts.test.mjs` → Expected: the new test fails with `ENOENT … 002-lab-strategies.sql`.

- [ ] **Step 3: Create the migration** `Context/guru/webapp/db/002-lab-strategies.sql`

```sql
-- Indicators lab saved strategies (spec §3). Additive and idempotent: bootstrap runs it on every start.
CREATE TABLE IF NOT EXISTS lab_strategy (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    owner text NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
    name varchar(120) NOT NULL,
    spec jsonb NOT NULL CHECK (jsonb_typeof(spec) = 'object'),
    market varchar(8) NOT NULL CHECK (market IN ('in','us','gl','crypto')),
    instrument varchar(64) NOT NULL,
    status varchar(10) NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','submitted','published','rejected')),
    version integer NOT NULL DEFAULT 1 CHECK (version >= 1),
    submitted_at timestamptz,
    reviewed_by text REFERENCES "user"(id) ON DELETE SET NULL,
    reviewed_at timestamptz,
    review_note varchar(1000),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS lab_strategy_owner_idx ON lab_strategy(owner, updated_at DESC);
CREATE INDEX IF NOT EXISTS lab_strategy_submitted_idx ON lab_strategy(submitted_at) WHERE status = 'submitted';
-- Immutable snapshot of each published version; it outlives the author's account and working row.
-- Only India and US are publishable until licensed global/crypto data exists (a later migration widens this).
CREATE TABLE IF NOT EXISTS lab_strategy_version (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    strategy_id integer REFERENCES lab_strategy(id) ON DELETE SET NULL,
    version integer NOT NULL,
    owner text REFERENCES "user"(id) ON DELETE SET NULL,
    name varchar(120) NOT NULL,
    spec jsonb NOT NULL,
    market varchar(8) NOT NULL CHECK (market IN ('in','us')),
    instrument varchar(64) NOT NULL,
    published_by text REFERENCES "user"(id) ON DELETE SET NULL,
    published_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (strategy_id, version)
);
CREATE INDEX IF NOT EXISTS lab_strategy_version_published_idx ON lab_strategy_version(published_at DESC);
```

- [ ] **Step 4: Apply it on start** — in `auth/src/bootstrap.mjs`, add this line directly after the existing `001-app.sql` line (line 9), changing nothing else:

```js
  await pool.query(await readFile(new URL('../../db/002-lab-strategies.sql', import.meta.url), 'utf8').catch(() => readFile(new URL('../../../db/002-lab-strategies.sql', import.meta.url), 'utf8')));
```

- [ ] **Step 5: Run to verify pass**

Run: `cd Context/guru/webapp && node --test tests/contracts.test.mjs && node --check auth/src/bootstrap.mjs` → Expected: `# pass 17`, `# fail 0`; no syntax error. (Applying the SQL to a real database is gate item Task 5 Step 6.)

- [ ] **Step 6: Commit**

```bash
git add Context/guru/webapp/db/002-lab-strategies.sql Context/guru/webapp/auth/src/bootstrap.mjs Context/guru/webapp/tests/contracts.test.mjs
git commit -m "Add lab strategy tables migration"
```

---

### Task 3: `/api/app/lab/strategies` routes

**Files:**
- Create: `Context/guru/webapp/auth/src/lab-strategies.mjs`
- Modify: `Context/guru/webapp/auth/src/server.mjs` (+2 lines)
- Modify: `Context/guru/webapp/scripts/verify.sh` (line 4: add `tests/lab-strategies.test.mjs` after `tests/contracts.test.mjs`)
- Test: `Context/guru/webapp/tests/lab-strategies.test.mjs`

**Interfaces:**
- Consumes: Task 1 exports; `pool` (`config.mjs:9`); `c.get('user')` set by `server.mjs:33`.
- Produces: `registerLabStrategies(app, pool)` registering the routes in the API table above. Route patterns: `${BASE}` (GET, POST), `${BASE}/published`, `${BASE}/review`, `${BASE}/:id{[1-9][0-9]{0,8}}` (GET, PUT, DELETE), `…/:id/duplicate`, `…/:id/:action{submit|withdraw|publish|reject}` (verified with hono 4.13.13: `/published` is not captured by `:id`, `/0`, 10-digit ids and unknown actions are 404).
- SQL (all parameterized):
  - read: `SELECT <COLUMNS> FROM lab_strategy s JOIN "user" u ON u.id = s.owner WHERE s.id=$1 [FOR UPDATE OF s]`
  - mine: `… WHERE s.owner=$1 AND ($2::text IS NULL OR s.status=$2) ORDER BY s.updated_at DESC LIMIT 200`
  - review: `… WHERE s.status='submitted' ORDER BY s.submitted_at LIMIT 200`
  - YT: `SELECT v.id AS "snapshotId", v.strategy_id AS id, v.version, v.name, v.spec, v.market, v.instrument, u.name AS "ownerName", v.published_at AS "publishedAt" FROM lab_strategy_version v LEFT JOIN "user" u ON u.id = v.owner ORDER BY v.published_at DESC LIMIT 200`
  - create/duplicate: `INSERT INTO lab_strategy (owner,name,spec,market,instrument) VALUES ($1,$2,$3::jsonb,$4,$5) RETURNING id`
  - update: `UPDATE lab_strategy SET name=$2, spec=$3::jsonb, market=$4, instrument=$5, status='draft', version=version+1, updated_at=now() WHERE id=$1`
  - transitions: `UPDATE lab_strategy SET <TRANSITIONS[action]>, updated_at=now() WHERE id=$1` (+ `$2` reviewer, `$3` note for publish/reject)
  - publish snapshot: `INSERT INTO lab_strategy_version (strategy_id,version,owner,name,spec,market,instrument,published_by) VALUES ($1,…,$8)`
  - delete: `DELETE FROM lab_strategy WHERE id=$1`; audit: `INSERT INTO vault_audit (actor_id,action,entity) VALUES ($1,$2,$3)`

- [ ] **Step 1: Write the failing tests** `Context/guru/webapp/tests/lab-strategies.test.mjs`

```js
// In-process route tests with a recording pool: no database, Docker or private .env needed.
// SQL behaviour itself is checked on an isolated dev stack by lab-strategies.integration.test.mjs.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { registerLabStrategies } from '../auth/src/lab-strategies.mjs';

const { Hono } = createRequire(new URL('../auth/package.json', import.meta.url))('hono');
const SPEC = { v: 1, entry: { all: [] }, exit: { any: [] } };
const USERS = { owner: { id: 'u1', role: 'researcher' }, other: { id: 'u2', role: 'researcher' }, viewer: { id: 'u3', role: 'viewer' }, admin: { id: 'u4', role: 'admin' } };

function harness(rows = {}) {
  const log = [];
  const query = async (sql, params = []) => {
    log.push(sql.trim().split(/\s+/)[0]);
    if (/^SELECT/.test(sql.trim()) && /WHERE s\.id=\$1/.test(sql)) return { rows: rows[params[0]] ? [rows[params[0]]] : [], rowCount: rows[params[0]] ? 1 : 0 };
    return { rows: [], rowCount: 0 };
  };
  const pool = { query, connect: async () => ({ query, release() {} }) };
  const app = new Hono();
  app.use('*', async (c, next) => { c.set('user', USERS[c.req.header('x-test-user')]); await next(); });
  registerLabStrategies(app, pool);
  const call = (user, method, path, body) => app.request(`/api/app/lab/strategies${path}`, { method, headers: { 'x-test-user': user, 'Content-Type': 'application/json' }, body: body === undefined ? undefined : JSON.stringify(body) });
  return { log, call };
}
const row = fields => ({ id: 7, owner: 'u1', ownerName: 'Owner', name: 'RSI dip', spec: SPEC, market: 'in', instrument: 'NIFTY 50', status: 'draft', version: 3, ...fields });

test('viewers cannot create and oversized specs are refused before any query', async () => {
  const { log, call } = harness();
  let response = await call('viewer', 'POST', '', { name: 'A', spec: SPEC, market: 'in', instrument: 'X' });
  assert.equal(response.status, 403);
  response = await call('owner', 'POST', '', { name: 'A', spec: { ...SPEC, pad: 'x'.repeat(8200) }, market: 'in', instrument: 'X' });
  assert.equal(response.status, 400);
  assert.match((await response.json()).error, /8 KB/);
  assert.deepEqual(log, []);
});

test('another user reading or editing a draft gets 404 and nothing is written', async () => {
  const { log, call } = harness({ 7: row() });
  assert.equal((await call('other', 'GET', '/7')).status, 404);
  assert.equal((await call('viewer', 'GET', '/7')).status, 404);
  assert.equal((await call('other', 'PUT', '/7', { version: 3, name: 'B', spec: SPEC, market: 'in', instrument: 'X' })).status, 404);
  assert.equal((await call('other', 'DELETE', '/7')).status, 404);
  assert.equal((await call('owner', 'GET', '/7')).status, 200);
  assert.equal((await call('admin', 'GET', '/7')).status, 200);
  assert.equal((await call('owner', 'GET', '/8')).status, 404);
  assert.ok(!log.includes('UPDATE') && !log.includes('DELETE') && !log.includes('INSERT'));
});

test('editing a published strategy or saving a stale version is a 409 and rolls back', async () => {
  const published = harness({ 7: row({ status: 'published' }) });
  let response = await published.call('owner', 'PUT', '/7', { version: 3, name: 'B', spec: SPEC, market: 'in', instrument: 'X' });
  assert.equal(response.status, 409);
  assert.match((await response.json()).error, /published strategy cannot be edited/);
  assert.equal((await published.call('owner', 'DELETE', '/7')).status, 409);
  const stale = harness({ 7: row() });
  response = await stale.call('owner', 'PUT', '/7', { version: 2, name: 'B', spec: SPEC, market: 'in', instrument: 'X' });
  assert.equal(response.status, 409);
  assert.match((await response.json()).error, /version 3/);
  assert.equal((await stale.call('owner', 'PUT', '/7', { name: 'B', spec: SPEC, market: 'in', instrument: 'X' })).status, 400);
  for (const { log } of [published, stale]) {
    assert.ok(!log.includes('UPDATE') && !log.includes('DELETE'));
    assert.equal(log.filter(s => s === 'ROLLBACK').length, log.filter(s => s === 'BEGIN').length);
  }
});

test('a crypto strategy cannot be submitted to YT Strategies', async () => {
  const { log, call } = harness({ 7: row({ market: 'crypto', instrument: 'BTCUSD' }) });
  const response = await call('owner', 'POST', '/7/submit', { version: 3 });
  assert.equal(response.status, 400);
  assert.match((await response.json()).error, /provisional/);
  assert.ok(!log.includes('UPDATE'));
});

test('review routes are administrator-only and rejection needs a note', async () => {
  const { log, call } = harness({ 7: row({ status: 'submitted' }) });
  assert.equal((await call('owner', 'GET', '/review')).status, 403);
  assert.equal((await call('owner', 'POST', '/7/publish', { version: 3 })).status, 403);
  assert.equal((await call('admin', 'POST', '/7/reject', { version: 3 })).status, 400);
  assert.equal((await call('admin', 'POST', '/7/reject', { version: 3, note: 'x'.repeat(1001) })).status, 400);
  assert.equal((await call('owner', 'GET', '?status=archived')).status, 400);
  assert.ok(!log.includes('UPDATE'));
});
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/webapp && node --test tests/lab-strategies.test.mjs` → Expected: `ERR_MODULE_NOT_FOUND … auth/src/lab-strategies.mjs`.

- [ ] **Step 3: Implement** `Context/guru/webapp/auth/src/lab-strategies.mjs`

```js
// Saved indicators-lab strategies (spec §3): per-user drafts, admin review, frozen YT Strategies snapshots.
import { roleAllows, LabRequestError, LAB_PUBLISHABLE_MARKETS, LAB_STATUSES, labAction, labCanRead, validateLabStrategy, validateLabVersion, validateLabNote } from './contracts.mjs';

const BASE = '/api/app/lab/strategies';
const ID = ':id{[1-9][0-9]{0,8}}';
const COLUMNS = `s.id, s.owner, u.name AS "ownerName", s.name, s.spec, s.market, s.instrument, s.status, s.version,
  s.submitted_at AS "submittedAt", s.reviewed_at AS "reviewedAt", s.review_note AS "reviewNote", s.created_at AS "createdAt", s.updated_at AS "updatedAt"`;
const FROM = 'FROM lab_strategy s JOIN "user" u ON u.id = s.owner';
const TRANSITIONS = {
  submit: "status='submitted', submitted_at=now()",
  withdraw: "status='draft', submitted_at=NULL",
  publish: "status='published', reviewed_by=$2, reviewed_at=now(), review_note=$3",
  reject: "status='rejected', reviewed_by=$2, reviewed_at=now(), review_note=$3",
};
const view = row => row && { ...row, publishable: LAB_PUBLISHABLE_MARKETS.has(row.market) };

async function body(c) {
  const data = await c.req.json();
  if (!data || typeof data !== 'object' || Array.isArray(data)) throw new LabRequestError('Provide a JSON object.');
  return data;
}

export function registerLabStrategies(app, pool) {
  const route = handler => async c => {
    try { return await handler(c, c.get('user')); }
    catch (error) { if (error instanceof LabRequestError) return c.json({ error: error.message }, error.status); throw error; }
  };
  // Locks the row so concurrent edits, transitions and reviews apply one at a time.
  const locked = async (id, work) => {
    const client = await pool.connect();
    try {
      await client.query('BEGIN');
      const row = (await client.query(`SELECT ${COLUMNS} ${FROM} WHERE s.id=$1 FOR UPDATE OF s`, [id])).rows[0];
      const result = await work(client, row);
      await client.query('COMMIT');
      return result;
    } catch (error) { await client.query('ROLLBACK'); throw error; } finally { client.release(); }
  };
  const reread = async (client, id) => view((await client.query(`SELECT ${COLUMNS} ${FROM} WHERE s.id=$1`, [id])).rows[0]);
  const audit = (client, user, action, id) => client.query('INSERT INTO vault_audit (actor_id,action,entity) VALUES ($1,$2,$3)', [user.id, `lab.${action}`, `lab_strategy:${id}`]);
  const insert = async (client, user, data, action) => {
    const id = (await client.query('INSERT INTO lab_strategy (owner,name,spec,market,instrument) VALUES ($1,$2,$3::jsonb,$4,$5) RETURNING id', [user.id, data.name, JSON.stringify(data.spec), data.market, data.instrument])).rows[0].id;
    await audit(client, user, action, id);
    return reread(client, id);
  };

  app.get(BASE, route(async (c, user) => {
    const status = c.req.query('status') ?? null;
    if (status !== null && !LAB_STATUSES.has(status)) throw new LabRequestError('Unknown status filter.');
    const result = await pool.query(`SELECT ${COLUMNS} ${FROM} WHERE s.owner=$1 AND ($2::text IS NULL OR s.status=$2) ORDER BY s.updated_at DESC LIMIT 200`, [user.id, status]);
    return c.json(result.rows.map(view));
  }));
  app.get(`${BASE}/published`, route(async c => {
    const result = await pool.query(`SELECT v.id AS "snapshotId", v.strategy_id AS id, v.version, v.name, v.spec, v.market, v.instrument, u.name AS "ownerName", v.published_at AS "publishedAt"
      FROM lab_strategy_version v LEFT JOIN "user" u ON u.id = v.owner ORDER BY v.published_at DESC LIMIT 200`);
    return c.json(result.rows);
  }));
  app.get(`${BASE}/review`, route(async (c, user) => {
    if (!roleAllows(user.role, 'admin')) throw new LabRequestError('Administrator access required', 403);
    const result = await pool.query(`SELECT ${COLUMNS} ${FROM} WHERE s.status='submitted' ORDER BY s.submitted_at LIMIT 200`);
    return c.json(result.rows.map(view));
  }));
  app.get(`${BASE}/${ID}`, route(async (c, user) => {
    const row = (await pool.query(`SELECT ${COLUMNS} ${FROM} WHERE s.id=$1`, [Number(c.req.param('id'))])).rows[0];
    if (!labCanRead(row, user)) throw new LabRequestError('Strategy not found.', 404);
    return c.json(view(row));
  }));
  app.post(BASE, route(async (c, user) => {
    if (!roleAllows(user.role, 'write')) throw new LabRequestError('Researcher access required', 403);
    const data = validateLabStrategy(await body(c));
    const client = await pool.connect();
    try {
      await client.query('BEGIN');
      const created = await insert(client, user, data, 'create');
      await client.query('COMMIT');
      return c.json(created, 201);
    } catch (error) { await client.query('ROLLBACK'); throw error; } finally { client.release(); }
  }));
  app.put(`${BASE}/${ID}`, route(async (c, user) => {
    const input = await body(c);
    const version = validateLabVersion(input.version), data = validateLabStrategy(input);
    return c.json(await locked(Number(c.req.param('id')), async (client, row) => {
      labAction(row, user, 'update', version);
      await client.query("UPDATE lab_strategy SET name=$2, spec=$3::jsonb, market=$4, instrument=$5, status='draft', version=version+1, updated_at=now() WHERE id=$1", [row.id, data.name, JSON.stringify(data.spec), data.market, data.instrument]);
      await audit(client, user, 'update', row.id);
      return reread(client, row.id);
    }));
  }));
  app.delete(`${BASE}/${ID}`, route(async (c, user) => {
    await locked(Number(c.req.param('id')), async (client, row) => {
      labAction(row, user, 'delete');
      await client.query('DELETE FROM lab_strategy WHERE id=$1', [row.id]);
      await audit(client, user, 'delete', row.id);
    });
    return c.body(null, 204);
  }));
  app.post(`${BASE}/${ID}/duplicate`, route(async (c, user) => {
    const copy = await locked(Number(c.req.param('id')), async (client, row) => {
      labAction(row, user, 'duplicate');
      return insert(client, user, { ...row, name: `Copy of ${row.name}`.slice(0, 120) }, 'duplicate');
    });
    return c.json(copy, 201);
  }));
  app.post(`${BASE}/${ID}/:action{submit|withdraw|publish|reject}`, route(async (c, user) => {
    const action = c.req.param('action'), input = await body(c);
    const version = validateLabVersion(input.version);
    const note = action === 'publish' || action === 'reject' ? validateLabNote(input.note, action === 'reject') : null;
    return c.json(await locked(Number(c.req.param('id')), async (client, row) => {
      labAction(row, user, action, version);
      const review = action === 'publish' || action === 'reject';
      await client.query(`UPDATE lab_strategy SET ${TRANSITIONS[action]}, updated_at=now() WHERE id=$1`, review ? [row.id, user.id, note] : [row.id]);
      // The published snapshot is insert-only: no route updates or deletes lab_strategy_version.
      if (action === 'publish') await client.query('INSERT INTO lab_strategy_version (strategy_id,version,owner,name,spec,market,instrument,published_by) VALUES ($1,$2,$3,$4,$5::jsonb,$6,$7,$8)', [row.id, row.version, row.owner, row.name, JSON.stringify(row.spec), row.market, row.instrument, user.id]);
      await audit(client, user, action, row.id);
      return reread(client, row.id);
    }));
  }));
}
```

- [ ] **Step 4: Register in `server.mjs`** — two added lines, nothing else changes:

```js
// after line 8: import { createBookLibrary, registerBookLibrary } from './book-library.mjs';
import { registerLabStrategies } from './lab-strategies.mjs';
// after line 39: registerBookLibrary(app, await createBookLibrary());
registerLabStrategies(app, pool);
```

Registering after the `/api/app/*` middleware (`server.mjs:30-38`) is what gives every lab route the session check and, for PUT/POST/DELETE, the trusted-`Origin` check — the same protection as notes, settings and annotations. Do not add another origin check.

- [ ] **Step 5: Add the test to the in-process list** — `scripts/verify.sh` line 4 becomes:

```bash
node --experimental-test-coverage --test tests/contracts.test.mjs tests/lab-strategies.test.mjs tests/backtest-trust.test.mjs tests/research-review.test.mjs tests/optimization-export.test.mjs tests/analyst-conclusion.test.mjs tests/conclusion-presentation.test.mjs tests/report-data.test.mjs tests/legacy-response.test.mjs
```

- [ ] **Step 6: Run to verify pass**

Run: `cd Context/guru/webapp && node --test tests/contracts.test.mjs tests/lab-strategies.test.mjs tests/book-library.test.mjs && node --check auth/src/server.mjs` → Expected: `# fail 0` (17 + 5 + the unchanged book-library tests; 32 at planning time).

- [ ] **Step 7: Commit**

```bash
git add Context/guru/webapp/auth/src/lab-strategies.mjs Context/guru/webapp/auth/src/server.mjs Context/guru/webapp/scripts/verify.sh Context/guru/webapp/tests/lab-strategies.test.mjs
git commit -m "Add saved lab strategy routes"
```

---

### Task 4: DB-backed route test for an isolated dev stack (manual gate)

**Files:**
- Create: `Context/guru/webapp/tests/lab-strategies.integration.test.mjs`

**Interfaces:**
- Consumes: a running **isolated** stack (`docs/DEVELOPMENT.md` "Two developers on one machine") built from this branch; env `VAULT_ENV_FILE` (that stack's private `.env`: `VAULT_PUBLIC_URL`, `VAULT_ADMIN_EMAIL`, `VAULT_ADMIN_PASSWORD`), `VAULT_COMPOSE_PROJECT` (its `-p` name, never `strategy-vault`), optional `VAULT_TEST_URL`.
- Produces: 8 subtests over real PostgreSQL; creates two researchers and one viewer with a random suffix and removes them, their strategies, snapshots and audit rows in `finally` via `docker compose -p <project> … exec -T postgres psql`.

- [ ] **Step 1: Create the test** (it is not run by any automated task)

```js
// Manual gate: saved lab strategies against an ISOLATED dev stack (docs/DEVELOPMENT.md "Two developers on one machine").
// VAULT_ENV_FILE=<dev runtime>/.env VAULT_COMPOSE_PROJECT=strategy-vault-dev-<name> node --test tests/lab-strategies.integration.test.mjs
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { randomBytes } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const { VAULT_ENV_FILE: envFile, VAULT_COMPOSE_PROJECT: project } = process.env;
if (!envFile || !project || project === 'strategy-vault') throw new Error('Set VAULT_ENV_FILE and VAULT_COMPOSE_PROJECT to an isolated dev stack; the live project "strategy-vault" is refused.');
const appDirectory = fileURLToPath(new URL('..', import.meta.url));
const env = Object.fromEntries((await readFile(envFile, 'utf8')).split('\n').filter(x => x && !x.startsWith('#')).map(x => { const i = x.indexOf('='); return [x.slice(0, i), x.slice(i + 1)]; }));
const base = process.env.VAULT_TEST_URL || env.VAULT_PUBLIC_URL;
async function call(path, { cookie = '', method = 'GET', body, origin = base } = {}) {
  const response = await fetch(base + path, { method, headers: { ...(cookie ? { Cookie: cookie } : {}), ...(method !== 'GET' ? { Origin: origin, 'Content-Type': 'application/json' } : {}) }, body: body === undefined ? undefined : JSON.stringify(body), redirect: 'manual' });
  const text = await response.text();
  return { status: response.status, text, json: () => JSON.parse(text) };
}
async function signIn(email, password) {
  const response = await fetch(base + '/api/auth/sign-in/email', { method: 'POST', headers: { Origin: base, 'Content-Type': 'application/json' }, body: JSON.stringify({ email, password }) });
  assert.equal(response.status, 200, await response.clone().text());
  return response.headers.getSetCookie().map(x => x.split(';')[0]).join('; ');
}
const fixture = randomBytes(5).toString('hex'), password = randomBytes(20).toString('hex');
const roles = ['researcher', 'researcher', 'viewer'], emails = roles.map((role, i) => `lab${i}.${fixture}@smoke.vault.local`);
const SPEC = { v: 1, direction: 'long', entry: { all: [{ lhs: { s: 'rsi', p: { length: 2 } }, op: 'crosses_below', rhs: { value: 10 } }] }, exit: { any: [{ lhs: { s: 'bars_held' }, op: '>=', rhs: { value: 10 } }] } };
const S = '/api/app/lab/strategies';

await test('saved lab strategies: ownership, versions, review and frozen YT snapshots', async t => {
  let admin, owner, other, viewer, id;
  try {
    admin = await signIn(env.VAULT_ADMIN_EMAIL, env.VAULT_ADMIN_PASSWORD);
    for (const [i, role] of roles.entries()) assert.equal((await call('/api/app/admin/users', { cookie: admin, method: 'POST', body: { name: `Lab ${i}`, email: emails[i], password, role } })).status, 201);
    [owner, other, viewer] = await Promise.all(emails.map(email => signIn(email, password)));
    await t.test('create needs sign-in, write role and a trusted origin', async () => {
      const draft = { name: ' RSI dip ', spec: SPEC, market: 'in', instrument: 'NIFTY 50' };
      assert.equal((await call(S, { method: 'POST', body: draft })).status, 401);
      assert.equal((await call(S, { cookie: viewer, method: 'POST', body: draft })).status, 403);
      assert.equal((await call(S, { cookie: owner, method: 'POST', body: draft, origin: 'https://untrusted.example' })).status, 403);
      assert.equal((await call(S, { cookie: owner, method: 'POST', body: { ...draft, spec: { ...SPEC, pad: 'x'.repeat(8200) } } })).status, 400);
      const created = await call(S, { cookie: owner, method: 'POST', body: draft });
      assert.equal(created.status, 201, created.text);
      const json = created.json(); id = json.id;
      assert.deepEqual({ name: json.name, status: json.status, version: json.version, publishable: json.publishable, spec: json.spec }, { name: 'RSI dip', status: 'draft', version: 1, publishable: true, spec: SPEC });
    });
    await t.test('another user and a viewer cannot see the draft', async () => {
      assert.equal((await call(`${S}/${id}`, { cookie: other })).status, 404);
      assert.equal((await call(`${S}/${id}`, { cookie: viewer })).status, 404);
      assert.equal((await call(`${S}/${id}`, { cookie: other, method: 'PUT', body: { version: 1, name: 'Hijack', spec: SPEC, market: 'in', instrument: 'NIFTY 50' } })).status, 404);
      assert.deepEqual((await call(S, { cookie: other })).json(), []);
      assert.equal((await call(S + '?status=draft', { cookie: owner })).json()[0].id, id);
    });
    await t.test('a stale version is refused instead of overwriting', async () => {
      const first = await call(`${S}/${id}`, { cookie: owner, method: 'PUT', body: { version: 1, name: 'RSI dip v2', spec: SPEC, market: 'in', instrument: 'NIFTY 50' } });
      assert.equal(first.status, 200, first.text); assert.equal(first.json().version, 2);
      const stale = await call(`${S}/${id}`, { cookie: owner, method: 'PUT', body: { version: 1, name: 'Lost update', spec: SPEC, market: 'in', instrument: 'NIFTY 50' } });
      assert.equal(stale.status, 409);
      assert.equal((await call(`${S}/${id}`, { cookie: owner })).json().name, 'RSI dip v2');
    });
    await t.test('submit, withdraw, reject with a note, edit, resubmit', async () => {
      assert.equal((await call(`${S}/${id}/submit`, { cookie: owner, method: 'POST', body: { version: 2 } })).json().status, 'submitted');
      assert.equal((await call(`${S}/${id}`, { cookie: owner, method: 'PUT', body: { version: 2, name: 'x', spec: SPEC, market: 'in', instrument: 'NIFTY 50' } })).status, 409);
      assert.equal((await call(`${S}/${id}/withdraw`, { cookie: owner, method: 'POST', body: { version: 2 } })).json().status, 'draft');
      assert.equal((await call(`${S}/${id}/submit`, { cookie: owner, method: 'POST', body: { version: 2 } })).status, 200);
      assert.equal((await call(`${S}/review`, { cookie: owner })).status, 403);
      assert.ok((await call(`${S}/review`, { cookie: admin })).json().some(s => s.id === id));
      assert.equal((await call(`${S}/${id}/reject`, { cookie: admin, method: 'POST', body: { version: 2 } })).status, 400);
      const rejected = await call(`${S}/${id}/reject`, { cookie: admin, method: 'POST', body: { version: 2, note: 'Add a time exit.' } });
      assert.deepEqual([rejected.json().status, rejected.json().reviewNote], ['rejected', 'Add a time exit.']);
      const edited = await call(`${S}/${id}`, { cookie: owner, method: 'PUT', body: { version: 2, name: 'RSI dip v3', spec: SPEC, market: 'in', instrument: 'NIFTY 50' } });
      assert.deepEqual([edited.json().status, edited.json().version], ['draft', 3]);
      assert.equal((await call(`${S}/${id}/submit`, { cookie: owner, method: 'POST', body: { version: 3 } })).status, 200);
    });
    await t.test('publish freezes the reviewed version for every signed-in role', async () => {
      assert.equal((await call(`${S}/${id}/publish`, { cookie: owner, method: 'POST', body: { version: 3 } })).status, 403);
      assert.equal((await call(`${S}/${id}/publish`, { cookie: admin, method: 'POST', body: { version: 2 } })).status, 409);
      const published = await call(`${S}/${id}/publish`, { cookie: admin, method: 'POST', body: { version: 3 } });
      assert.equal(published.json().status, 'published', published.text);
      for (const [method, path, body] of [['PUT', `${S}/${id}`, { version: 3, name: 'x', spec: SPEC, market: 'in', instrument: 'NIFTY 50' }], ['DELETE', `${S}/${id}`], ['POST', `${S}/${id}/withdraw`, { version: 3 }]])
        assert.equal((await call(path, { cookie: owner, method, body })).status, 409);
      const snapshot = (await call(`${S}/published`, { cookie: viewer })).json().find(s => s.id === id);
      assert.deepEqual({ version: snapshot.version, name: snapshot.name, spec: snapshot.spec, market: snapshot.market }, { version: 3, name: 'RSI dip v3', spec: SPEC, market: 'in' });
      assert.equal((await call(`${S}/${id}`, { cookie: viewer })).status, 200);
      const copy = await call(`${S}/${id}/duplicate`, { cookie: other, method: 'POST' });
      assert.equal(copy.status, 201);
      assert.deepEqual({ name: copy.json().name, status: copy.json().status, version: copy.json().version }, { name: 'Copy of RSI dip v3', status: 'draft', version: 1 });
      assert.equal((await call(`${S}/${copy.json().id}`, { cookie: other, method: 'DELETE' })).status, 204);
    });
    await t.test('crypto strategies save but cannot be submitted', async () => {
      const crypto = (await call(S, { cookie: owner, method: 'POST', body: { name: 'BTC dip', spec: SPEC, market: 'crypto', instrument: 'BTCUSD' } })).json();
      assert.equal(crypto.publishable, false);
      const refused = await call(`${S}/${crypto.id}/submit`, { cookie: owner, method: 'POST', body: { version: 1 } });
      assert.equal(refused.status, 400); assert.match(refused.json().error, /provisional/);
    });
    await t.test('two simultaneous saves of one version: one wins, one conflicts', async () => {
      const draft = (await call(S, { cookie: owner, method: 'POST', body: { name: 'Race', spec: SPEC, market: 'us', instrument: 'SPY' } })).json();
      const save = name => call(`${S}/${draft.id}`, { cookie: owner, method: 'PUT', body: { version: 1, name, spec: SPEC, market: 'us', instrument: 'SPY' } });
      const results = await Promise.all([save('A'), save('B')]);
      assert.deepEqual(results.map(r => r.status).sort(), [200, 409]);
      const saved = (await call(`${S}/${draft.id}`, { cookie: owner })).json();
      assert.deepEqual([saved.version, saved.name], [2, results.find(r => r.status === 200).json().name]);
    });
  } finally {
    // Only this run's fixture users and their strategies are removed.
    const users = `SELECT id FROM "user" WHERE email IN ('${emails.join("','")}')`;
    const sql = `DELETE FROM vault_audit WHERE actor_id IN (${users}) OR entity IN (${users}) OR entity IN (SELECT 'lab_strategy:'||id FROM lab_strategy WHERE owner IN (${users})); DELETE FROM lab_strategy_version WHERE strategy_id IN (SELECT id FROM lab_strategy WHERE owner IN (${users})); DELETE FROM "user" WHERE email IN ('${emails.join("','")}');`;
    execFileSync('docker', ['compose', '-p', project, '--project-directory', appDirectory, '--env-file', envFile, 'exec', '-T', 'postgres', 'psql', '-U', 'vault', '-d', 'vault', '-v', 'ON_ERROR_STOP=1', '-c', sql], { stdio: ['ignore', 'pipe', 'pipe'] });
  }
});
```

- [ ] **Step 2: Prove it cannot touch the live deployment** — automated, no Docker:

Run: `cd Context/guru/webapp && node --test tests/lab-strategies.integration.test.mjs 2>&1 | grep -q 'live project "strategy-vault" is refused' && echo refused` → Expected: `refused` (no env set → it throws before any request); and `VAULT_ENV_FILE=/dev/null VAULT_COMPOSE_PROJECT=strategy-vault node --test tests/lab-strategies.integration.test.mjs 2>&1 | grep -q 'is refused' && echo refused` → `refused`.

- [ ] **Step 3: Commit**

```bash
git add Context/guru/webapp/tests/lab-strategies.integration.test.mjs
git commit -m "Add isolated-stack test for saved lab strategies"
```

---

### Task 5: Phase gate

- [ ] **Step 1: Auth in-process tests** — `npm --prefix Context/guru/webapp/auth ci && cd Context/guru/webapp && node --test tests/contracts.test.mjs tests/lab-strategies.test.mjs tests/book-library.test.mjs` → `# fail 0` (32 at planning time); `npm --prefix auth test` → pass.
- [ ] **Step 2: Book guard (default mode)** — `python3 scripts/check_book_untouched.py --base origin/Manan` → `"problems": []`. No engine compare is needed: no Rust file changes in this phase (Step 4 proves it).
- [ ] **Step 3: Gateway unchanged** — `cd Context/guru/webapp && CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="${VAULT_RUNTIME_ROOT:-$HOME/.cache/strategy-vault-webapp}/test-target" cargo test --locked --manifest-path gateway/Cargo.toml` → pass, same count as before this phase.
- [ ] **Step 4: Scope proof** — `git diff 9fe0f21 --stat` (or the phase's branch point) lists only the files in "File structure" plus this plan; `git diff 9fe0f21 --name-only | grep -E '\.rs$|frontend/|compose|Dockerfile|package(-lock)?\.json|001-app\.sql'` → no output; `git diff 9fe0f21 -U0 -- Context/guru/webapp/auth/src/server.mjs Context/guru/webapp/auth/src/bootstrap.mjs Context/guru/webapp/auth/src/contracts.mjs | grep '^-[^-]'` → no output (additions only).
- [ ] **Step 5: Upload check** — `python3 scripts/check_upload.py` → `"findings": []`.
- [ ] **Step 6: Manual gate item for the user — DB-backed test on an isolated stack.** Not run by an agent. On the user's own isolated dev stack (separate runtime dir, port, engine image tag and Compose project name as in `docs/DEVELOPMENT.md`; never the shared live `strategy-vault` project):
  ```bash
  docker compose -p strategy-vault-dev-<name> -f Context/guru/webapp/compose.yaml --env-file "$HOME/.cache/strategy-vault-dev-<name>/.env" up -d --build auth
  cd Context/guru/webapp
  VAULT_ENV_FILE="$HOME/.cache/strategy-vault-dev-<name>/.env" VAULT_COMPOSE_PROJECT=strategy-vault-dev-<name> node --test tests/lab-strategies.integration.test.mjs
  ```
  Expected: 8 subtests pass; `docker compose -p strategy-vault-dev-<name> … logs auth` shows `Application schema ready` after a second `restart auth` (migration re-runs cleanly); `… exec -T postgres psql -U vault -d vault -c '\d lab_strategy'` shows the table. Record the result in the PR description.
- [ ] **Step 7: Report** — commit any gate fixes, then draft the PR description (tests before/after, guard output, decisions 1–14, the manual gate result or "pending user run"). Do not push; the user pushes.
