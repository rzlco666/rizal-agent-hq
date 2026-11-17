// @ts-nocheck
import { gzipSync } from 'node:zlib';
import { readFileSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..');
const build = join(root, 'build');
const limits = { entry: 350 * 1024, office: 300 * 1024 };
const forbidden = [
  /unpkg\.com/i,
  /jsdelivr\.net/i,
  /cdnjs\.cloudflare\.com/i,
  /fonts\.googleapis\.com/i,
  /fonts\.gstatic\.com/i,
  /\/\/cdn\./i
];
const remote = /https?:\/\/[^\s"')]+/gi;

const html = readFileSync(join(build, 'index.html'), 'utf8');
const assets = [
  ...html.matchAll(/(?:src|href)="(\/_app\/immutable\/[^"]+\.(?:js|css))"/g)
].map((match) => match[1]);
if (assets.length === 0) throw new Error('index.html has no immutable assets');

const sizes = { entry: 0, office: 0 };
for (const asset of assets) {
  const file = resolve(build, `.${asset}`);
  const bytes = readFileSync(file);
  const text = bytes.toString('utf8');
  for (const pattern of forbidden) {
    if (pattern.test(text)) throw new Error(`remote CDN reference in ${asset}: ${pattern}`);
  }
  for (const match of text.matchAll(remote)) {
    if (!match[0].startsWith('http://www.w3.org/') && !match[0].startsWith('https://svelte.dev/')) {
      throw new Error(`remote URL in ${asset}: ${match[0]}`);
    }
  }
  const gzip = gzipSync(bytes).length;
  sizes[asset.includes('/office') ? 'office' : 'entry'] += gzip;
}

console.log(`entry gzip: ${sizes.entry} bytes / ${limits.entry}`);
console.log(`office gzip: ${sizes.office} bytes / ${limits.office}`);
if (sizes.entry > limits.entry) throw new Error('initial bundle exceeds 350 KB gzip');
if (sizes.office > limits.office) throw new Error('office chunk exceeds 300 KB gzip');
if (!statSync(join(build, 'index.html')).isFile()) throw new Error('missing index.html');
console.log('bundle check ok');
