import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

async function validate(text) {
  const directory = await mkdtemp(path.join(os.tmpdir(), 'citadel-links-'));
  try {
    await writeFile(path.join(directory, 'index.mdx'), text);
    return spawnSync(process.execPath, ['scripts/validate-links.mjs', directory], {
      encoding: 'utf8',
    });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

test('MDX navigation cards cannot point at missing pages', async () => {
  const result = await validate('<Card href="/docs/missing" />');
  assert.equal(result.status, 1);
  assert.match(result.stderr, /missing internal page/);
});

test('headings inside example code are not valid navigation anchors', async () => {
  const result = await validate('[Example](#example)\n\n```md\n## Example\n```');
  assert.equal(result.status, 1);
  assert.match(result.stderr, /missing heading/);
});

test('examples do not create false broken links and real duplicate headings resolve', async () => {
  const result = await validate(
    '## Start\n## Start\n[Second](#start-1)\n<Card href="/docs#start" />\n' +
    '```md\n[Example](/docs/missing)\n```',
  );
  assert.equal(result.status, 0, result.stderr);
});

test('nested Markdown links are reported as malformed', async () => {
  const result = await validate('[[Start](/docs)](/docs)');
  assert.equal(result.status, 1);
  assert.match(result.stderr, /malformed nested Markdown link/);
});
