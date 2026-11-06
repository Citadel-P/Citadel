import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createAnalytics } from '../src/components/analytics/client.ts';

function fixture(t, origin, environment, siteUrl = origin, options = {}) {
  const requests = [];
  const storage = new Map();
  const listeners = new Map();
  if (options.saved) storage.set('citadel.docs.analytics-consent.v1', JSON.stringify(options.saved));
  const previousEnvironment = process.env.NODE_ENV;
  const globals = {
    location: new URL(origin),
    navigator: {},
    localStorage: {
      getItem: (key) => storage.get(key) ?? null,
      setItem: (key, value) => storage.set(key, value),
    },
    window: { addEventListener: (name, listener) => listeners.set(name, listener), removeEventListener() {} },
    document: { addEventListener() {}, removeEventListener() {} },
    fetch: async (...args) => {
      requests.push(args);
      if (options.collect) {
        if (args[0].endsWith('/config.json')) return {
          ok: true, headers: new Headers({ 'content-type': 'application/json' }),
          json: async () => ({ enabled: true, hostname: 'docs.citadelplane.com', websiteId: '09ec38ac-910c-4fca-b346-0403a879b90c' }),
        };
        if (options.holdSends) return new Promise((_, reject) => args[1].signal.addEventListener('abort', () => reject(new Error('aborted'))));
        return { ok: true };
      }
      throw new Error('No analytics request is allowed in this fixture');
    },
  };
  const previous = Object.fromEntries(Object.keys(globals).map((key) =>
    [key, Object.getOwnPropertyDescriptor(globalThis, key)]));
  process.env.NODE_ENV = environment;
  for (const [key, value] of Object.entries(globals)) {
    Object.defineProperty(globalThis, key, { configurable: true, writable: true, value });
  }
  const client = createAnalytics(options.routes ?? { '/': 'Home' }, siteUrl);
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
  return { client, requests, storage, listeners };
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

const key = 'citadel.docs.analytics-consent.v1';
const origin = 'https://docs.citadelplane.com';
async function production(t, options = {}) {
  const result = fixture(t, origin, 'production', origin, {
    collect: true, routes: { '/': 'Home', '/privacy': 'Privacy', '/reference/api': 'API',
      '/getting-started/install': 'Install', '/getting-started/quick-start': 'Quick start' }, ...options,
  });
  await new Promise((resolve) => setImmediate(resolve));
  return { ...result, sends: () => result.requests.filter(([url]) => url.endsWith('/api/send')),
    payloads: () => result.requests.filter(([url]) => url.endsWith('/api/send')).map(([, init]) => JSON.parse(init.body).payload),
    visit(path) { location.href = origin + path; result.client.trackPage(location.pathname); } };
}

for (const choice of ['basic', 'accepted', 'declined', null]) {
  test(`${choice}: exact public payloads and allowed interaction scope`, async (t) => {
    const { client, visit, payloads, sends } = await production(t);
    if (choice) client.choose(choice);
    visit('/getting-started/install/?private=secret#bearer');
    client.installCommandCopied('/getting-started/install', 'download');
    client.searchCompleted(2000);
    client.searchCompleted(0);
    client.searchResultOpened('/reference/api?secret=value#fragment');
    client.ratePage('/getting-started/install', true);
    client.ratePage('/getting-started/install', false);
    visit('/reference/api');
    client.openApiDownloaded();
    const events = payloads().filter((p) => p.name);
    if (!choice || choice === 'declined') return assert.deepEqual(payloads(), []);
    const expected = ['install_command_copied', 'page_helpful', 'openapi_download_requested'];
    if (choice === 'accepted') expected.push('search_used', 'search_used', 'search_no_results', 'search_result_opened');
    assert.deepEqual(events.map((p) => p.name).sort(), expected.sort());
    if (choice === 'basic') assert(payloads().every((p) => !Object.hasOwn(p, 'data')));
    else {
      assert.deepEqual(events.find((p) => p.name === 'install_command_copied').data, { step: 'download' });
      assert.deepEqual(events.find((p) => p.name === 'search_used').data, { result_count: 1000 });
      assert.deepEqual(events.find((p) => p.name === 'search_result_opened').data, { selected_page: '/reference/api' });
    }
    assert(!/secret|bearer|fragment|private/.test(JSON.stringify(payloads())));
    for (const [url, init] of sends()) {
      assert.equal(url, 'https://analytics.citadelplane.com/docs/api/send');
      assert.equal(init.credentials, 'omit');
      assert.equal(init.referrerPolicy, 'no-referrer');
      assert.deepEqual(Object.keys(init.headers), ['Content-Type']);
    }
  });
}

for (const choice of ['basic', 'accepted']) {
  test(`${choice}: route and input guards reject excluded interactions`, async (t) => {
    const { client, visit, payloads } = await production(t);
    client.choose(choice);
    visit('/unknown-private');
    client.searchCompleted(0);
    client.searchResultOpened('/');
    client.ratePage('/unknown-private', true);
    client.installCommandCopied('/getting-started/install', 'download');
    client.openApiDownloaded();
    assert.deepEqual(payloads(), []);
    visit('/');
    client.searchCompleted(-1);
    client.searchCompleted(NaN);
    client.searchCompleted(0.5);
    client.searchResultOpened('https://private.example/');
    client.searchResultOpened('/unknown-private');
    client.ratePage('/', 'private-text');
    client.installCommandCopied('/', 'download');
    client.openApiDownloaded();
    visit('/getting-started/install');
    client.installCommandCopied('/getting-started/install', 'private-command');
    assert.equal(payloads().filter((p) => p.name).length, 0);
    visit('/privacy');
    const count = payloads().length;
    assert.equal(client.canCollectPage('/privacy'), choice === 'accepted');
    client.ratePage('/privacy', false);
    assert.equal(payloads().length, count + (choice === 'accepted' ? 1 : 0));
    assert.equal(payloads().some((p) => p.url === '/privacy'), choice === 'accepted');
  });

  for (const signal of ['doNotTrack', 'globalPrivacyControl']) {
    test(`${choice}: ${signal} applies consent precedence to pageviews and events`, async (t) => {
      const { client, visit, payloads } = await production(t);
      navigator.globalPrivacyControl = true;
      navigator[signal] = signal === 'doNotTrack' ? '1' : true;
      client.choose(choice);
      const allowed = choice === 'accepted' && signal === 'globalPrivacyControl';
      assert.equal(client.snapshot().collecting, allowed);
      visit('/getting-started/install');
      client.installCommandCopied('/getting-started/install', 'start');
      client.searchCompleted(0);
      assert.equal(client.ratePage('/getting-started/install', true), allowed);
      assert.deepEqual(payloads().map((p) => p.name ?? p.url), allowed
        ? ['/getting-started/install', 'install_command_copied', 'search_used', 'search_no_results', 'page_helpful']
        : []);
    });
  }

  test(`${choice}: active expiry cancels delivery and requires a new choice`, async (t) => {
    const expires = Date.now() + 60_000;
    const { client, visit, sends } = await production(t, { saved: { choice, expires }, holdSends: true });
    visit('/');
    assert.equal(sends().length, 1);
    t.mock.method(Date, 'now', () => expires + 1);
    client.searchCompleted(0);
    assert.equal(client.snapshot().choice, null);
    assert.equal(client.snapshot().collecting, false);
    assert(sends()[0][1].signal.aborted);
    visit('/reference/api');
    client.openApiDownloaded();
    assert.equal(sends().length, 1);
  });
}

test('GPC override requires valid Full consent and ends on cross-tab downgrade or expiry', async (t) => {
  const { client, visit, sends, storage, listeners } = await production(t, { holdSends: true });
  navigator.globalPrivacyControl = true;
  visit('/');
  assert.equal(sends().length, 0);
  const expires = Date.now() + 60_000;
  const change = (choice) => {
    storage.set(key, JSON.stringify({ choice, expires }));
    listeners.get('storage')({ key });
  };
  change('accepted');
  assert.equal(client.snapshot().collecting, true);
  visit('/');
  assert.equal(sends().length, 1);
  for (const choice of ['basic', 'declined']) {
    change(choice);
    assert.equal(client.snapshot().collecting, false);
    assert(sends().every(([, init]) => init.signal.aborted));
    visit('/reference/api');
    client.openApiDownloaded();
    assert.equal(sends().length, 1);
  }
  change('accepted');
  visit('/');
  assert.equal(sends().length, 2);
  t.mock.method(Date, 'now', () => expires + 1);
  client.searchCompleted(0);
  assert.equal(client.snapshot().choice, null);
  assert.equal(client.snapshot().collecting, false);
  assert(sends().every(([, init]) => init.signal.aborted));
  visit('/reference/api');
  assert.equal(sends().length, 2);
});

test('mode and cross-tab changes cancel pending requests without duplicate navigation views', async (t) => {
  const { client, visit, payloads, sends, storage, listeners } = await production(t, { holdSends: true });
  const views = () => payloads().filter((p) => !p.name).map((p) => p.url);
  client.choose('basic');
  visit('/');
  client.choose('accepted');
  client.trackPage('/');
  assert(sends()[0][1].signal.aborted);
  client.edit(true); client.edit(false);
  storage.set(key, JSON.stringify({ choice: 'basic', expires: Date.now() + 60_000 }));
  listeners.get('storage')({ key });
  client.trackPage('/');
  assert.deepEqual(views(), ['/']);
  visit('/privacy'); // Excluded navigation must break the page deduplication boundary.
  visit('/');
  client.trackPage('/');
  assert.deepEqual(views(), ['/', '/']);
  client.choose('accepted');
  visit('/privacy');
  client.choose('basic'); client.trackPage('/privacy');
  client.choose('accepted'); client.trackPage('/privacy');
  assert.deepEqual(views(), ['/', '/', '/privacy']);
  storage.clear(); listeners.get('storage')({ key: null });
  assert.equal(client.snapshot().choice, null);
  assert(sends().every(([, init]) => init.signal.aborted));
  visit('/');
  client.choose('declined');
  client.trackPage('/');
  assert.equal(views().length, 3);
  client.choose('basic'); client.trackPage('/');
  assert.equal(views().length, 4);
});

for (const saved of [
  { choice: 'declined', expires: Date.now() + 60_000 },
  { choice: 'basic', expires: Date.now() - 1 },
  { choice: 'accepted', expires: Date.now() - 1 },
  { choice: 'unknown', expires: Date.now() + 60_000 },
  { choice: 'basic', expires: Date.now() + 181 * 86400000 },
]) {
  test(`stored ${JSON.stringify(saved)} cannot authorize collection`, async (t) => {
    const { client, visit, payloads, storage } = await production(t, { saved });
    visit('/'); client.ratePage('/', true); client.searchCompleted(0);
    assert.deepEqual(payloads(), []);
    assert.deepEqual(JSON.parse(storage.get(key)), saved);
  });
}
