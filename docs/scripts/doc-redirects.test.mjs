import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { redirectDestination } from '../src/lib/doc-redirects.mjs';

const redirects = JSON.parse(await readFile(new URL('../redirects.json', import.meta.url), 'utf8'));

test('old resource links retain their heading and query on a project site', () => {
  assert.equal(
    redirectDestination(redirects['/docs/guides/git-stacks'], '#rollback', '?from=bookmark', '/Citadel'),
    '/Citadel/docs/resources/stacks/git/?from=bookmark#rollback',
  );
});

test('old Swarm and Build Pool headings resolve to their split resource pages', () => {
  assert.equal(
    redirectDestination(redirects['/docs/concepts/docker-swarm'], '#manage-a-swarm-service'),
    '/docs/resources/swarm-services/#manage-a-swarm-service',
  );
  assert.equal(
    redirectDestination(redirects['/docs/concepts/docker-swarm'], '#create-and-deploy-a-swarm-stack'),
    '/docs/resources/stacks/swarm/#create-and-deploy-a-swarm-stack',
  );
  assert.equal(
    redirectDestination(redirects['/docs/guides/builds'], '#build-pools', '', '/Citadel'),
    '/Citadel/docs/resources/build-pools/#configure-a-pool',
  );
});

test('old concept links go to Guides and the old category goes to Resources', () => {
  assert.equal(
    redirectDestination(redirects['/docs/concepts/access-control']),
    '/docs/guides/access-control/',
  );
  assert.equal(redirectDestination(redirects['/docs/concepts']), '/docs/resources/');
});
