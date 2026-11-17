// @ts-nocheck
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
export const webRoot = join(here, '..');

const catalogNames = ['id', 'en', 'zh'];
const catalogs = Object.fromEntries(
  catalogNames.map((locale) => [
    locale,
    JSON.parse(readFileSync(join(webRoot, 'src', 'lib', 'i18n', `${locale}.json`), 'utf8'))
  ])
);

export function localeKeyIssues() {
  const base = Object.keys(catalogs.id).sort();
  return catalogNames.slice(1).flatMap((locale) => {
    const keys = Object.keys(catalogs[locale]).sort();
    return [
      ...keys.filter((key) => !base.includes(key)).map((key) => `${locale}: extra ${key}`),
      ...base.filter((key) => !keys.includes(key)).map((key) => `${locale}: missing ${key}`)
    ];
  });
}

const forbidden = [
  'min ago',
  'h ago',
  'menit lalu',
  'jam lalu',
  '分钟前',
  '小时前',
  'Tabel',
  'Sebelumnya',
  'Berikutnya',
  'Halaman'
];

function sourceFiles(directory) {
  return readdirSync(directory).flatMap((entry) => {
    const path = join(directory, entry);
    if (statSync(path).isDirectory()) return sourceFiles(path);
    if (!/\.(svelte|ts)$/.test(entry)) return [];
    if (path.endsWith(join('lib', 'api', 'schema.d.ts'))) return [];
    return [path];
  });
}

export function findHardcodedCopy(root = join(webRoot, 'src')) {
  const issues = [];
  for (const file of sourceFiles(root)) {
    const text = readFileSync(file, 'utf8');
    for (const phrase of forbidden) {
      if (text.includes(phrase)) issues.push(`${relative(webRoot, file)}: ${phrase}`);
    }
  }
  return issues;
}

const issues = [...localeKeyIssues(), ...findHardcodedCopy()];
if (issues.length > 0) {
  console.error(`i18n check failed:\n${issues.join('\n')}`);
  process.exit(1);
}
console.log(`i18n check ok (${catalogNames.length} locales)`);
