import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gzipSync } from 'node:zlib';

const buildDir = fileURLToPath(new URL('../../build/', import.meta.url));
const html = readFileSync(join(buildDir, 'index.html'), 'utf8');
const assets = [
  ...html.matchAll(/(?:src|href)="(\/_app\/immutable\/[^"]+\.(?:js|css))"/g)
].map((match) => match[1]);

function gzipTotal(pattern: (asset: string) => boolean): number {
  return assets
    .filter(pattern)
    .reduce((total, asset) => total + gzipSync(readFileSync(join(buildDir, asset.slice(1)))).length, 0);
}

test('initial bundle is under 350kb gzip', () => {
  const bytes = gzipTotal((asset) => !asset.includes('/office'));
  console.log(`initial bundle gzip: ${bytes} bytes`);
  expect(bytes).toBeLessThan(350 * 1024);
});

test('office chunk is under 300kb gzip', () => {
  const bytes = gzipTotal((asset) => asset.includes('/office'));
  console.log(`office chunk gzip: ${bytes} bytes`);
  expect(bytes).toBeLessThan(300 * 1024);
});

test('no asset is loaded from a remote origin', () => {
  const forbidden = [
    /unpkg\.com/i,
    /jsdelivr\.net/i,
    /cdnjs\.cloudflare\.com/i,
    /fonts\.googleapis\.com/i,
    /fonts\.gstatic\.com/i,
    /\/\/cdn\./i
  ];
  for (const asset of assets) {
    const text = readFileSync(join(buildDir, asset.slice(1)), 'utf8');
    for (const pattern of forbidden) {
      expect(pattern.test(text), `remote origin in ${asset}: ${pattern}`).toBe(false);
    }
  }
});
