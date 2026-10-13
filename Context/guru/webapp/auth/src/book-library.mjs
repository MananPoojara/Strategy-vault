// Authenticated, read-only access to a generated allowlist of local PDF files.
// A filename match is a local reading aid, not certification of book identity or rules.
import { constants } from 'node:fs';
import { open, realpath } from 'node:fs/promises';
import { basename, extname, isAbsolute, relative, resolve, sep } from 'node:path';
import { Readable } from 'node:stream';
import { roleAllows } from './contracts.mjs';

const ID = /^[A-Za-z0-9_-]{1,96}$/;
const MAX_MANIFEST_BYTES = 8 * 1024 * 1024;
const MAX_FILES = 20000;
const MAX_OPEN_READS = 8;
const PRIVATE_HEADERS = {
  'Cache-Control': 'private, no-store',
  'X-Content-Type-Options': 'nosniff',
};

function inside(root, target) {
  const rel = relative(root, target);
  return rel !== '' && rel !== '..' && !rel.startsWith(`..${sep}`) && !isAbsolute(rel);
}

function safeRelative(value) {
  return typeof value === 'string' && value.length <= 2048 && value.length > 0
    && !isAbsolute(value) && !/[\\\x00-\x1f\x7f]/.test(value)
    && value.split('/').every(part => part !== '' && part !== '.' && part !== '..')
    && /\.(pdf|doc|docx|djvu|epub)$/i.test(value);
}

function validHeader(extension,signature){
  if(extension==='.pdf')return signature.subarray(0,5).toString('ascii')==='%PDF-';
  if(extension==='.doc'){
    if(signature.subarray(0,8).equals(Buffer.from('d0cf11e0a1b11ae1','hex'))||signature.subarray(0,5).toString('ascii')==='{\\rtf')return true;
    if(!signature.length||signature.subarray(0,2).toString('ascii')==='MZ'||signature.subarray(0,4).equals(Buffer.from('7f454c46','hex'))||signature.subarray(0,2).toString('ascii')==='#!')return false;
    if(signature.some(byte=>(byte<32&&![9,10,11,12,13].includes(byte))||byte===127))return false;
    let decoded;
    try{decoded=new TextDecoder('utf-8',{fatal:true}).decode(signature);}
    catch{
      if(signature.some(byte=>[0x81,0x8d,0x8f,0x90,0x9d].includes(byte)))return false;
      decoded=new TextDecoder('windows-1252',{fatal:true}).decode(signature);
    }
    return decoded.trim().length>0;
  }
  if(extension==='.docx'||extension==='.epub')return signature.subarray(0,4).equals(Buffer.from('504b0304','hex'));
  return extension==='.djvu'&&signature.subarray(0,8).toString('ascii')==='AT&TFORM';
}

function error(status, message, extra = {}) {
  return new Response(JSON.stringify({ error: message }), {
    status, headers: { ...PRIVATE_HEADERS, 'Content-Type': 'application/json; charset=utf-8', ...extra },
  });
}

// Accept one byte range only. Multiple ranges require multipart bodies and are rejected.
export function byteRange(value, size) {
  if (value === null || value === undefined) return { start: 0, end: size - 1, partial: false };
  if (typeof value !== 'string' || value.length > 100) return null;
  const match = /^bytes=(\d*)-(\d*)$/.exec(value);
  if (!match || (!match[1] && !match[2]) || size < 1) return null;
  let start, end;
  if (!match[1]) {
    const suffix = Number(match[2]);
    if (!Number.isSafeInteger(suffix) || suffix <= 0) return null;
    start = Math.max(0, size - suffix); end = size - 1;
  } else {
    start = Number(match[1]); end = match[2] ? Number(match[2]) : size - 1;
    if (!Number.isSafeInteger(start) || !Number.isSafeInteger(end) || start >= size || end < start) return null;
    end = Math.min(end, size - 1);
  }
  return { start, end, partial: true };
}

function disposition(filename,pdf) {
  const encoded = encodeURIComponent(filename).replace(/[!'()*]/g, c => `%${c.charCodeAt(0).toString(16).toUpperCase()}`);
  return `${pdf?'inline':'attachment'}; filename="book${extname(filename).toLowerCase()}"; filename*=UTF-8''${encoded}`;
}

export async function createBookLibrary({ booksDir = process.env.VAULT_BOOKS_DIR, manifestPath = process.env.VAULT_BOOK_MANIFEST } = {}) {
  let root, files = new Map(), activeReads = 0;
  if (booksDir && manifestPath) {
    let manifest;
    try {
      root = await realpath(booksDir);
      manifest = await open(manifestPath, constants.O_RDONLY | constants.O_NOFOLLOW);
      const stat = await manifest.stat();
      if (!stat.isFile() || stat.size < 1 || stat.size > MAX_MANIFEST_BYTES) throw new Error('Invalid manifest');
      const data = Buffer.alloc(stat.size + 1);
      let count = 0;
      while (count < data.length) {
        const got = await manifest.read(data, count, data.length - count, count);
        if (!got.bytesRead) break;
        count += got.bytesRead;
      }
      if (count !== stat.size) throw new Error('Manifest changed');
      const decoded = JSON.parse(data.subarray(0, count).toString('utf8'));
      if (decoded.version !== 1 || !decoded.files || typeof decoded.files !== 'object' || Array.isArray(decoded.files)) throw new Error('Invalid manifest');
      const entries = Object.entries(decoded.files);
      if (entries.length > MAX_FILES) throw new Error('Invalid manifest');
      for (const [id, item] of entries) {
        if (!ID.test(id) || !item || !safeRelative(item.path)) continue;
        const filename = basename(item.path);
        if (filename.length > 255 || filename.startsWith('~$')) continue;
        files.set(id, { path: item.path, filename });
      }
    } catch {
      root = undefined; files = new Map();
    } finally { await manifest?.close(); }
  }

  return {
    enabled: Boolean(root),
    async respond(id, request, { metadata = false } = {}) {
      if (!['GET', 'HEAD'].includes(request.method)) return error(405, 'Read-only library.', { Allow: 'GET, HEAD' });
      if (!root) return error(503, 'Local book reading is unavailable.');
      if (!ID.test(id || '') || !files.has(id)) return error(404, 'Book file unavailable.');
      if (activeReads >= MAX_OPEN_READS) return error(429, 'The library is busy. Please try again.', { 'Retry-After': '3' });
      activeReads++;
      let released = false, streamOwnsRead = false;
      const release = () => { if (!released) { released = true; activeReads--; } };
      const item = files.get(id);
      let handle;
      try {
        const candidate = await realpath(resolve(root, item.path));
        if (!inside(root, candidate) || !safeRelative(relative(root,candidate))) return error(404, 'Book file unavailable.');
        handle = await open(candidate, constants.O_RDONLY | constants.O_NOFOLLOW);
        // Check the opened descriptor, not only the pathname checked before open.
        // This service runs on Linux; /proc prevents a changed symlink/parent from escaping.
        const opened = await realpath(`/proc/self/fd/${handle.fd}`);
        if (!inside(root, opened) || opened !== candidate) return error(404, 'Book file unavailable.');
        const stat = await handle.stat();
        if (!stat.isFile() || stat.size < 5 || !Number.isSafeInteger(stat.size)) return error(404, 'Book file unavailable.');
        const signature = Buffer.alloc(Math.min(extname(candidate).toLowerCase()==='.doc'?4096:16,stat.size));
        const got = await handle.read(signature, 0, signature.length, 0);
        const extension=extname(candidate).toLowerCase(),pdf=extension==='.pdf';
        if(got.bytesRead!==signature.length||!validHeader(extension,signature))return error(404,'Book file unavailable.');
        const mediaType=pdf?'application/pdf':'application/octet-stream';
        if (metadata) return new Response(request.method === 'HEAD' ? null : JSON.stringify({
          id, filename: item.filename, mediaType, bytes: stat.size,
          url: `/api/app/books/files/${id}`, sourceIdentityVerified: false,
        }), { headers: { ...PRIVATE_HEADERS, 'Content-Type': 'application/json; charset=utf-8' } });
        const range = byteRange(request.headers.get('range'), stat.size);
        if (!range) return error(416, 'Requested byte range is unavailable.', { 'Content-Range': `bytes */${stat.size}` });
        const headers = {
          ...PRIVATE_HEADERS, 'Content-Type': mediaType, 'Accept-Ranges': 'bytes',
          'Content-Length': String(range.end - range.start + 1),
          'Content-Disposition': disposition(item.filename,pdf),
          'Content-Security-Policy': "default-src 'none'; frame-ancestors 'self'",
        };
        if (range.partial) headers['Content-Range'] = `bytes ${range.start}-${range.end}/${stat.size}`;
        if (request.method === 'HEAD') return new Response(null, { status: range.partial ? 206 : 200, headers });
        const stream = handle.createReadStream({ start: range.start, end: range.end, autoClose: true, highWaterMark: 64 * 1024 });
        handle = undefined; // The stream owns the descriptor and closes on finish/cancel.
        streamOwnsRead = true;
        stream.once('close', release);
        const abort = () => stream.destroy();
        request.signal.addEventListener('abort', abort, { once: true });
        stream.once('close', () => request.signal.removeEventListener('abort', abort));
        if (request.signal.aborted) abort();
        return new Response(Readable.toWeb(stream, {
          strategy: { highWaterMark: 64 * 1024, size: chunk => chunk.byteLength },
        }), { status: range.partial ? 206 : 200, headers });
      } catch {
        return error(404, 'Book file unavailable.');
      } finally {
        await handle?.close();
        if (!streamOwnsRead) release();
      }
    },
  };
}

// Register after the existing /api/app/* session middleware. Defense in depth:
// an accidentally reordered hook still cannot disclose files without a read role.
export function registerBookLibrary(app, library) {
  const serve = metadata => async c => {
    const user = c.get('user');
    if (!user || !roleAllows(user.role, 'read')) return c.json({ error: 'Please sign in.' }, 401);
    return library.respond(c.req.param('id'), c.req.raw, { metadata });
  };
  app.get('/api/app/books/files/:id/info', serve(true));
  app.get('/api/app/books/files/:id', serve(false));
}
