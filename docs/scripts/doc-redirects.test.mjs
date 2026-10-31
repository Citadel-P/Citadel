import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { redirectDestination, legacyDocRedirects } from '../src/lib/doc-redirects.mjs';

const redirects = JSON.parse(await readFile(new URL('../redirects.json', import.meta.url), 'utf8'));

test('old resource links retain their heading and query on a project site', () => {
  assert.equal(
    redirectDestination(redirects['/docs/guides/git-stacks'], '#rollback', '?from=bookmark', '/Citadel'),
    '/Citadel/resources/stacks/git/?from=bookmark#rollback',
  );
});

test('old Swarm and Build Pool headings resolve to their split resource pages', () => {
  assert.equal(
    redirectDestination(redirects['/docs/concepts/docker-swarm'], '#manage-a-swarm-service'),
    '/resources/swarm-services/#manage-a-swarm-service',
  );
  assert.equal(
    redirectDestination(redirects['/docs/concepts/docker-swarm'], '#create-and-deploy-a-swarm-stack'),
    '/resources/stacks/swarm/#create-and-deploy-a-swarm-stack',
  );
  assert.equal(
    redirectDestination(redirects['/docs/guides/builds'], '#build-pools', '', '/Citadel'),
    '/Citadel/resources/build-pools/#configure-a-pool',
  );
});

test('old concept links go to Guides and the old category goes to Resources', () => {
  assert.equal(
    redirectDestination(redirects['/docs/concepts/access-control']),
    '/guides/access-control/',
  );
  assert.equal(redirectDestination(redirects['/docs/concepts']), '/resources/');
});


test('every existing docs page redirects to the root route with query and fragment intact', () => {
  const rules = legacyDocRedirects(['/', '/getting-started/install'], redirects);
  assert.equal(redirectDestination(rules['/docs'], '', '', '/Citadel'), '/Citadel/');
  assert.equal(redirectDestination(rules['/docs/getting-started/install'], '#network-access', '?from=old'),
    '/getting-started/install/?from=old#network-access');
  assert.equal(redirectDestination(rules['/docs/guides/builds'], '#build-pools'),
    '/resources/build-pools/#configure-a-pool');
});
