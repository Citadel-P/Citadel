import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { apiOutputs, verifyApi } from '../../scripts/api-generation';

it('detects edited, missing and unexpected generated files, including new outputs, without replacing edits', () => {
  const root = mkdtempSync(path.join(os.tmpdir(), 'citadel-api-generation-'));
  const write = (name: string, value: string) => {
    mkdirSync(path.dirname(path.join(root, name)), { recursive: true });
    writeFileSync(path.join(root, name), value);
  };
  const output = 'src/api/generated/new-output.ts';
  const foundation = 'src/api/generated/foundation-api.ts';
  const generate = () => {
    for (const name of apiOutputs(root).keys()) rmSync(path.join(root, name));
    write(output, 'generated');
  };
  try {
    write(foundation, 'Rust owns this file');
    expect(() => verifyApi(root, generate)).toThrow(output);
    write(output, 'generated');
    expect(() => verifyApi(root, generate)).not.toThrow();
    write(output, 'edited');
    expect(() => verifyApi(root, generate)).toThrow(output);
    expect(readFileSync(path.join(root, output), 'utf8')).toBe('edited');
    write(output, 'generated');
    write('src/api/generated/unexpected.ts', 'unexpected');
    expect(() => verifyApi(root, generate)).toThrow('unexpected.ts');
    expect(readFileSync(path.join(root, foundation), 'utf8')).toBe('Rust owns this file');
    expect(() =>
      verifyApi(root, () => {
        generate();
        throw new Error('generator failed');
      }),
    ).toThrow('generator failed');
    expect(readFileSync(path.join(root, 'src/api/generated/unexpected.ts'), 'utf8')).toBe('unexpected');
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
