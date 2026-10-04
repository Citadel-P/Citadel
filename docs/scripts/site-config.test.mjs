import assert from 'node:assert/strict';
import test from 'node:test';
import { getDocsBasePath } from '../site-config.mjs';

test('root and local deployments do not prefix assets', () => {
  assert.equal(getDocsBasePath('https://docs.example.org/'), '');
  assert.equal(getDocsBasePath('http://localhost:3000'), '');
});

test('project Pages and nested deployments retain their complete path', () => {
  assert.equal(getDocsBasePath('https://citadel-p.github.io/Citadel/'), '/Citadel');
  assert.equal(getDocsBasePath('https://example.org/products/citadel///'), '/products/citadel');
});

test('invalid publication addresses fail before export', () => {
  assert.throws(() => getDocsBasePath('docs.example.org'));
  assert.throws(() => getDocsBasePath('file:///tmp/docs'), /HTTP or HTTPS/);
});
