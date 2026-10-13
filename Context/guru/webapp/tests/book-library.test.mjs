import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, symlink, rm, stat, truncate } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { createBookLibrary, registerBookLibrary, byteRange } from '../auth/src/book-library.mjs';

const { Hono } = createRequire(new URL('../auth/package.json', import.meta.url))('hono');
const PDF = Buffer.from('%PDF-1.4\nlocal book bytes\n%%EOF\n');
const request = (headers = {}, method = 'GET') => new Request('http://vault.test/api/app/books/files/book_valid', { headers, method });
const digest = bytes => createHash('sha256').update(bytes).digest('hex');

async function fixture(t) {
  const dir = await mkdtemp(join(tmpdir(), 'vault-book-reader-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const booksDir = join(dir, 'Books'), author = join(booksDir, 'Author');
  await mkdir(author, { recursive: true });
  await writeFile(join(author, 'Book α.pdf'), PDF);
  await writeFile(join(author, 'Bad.pdf'), '<html>not a PDF</html>');
  await writeFile(join(author, 'not-pdf.txt'), PDF);
  await writeFile(join(dir, 'outside.pdf'), PDF);
  await mkdir(join(dir, 'outside-directory'));
  await writeFile(join(dir, 'outside-directory', 'other.pdf'), PDF);
  await symlink(join(dir, 'outside.pdf'), join(author, 'escape.pdf'));
  await symlink(join(dir, 'outside-directory'), join(booksDir, 'escape-directory'));
  await symlink(join(author, 'Book α.pdf'), join(author, 'inside.pdf'));
  const manifestPath = join(dir, 'index.json');
  const manifest = { version: 1, files: {
    book_valid: { path: 'Author/Book α.pdf' },
    book_bad: { path: 'Author/Bad.pdf' },
    book_text: { path: 'Author/not-pdf.txt' },
    book_escape: { path: 'Author/escape.pdf' },
    book_escape_parent: { path: 'escape-directory/other.pdf' },
    book_inside: { path: 'Author/inside.pdf' },
    book_traversal: { path: '../outside.pdf' },
    book_absolute: { path: join(dir, 'outside.pdf') },
    book_missing: { path: 'Author/Missing.pdf' },
    book_backslash: { path: 'Author\\Book α.pdf' },
  } };
  await writeFile(manifestPath, JSON.stringify(manifest));
  const library = await createBookLibrary({ booksDir, manifestPath });
  return { dir, booksDir, manifestPath, library };
}

test('only a manifest-listed PDF is streamed unchanged, with private response headers', async t => {
  const f = await fixture(t), path = join(f.booksDir, 'Author', 'Book α.pdf');
  const before = await stat(path), response = await f.library.respond('book_valid', request());
  assert.equal(response.status, 200);
  assert.equal(response.headers.get('content-type'), 'application/pdf');
  assert.equal(response.headers.get('cache-control'), 'private, no-store');
  assert.equal(response.headers.get('x-content-type-options'), 'nosniff');
  assert.equal(response.headers.get('content-length'), String(PDF.length));
  assert.match(response.headers.get('content-disposition'), /filename\*=UTF-8''Book%20%CE%B1\.pdf/);
  assert.equal(digest(Buffer.from(await response.arrayBuffer())), digest(PDF));
  assert.equal(digest(await readFile(path)), digest(PDF));
  assert.equal((await stat(path)).mtimeMs, before.mtimeMs);
});

test('original document formats are unchanged authenticated attachments, never inline HTML', async t => {
  const f=await fixture(t), files={};
  const inputs={doc:Buffer.from('d0cf11e0a1b11ae100010203','hex'),docx:Buffer.from('504b030400010203','hex'),epub:Buffer.from('504b030400010203','hex'),djvu:Buffer.from('AT&TFORM0000DJVU'),rtf:Buffer.from('{\\rtf1 original text}'),text:Buffer.from('Original source '+String.fromCharCode(160)+' document','latin1'),html:Buffer.from('<html>original source</html>'),bad:Buffer.from('MZ executable')};
  for(const [kind,bytes] of Object.entries(inputs)){
    const extension=['rtf','text','html','bad'].includes(kind)?'doc':kind;
    await writeFile(join(f.booksDir,'Author',`${kind}.${extension}`),bytes);
    files[kind]={path:`Author/${kind}.${extension}`};
  }
  await writeFile(f.manifestPath,JSON.stringify({version:1,files}));
  const library=await createBookLibrary(f);
  for(const kind of ['doc','docx','epub','djvu','rtf','text','html']){
    const response=await library.respond(kind,request());
    assert.equal(response.status,200,kind);
    assert.equal(response.headers.get('content-type'),'application/octet-stream');
    assert.match(response.headers.get('content-disposition'),/^attachment;/);
    assert.equal(response.headers.get('x-content-type-options'),'nosniff');
    assert.deepEqual(Buffer.from(await response.arrayBuffer()),inputs[kind]);
  }
  assert.equal((await library.respond('bad',request())).status,404);
});

test('unlisted IDs and traversal cannot address files, even if bytes exist', async t => {
  const { library } = await fixture(t);
  for (const id of ['unknown', '../outside', '/tmp/a', '..', '%2e%2e', 'a/b', 'a\\b', 'x\n', 'x'.repeat(97), 'book_traversal', 'book_absolute', 'book_backslash', 'book_missing']) {
    assert.equal((await library.respond(id, request())).status, 404, id);
  }
});

test('outside file and parent-directory symlinks are refused; inside symlinks remain confined', async t => {
  const { library } = await fixture(t);
  for (const id of ['book_escape', 'book_escape_parent']) assert.equal((await library.respond(id, request())).status, 404);
  const response = await library.respond('book_inside', request());
  assert.equal(response.status, 200);
  assert.deepEqual(Buffer.from(await response.arrayBuffer()), PDF);
});

test('filename extension and PDF byte signature must both be valid', async t => {
  const { library } = await fixture(t);
  for (const id of ['book_bad', 'book_text']) assert.equal((await library.respond(id, request())).status, 404);
});

test('single byte ranges, suffixes and HEAD return exact byte counts', async t => {
  const { library } = await fixture(t);
  for (const [range, start, end] of [['bytes=0-4', 0, 4], ['bytes=5-', 5, PDF.length - 1], ['bytes=-5', PDF.length - 5, PDF.length - 1], ['bytes=0-9999', 0, PDF.length - 1]]) {
    const response = await library.respond('book_valid', request({ Range: range }));
    assert.equal(response.status, 206);
    assert.equal(response.headers.get('content-range'), `bytes ${start}-${end}/${PDF.length}`);
    assert.equal(response.headers.get('content-length'), String(end - start + 1));
    assert.deepEqual(Buffer.from(await response.arrayBuffer()), PDF.subarray(start, end + 1));
  }
  const head = await library.respond('book_valid', request({ Range: 'bytes=0-4' }, 'HEAD'));
  assert.equal(head.status, 206); assert.equal(head.headers.get('content-length'), '5');
  assert.equal((await head.arrayBuffer()).byteLength, 0);
});

test('invalid, huge and multipart ranges fail without disclosing file paths', async t => {
  const { library } = await fixture(t);
  for (const range of ['bytes=', 'bytes=-0', 'bytes=99-100', 'bytes=10-4', 'items=0-4', 'bytes=0-4,8-9', 'bytes=9007199254740993-', 'bytes= 0-4']) {
    const response = await library.respond('book_valid', request({ Range: range }));
    assert.equal(response.status, 416, range);
    assert.equal(response.headers.get('content-range'), `bytes */${PDF.length}`);
    assert.equal((await response.text()).includes('Author'), false);
  }
  assert.equal(byteRange('bytes=0-1', 0), null);
});

test('metadata returns only a local file reading link and does not certify source identity', async t => {
  const { library } = await fixture(t);
  const data = await (await library.respond('book_valid', request(), { metadata: true })).json();
  assert.deepEqual(data, { id: 'book_valid', filename: 'Book α.pdf', mediaType: 'application/pdf', bytes: PDF.length, url: '/api/app/books/files/book_valid', sourceIdentityVerified: false });
  assert.equal(JSON.stringify(data).includes('/tmp/'), false);
});

test('disabled, unavailable and malformed manifests leave the reader unavailable', async t => {
  assert.equal((await (await createBookLibrary({ booksDir: '', manifestPath: '' })).respond('book_valid', request())).status, 503);
  const f = await fixture(t);
  for (const content of ['bad json', JSON.stringify({ version: 2, files: {} }), JSON.stringify({ version: 1, files: [] })]) {
    await writeFile(f.manifestPath, content);
    const lib = await createBookLibrary(f);
    assert.equal(lib.enabled, false); assert.equal((await lib.respond('book_valid', request())).status, 503);
  }
  await truncate(f.manifestPath, 8 * 1024 * 1024 + 1);
  assert.equal((await createBookLibrary(f)).enabled, false);
});

test('slow reads have a bounded concurrency limit, and cancelling returns the capacity', async t => {
  const f = await fixture(t);
  await writeFile(join(f.booksDir, 'Author', 'Book α.pdf'), Buffer.concat([PDF, Buffer.alloc(1024 * 1024)]));
  const responses = [];
  for (let i = 0; i < 8; i++) {
    const response = await f.library.respond('book_valid', request());
    assert.equal(response.status, 200);
    responses.push(response);
  }
  const busy = await f.library.respond('book_valid', request());
  assert.equal(busy.status, 429); assert.equal(busy.headers.get('retry-after'), '3');
  await Promise.all(responses.map(response => response.body.cancel()));
  let head;
  for (let i = 0; i < 20; i++) {
    head = await f.library.respond('book_valid', request({}, 'HEAD'));
    if (head.status === 200) break;
    await new Promise(resolve => setTimeout(resolve, 5));
  }
  assert.equal(head.status, 200);
});

test('PDF route denies anonymous and unknown-role users before reading; viewers can read', async t => {
  const { library } = await fixture(t);
  let calls = 0;
  const guarded = { respond: (...args) => { calls++; return library.respond(...args); } };
  const app = new Hono();
  app.use('*', async (c, next) => {
    c.header('Cache-Control', 'no-store');
    const role = c.req.header('x-test-role');
    if (role) c.set('user', { role });
    await next();
  });
  registerBookLibrary(app, guarded);
  for (const role of [undefined, 'unknown']) {
    const response = await app.request('/api/app/books/files/book_valid', { headers: role ? { 'x-test-role': role } : {} });
    assert.equal(response.status, 401);
  }
  assert.equal(calls, 0);
  for (const role of ['viewer', 'researcher', 'admin']) {
    const response = await app.request('/api/app/books/files/book_valid', { headers: { 'x-test-role': role } });
    assert.equal(response.status, 200);
    assert.equal(response.headers.get('cache-control'), 'private, no-store');
    assert.deepEqual(Buffer.from(await response.arrayBuffer()), PDF);
  }
  const metadata = await app.request('/api/app/books/files/book_valid/info', { headers: { 'x-test-role': 'viewer' } });
  assert.equal((await metadata.json()).sourceIdentityVerified, false);
  assert.equal((await library.respond('book_valid', request({}, 'POST'))).status, 405);
});
