import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { pipeline } from 'node:stream/promises';
import { createGzip } from 'node:zlib';

const root = path.dirname(fileURLToPath(import.meta.url));
const publicDir = path.join(root, 'public');
const cacheDir = path.join(root, 'runtime', 'share-cache');
fs.mkdirSync(cacheDir, { recursive: true });
const wasm = path.join(publicDir, 'stronghold_game_bg.wasm');
const compressedWasm = path.join(cacheDir, 'stronghold_game_bg.wasm.gz');
if (!fs.existsSync(compressedWasm) || fs.statSync(compressedWasm).mtimeMs < fs.statSync(wasm).mtimeMs) {
  await pipeline(fs.createReadStream(wasm), createGzip({ level: 6 }), fs.createWriteStream(compressedWasm));
}
const types = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8',
  '.wasm': 'application/wasm', '.json': 'application/json', '.png': 'image/png',
  '.jpg': 'image/jpeg', '.glb': 'model/gltf-binary', '.ktx2': 'image/ktx2' };
const server = http.createServer((req, res) => {
  if (!['GET', 'HEAD'].includes(req.method)) { res.writeHead(405).end(); return; }
  let pathname;
  try { pathname = decodeURIComponent(new URL(req.url, 'http://localhost').pathname); }
  catch { res.writeHead(400).end(); return; }
  const relative = pathname === '/' ? 'index.html' : pathname.slice(1);
  // Only the compiled browser game and assets are shared, never source/data files.
  if (!(relative === 'index.html' || relative === 'network.js' || relative === 'stronghold_game.js'
    || relative === 'stronghold_game_bg.wasm' || relative.startsWith('assets/'))) {
    res.writeHead(404).end(); return;
  }
  const file = path.resolve(publicDir, relative);
  if (!file.startsWith(publicDir + path.sep) || !fs.existsSync(file) || !fs.statSync(file).isFile()) {
    res.writeHead(404).end(); return;
  }
  res.setHeader('Cache-Control', 'no-store');
  res.setHeader('Content-Type', types[path.extname(file)] || 'application/octet-stream');
  if (relative === 'index.html') {
    const html = fs.readFileSync(file, 'utf8').replace('<head>', '<head>\n<meta name="stronghold-server" content="/">');
    res.setHeader('Content-Length', Buffer.byteLength(html));
    res.writeHead(200); res.end(req.method === 'HEAD' ? undefined : html); return;
  }
  const gzip = relative === 'stronghold_game_bg.wasm' && /\bgzip\b/.test(req.headers['accept-encoding'] || '');
  const source = gzip ? compressedWasm : file;
  if (relative === 'stronghold_game_bg.wasm') res.setHeader('Vary', 'Accept-Encoding');
  if (gzip) res.setHeader('Content-Encoding', 'gzip');
  res.setHeader('Content-Length', fs.statSync(source).size);
  res.writeHead(200);
  if (req.method === 'HEAD') res.end();
  else fs.createReadStream(source).on('error', () => res.destroy()).pipe(res);
});

server.on('upgrade', (req, socket, head) => {
  const url = new URL(req.url, 'http://localhost');
  // Expose only the game protocol, not database administration/SQL/publishing.
  if (url.pathname !== '/v1/database/stronghold-local/subscribe') {
    socket.end('HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n'); return;
  }
  const upstream = http.request({ host: '127.0.0.1', port: 18766,
    path: req.url, method: 'GET', headers: { ...req.headers, host: '127.0.0.1:18766' } });
  upstream.on('upgrade', (response, upstreamSocket, upstreamHead) => {
    socket.write(`HTTP/1.1 ${response.statusCode} ${response.statusMessage}\r\n`);
    for (let i = 0; i < response.rawHeaders.length; i += 2)
      socket.write(`${response.rawHeaders[i]}: ${response.rawHeaders[i + 1]}\r\n`);
    socket.write('\r\n');
    if (upstreamHead.length) socket.write(upstreamHead);
    if (head.length) upstreamSocket.write(head);
    socket.on('error', () => upstreamSocket.destroy());
    upstreamSocket.on('error', () => socket.destroy());
    socket.on('close', () => upstreamSocket.destroy());
    upstreamSocket.on('close', () => socket.destroy());
    socket.pipe(upstreamSocket).pipe(socket);
  });
  upstream.on('response', (response) => {
    socket.end(`HTTP/1.1 ${response.statusCode} Upstream Error\r\nConnection: close\r\n\r\n`);
    response.resume();
  });
  upstream.on('error', () => socket.destroy());
  socket.on('error', () => upstream.destroy());
  upstream.end();
});
server.listen(18767, '127.0.0.1', () => console.log('Sharing gateway ready: http://127.0.0.1:18767/'));
