// @ts-nocheck
import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const webRoot = join(here, '..');
const repoRoot = join(webRoot, '..');
const spec = join(repoRoot, 'target', 'openapi.json');
const generated = join(webRoot, '.tmp', 'schema-check.d.ts');
const committed = join(webRoot, 'src', 'lib', 'api', 'schema.d.ts');

function run(command, args, cwd) {
  const result = spawnSync(command, args, { cwd, encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed:\n${result.stderr || result.stdout}`);
  }
}

run('cargo', ['run', '-q', '-p', 'hq-server', '--', 'openapi', '--out', spec, '--check'], repoRoot);
mkdirSync(dirname(generated), { recursive: true });
run('node', [join(repoRoot, 'tools', 'gen-api', 'gen.mjs'), spec, '--out', generated], repoRoot);

if (readFileSync(generated, 'utf8') !== readFileSync(committed, 'utf8')) {
  throw new Error('generated schema.d.ts is stale; run the S06 generation command');
}
console.log('api:check ok');
