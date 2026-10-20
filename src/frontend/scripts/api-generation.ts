import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const generated = 'src/api/generated';
const rustOwned = `${generated}/foundation-api.ts`;
const schema = 'src/api/schema/swagger.json';

// npm owns this directory except xtask's foundation API. Directory discovery
// makes new, missing and unexpected generator outputs part of the same check.
export function apiOutputs(root: string): Map<string, Buffer> {
  const files = new Map<string, Buffer>();
  function visit(relative: string) {
    if (!existsSync(path.join(root, relative))) return;
    for (const entry of readdirSync(path.join(root, relative), { withFileTypes: true })) {
      const name = `${relative}/${entry.name}`;
      if (entry.isDirectory()) visit(name);
      else if (name !== rustOwned) files.set(name, readFileSync(path.join(root, name)));
    }
  }
  visit(generated);
  if (existsSync(path.join(root, schema))) files.set(schema, readFileSync(path.join(root, schema)));
  return files;
}

function clearOutputs(root: string) {
  for (const name of apiOutputs(root).keys()) rmSync(path.join(root, name));
}

export function verifyApi(root: string, generate: () => void) {
  const before = apiOutputs(root);
  try {
    generate();
    const after = apiOutputs(root);
    const changed = [...new Set([...before.keys(), ...after.keys()])].filter(
      (name) => !before.has(name) || !after.has(name) || !before.get(name)!.equals(after.get(name)!),
    );
    if (changed.length)
      throw new Error(
        `Generated API output drift:\n${changed.sort().join('\n')}\nRun npm run api:generate and review the changes.`,
      );
  } finally {
    // Verification never stages changes or replaces a developer's edits.
    clearOutputs(root);
    for (const [name, contents] of before) {
      mkdirSync(path.dirname(path.join(root, name)), { recursive: true });
      writeFileSync(path.join(root, name), contents);
    }
  }
}

function generateApi(root: string) {
  clearOutputs(root);
  for (const [command, ...args] of [
    ['tsx', 'scripts/sync-rust-schema.ts'],
    [
      'swagger-typescript-api',
      'generate',
      '--custom-config',
      './scripts/api-generator.config.mjs',
      '--path',
      './src/api/schema/swagger.json',
      '-o',
      './src/api/',
      '-n',
      'generated/api.types.ts',
      '--responses',
    ],
    ['tsx', 'scripts/generate-resources.ts'],
  ]) {
    const result = spawnSync(command, args, { cwd: root, stdio: 'inherit' });
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(`${command} failed (${result.status})`);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = fileURLToPath(new URL('..', import.meta.url));
  if (process.argv[2] === 'generate') generateApi(root);
  else if (process.argv[2] === 'verify') verifyApi(root, () => generateApi(root));
  else throw new Error('Expected generate or verify');
}
