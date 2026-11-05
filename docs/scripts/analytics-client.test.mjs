import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createAnalytics } from '../src/components/analytics/client.ts';

function fixture(t, origin, environment, siteUrl = origin) {
  const requests = [];
  const storage = new Map();
  const previousEnvironment = process.env.NODE_ENV;
  const globals = {
    location: new URL(origin),
    navigator: {},
    localStorage: {
      getItem: (key) => storage.get(key) ?? null,
      setItem: (key, value) => storage.set(key, value),
    },
    window: { addEventListener() {}, removeEventListener() {} },
    document: { addEventListener() {}, removeEventListener() {} },
    fetch: async (...args) => {
      requests.push(args);
      throw new Error('No analytics request is allowed in this fixture');
    },
  };
  const previous = Object.fromEntries(Object.keys(globals).map((key) =>
    [key, Object.getOwnPropertyDescriptor(globalThis, key)]));
  process.env.NODE_ENV = environment;
  for (const [key, value] of Object.entries(globals)) {
    Object.defineProperty(globalThis, key, { configurable: true, writable: true, value });
  }
  const client = createAnalytics({ '/': 'Home' }, siteUrl);
  const stop = client.start();
  t.after(() => {
    stop?.();
    for (const [key, descriptor] of Object.entries(previous)) {
      if (descriptor) Object.defineProperty(globalThis, key, descriptor);
      else delete globalThis[key];
    }
    if (previousEnvironment === undefined) delete process.env.NODE_ENV;
    else process.env.NODE_ENV = previousEnvironment;
  });
  return { client, requests, storage };
}

for (const origin of ['http://localhost:3000', 'http://127.0.0.1:3000', 'http://[::1]:3000']) {
  test(`development consent preview never collects on ${origin}`, (t) => {
    const { client, requests, storage } = fixture(t, origin, 'development');
    assert.equal(client.snapshot().enabled, true);
    assert.equal(client.snapshot().preview, true);
    client.choose('accepted');
    assert.equal(client.snapshot().choice, 'accepted');
    assert.equal(client.snapshot().collecting, false);
    client.trackPage('/');
    client.searchCompleted(0);
    client.searchResultOpened('/');
    assert.equal(client.ratePage('/', true), false);
    client.edit(true);
    assert.equal(client.snapshot().editing, true);
    client.choose('declined');
    assert.equal(client.snapshot().choice, 'declined');
    assert.equal(client.snapshot().editing, false);
    assert.equal(storage.size, 1);
    assert.deepEqual(requests, []);
  });
}

for (const siteUrl of ['http://localhost:3000', 'https://docs.citadelplane.com']) {
  test(`production export remains disabled locally with build URL ${siteUrl}`, (t) => {
    const { client, requests } = fixture(t, 'http://localhost:3000', 'production', siteUrl);
    client.trackPage('/');
    assert.equal(client.snapshot().enabled, false);
    assert.equal(client.snapshot().preview, false);
    assert.equal(client.snapshot().collecting, false);
    assert.deepEqual(requests, []);
  });
}

test('development preview is not enabled on a non-loopback hostname', (t) => {
  const { client, requests } = fixture(t, 'http://localhost.example:3000', 'development');
  assert.equal(client.snapshot().enabled, false);
  assert.equal(client.snapshot().preview, false);
  assert.deepEqual(requests, []);
});
