// Browser checks against a local static export. All analytics traffic is intercepted.
// Build with NEXT_PUBLIC_DOCS_URL=https://docs.citadelplane.com first.
import assert from 'node:assert/strict';
import { after, before, test } from 'node:test';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { chromium } from '../../test/e2e/node_modules/playwright/index.mjs';

const origin = 'https://docs.citadelplane.com';
const endpoint = 'https://analytics.citadelplane.com/docs';
const storageKey = 'citadel.docs.analytics-consent.v1';
const config = { enabled: true, hostname: 'docs.citadelplane.com', websiteId: '09ec38ac-910c-4fca-b346-0403a879b90c' };
const root = path.resolve('out');
let browser;
let server;
let local;

before(async () => {
  server = createServer(async (request, response) => {
    const url = new URL(request.url, 'http://localhost');
    const filename = path.resolve(root, `.${decodeURIComponent(url.pathname)}`,
      url.pathname.endsWith('/') ? 'index.html' : '');
    if (!filename.startsWith(root + '/')) { response.writeHead(403).end(); return; }
    try {
      const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css',
        '.json': 'application/json', '.svg': 'image/svg+xml', '.png': 'image/png', '.woff2': 'font/woff2' };
      response.setHeader('Content-Type', mime[path.extname(filename)] ?? 'application/octet-stream');
      response.end(await readFile(filename));
    } catch { response.writeHead(404).end(await readFile(path.join(root, '404.html'))); }
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  local = `http://127.0.0.1:${server.address().port}`;
  browser = await chromium.launch({ headless: true });
});
after(async () => {
  await browser?.close();
  if (server) await new Promise((resolve) => server.close(resolve));
});

async function fixture(t, { saved, signal, storageUnavailable, configValue = config, failConfig = false, failSend = false, viewport } = {}) {
  const context = await browser.newContext({ viewport });
  t.after(async () => {
    await context.unrouteAll({ behavior: 'ignoreErrors' });
    await context.close();
  });
  const sends = [];
  const configs = [];
  const errors = [];
  await context.route(`${origin}/**`, async (route) => {
    const pathname = decodeURIComponent(new URL(route.request().url()).pathname);
    const file = path.join(root, pathname, pathname.endsWith('/') ? 'index.html' : '');
    const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css',
      '.json': 'application/json', '.svg': 'image/svg+xml', '.png': 'image/png', '.woff2': 'font/woff2' };
    let body;
    let status = 200;
    try { body = readFileSync(file); } catch { body = readFileSync(path.join(root, '404.html')); status = 404; }
    await route.fulfill({ status, body, contentType: mime[path.extname(file)] ?? 'application/octet-stream' });
  });
  await context.route(`${endpoint}/**`, async (route) => {
    const request = route.request();
    const headers = { 'access-control-allow-origin': origin, 'access-control-allow-headers': 'content-type' };
    if (request.method() === 'OPTIONS') return route.fulfill({ status: 204, headers });
    if (request.url().endsWith('/config.json')) {
      configs.push(request);
      if (failConfig) return route.abort();
      return route.fulfill({ json: configValue, headers });
    }
    sends.push({ payload: request.postDataJSON(), headers: await request.allHeaders() });
    if (failSend) return route.abort();
    return route.fulfill({ json: {}, headers });
  });
  await context.addInitScript(({ saved, signal, storageUnavailable, storageKey }) => {
    if (saved) localStorage.setItem(storageKey, JSON.stringify(saved));
    if (signal === 'dnt') Object.defineProperty(navigator, 'doNotTrack', { value: '1' });
    if (signal === 'gpc') Object.defineProperty(navigator, 'globalPrivacyControl', { value: true });
    if (storageUnavailable) {
      Storage.prototype.getItem = Storage.prototype.setItem = () => { throw new Error('Storage disabled'); };
    }
  }, { saved, signal, storageUnavailable, storageKey });
  const page = await context.newPage();
  page.on('pageerror', (error) => errors.push(error.message));
  t.after(() => assert.deepEqual(errors, []));
  return { page, context, sends, configs };
}
const accepted = () => ({ choice: 'accepted', expires: Date.now() + 60_000 });
async function settle(page) { await page.waitForTimeout(150); }
async function until(predicate) {
  for (let i = 0; i < 100 && !predicate(); i++) await new Promise((resolve) => setTimeout(resolve, 30));
  assert(predicate());
}
async function open(page, route = '/') {
  await page.goto(origin + route);
  await page.getByRole('button', { name: 'Analytics preferences', exact: true }).waitFor();
}

async function savePreference(page, label) {
  const names = { 'Accept all': 'Full', 'Reject non-essentials': 'Basic' };
  const dialog = page.getByRole('dialog', { name: 'Analytics preferences', exact: true });
  await dialog.getByRole('radio', { name: names[label] ?? label, exact: true }).check();
  await dialog.getByRole('button', { name: 'Save preferences', exact: true }).click();
  await dialog.waitFor({ state: 'hidden' });
}

test('opt-in, clean payload, client navigation, hashes and withdrawal', async (t) => {
  const { page, sends, configs } = await fixture(t);
  await open(page, '/?secret=never-send#private');
  await settle(page);
  assert.equal(sends.length, 0);
  assert.equal(configs.length, 1);
  await page.getByRole('button', { name: 'Accept all', exact: true }).click();
  await until(() => sends.length === 1);
  assert.deepEqual(sends[0].payload, { type: 'event', payload: {
    website: config.websiteId, hostname: config.hostname, url: '/', title: 'What is Citadel?',
  } });
  assert.equal(sends[0].headers.cookie, undefined);
  assert.equal(sends[0].headers.referer, undefined);
  assert.equal(sends[0].headers.authorization, undefined);
  await page.evaluate(() => { location.hash = 'another-anchor'; });
  await settle(page);
  assert.equal(sends.length, 1);
  await page.locator('a[href="/getting-started/quick-start/"]').first().click();
  await until(() => sends.length === 2);
  assert.equal(sends[1].payload.payload.url, '/getting-started/quick-start');
  assert.equal(configs.length, 1, 'Navigation should keep the analytics provider mounted');
  await page.getByRole('button', { name: 'Analytics preferences', exact: true }).click();
  assert.equal(await page.locator(':focus').getAttribute('aria-labelledby'), 'docs-analytics-title');
  await savePreference(page, 'Off');
  assert.equal(await page.locator(':focus').textContent(), 'Analytics preferences');
  await page.goto(origin + '/resources/');
  await settle(page);
  assert.equal(sends.length, 2);
});

test('preferences are centered, trap focus and discard unsaved changes', async (t) => {
  const { page, sends } = await fixture(t, { viewport: { width: 390, height: 844 } });
  await page.goto(origin + '/');
  const trigger = page.getByRole('button', { name: 'Manage preferences', exact: true });
  const dialog = page.getByRole('dialog', { name: 'Analytics preferences', exact: true });
  await trigger.click();
  await dialog.waitFor();
  assert.equal(await page.getByRole('region', { name: 'Optional analytics' }).count(), 0);
  await page.screenshot({ path: '/tmp/citadel-docs-analytics-dialog-mobile.png' });
  const bounds = await dialog.boundingBox();
  // The docs reserve a scrollbar gutter, so center within the usable viewport.
  const width = await page.evaluate(() => document.documentElement.getBoundingClientRect().width);
  assert(Math.abs(bounds.x + bounds.width / 2 - width / 2) < 2);
  assert(Math.abs(bounds.y + bounds.height / 2 - 422) < 2);
  assert(bounds.y >= 16 && bounds.y + bounds.height <= 828);
  assert.equal(await dialog.getByRole('radio', { name: 'Basic', exact: true }).isChecked(), true);
  await dialog.getByRole('radio', { name: 'Full', exact: true }).check();
  assert.equal(await page.evaluate((key) => localStorage.getItem(key), storageKey), null);
  await dialog.getByRole('button', { name: 'Save preferences' }).focus();
  await page.keyboard.press('Tab');
  assert(await dialog.evaluate((element) => element.contains(document.activeElement)));
  await page.keyboard.press('Escape');
  await dialog.waitFor({ state: 'hidden' });
  assert(await trigger.evaluate((element) => element === document.activeElement));
  await trigger.click();
  assert.equal(await dialog.getByRole('radio', { name: 'Basic', exact: true }).isChecked(), true);
  await page.screenshot({ path: '/tmp/citadel-docs-analytics-dialog-mobile.png' });
  await dialog.getByRole('radio', { name: 'Off', exact: true }).check();
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  assert.equal(await page.evaluate((key) => localStorage.getItem(key), storageKey), null);
  assert.equal(sends.length, 0);
  await trigger.click();
  await dialog.getByRole('button', { name: 'Save preferences', exact: true }).click();
  await dialog.waitFor({ state: 'hidden' });
  await until(() => sends.length === 1);
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.getByRole('button', { name: 'Analytics preferences', exact: true }).click();
  assert.equal(await dialog.getByRole('radio', { name: 'Basic', exact: true }).isChecked(), true);
  await dialog.getByRole('radio', { name: 'Full', exact: true }).check();
  await page.keyboard.press('Escape');
  assert.equal(await page.evaluate((key) => JSON.parse(localStorage.getItem(key)).choice, storageKey), 'basic');
  assert.equal(sends.length, 1);
});

test('Off is remembered and can be changed', async (t) => {
  const { page, sends } = await fixture(t);
  await open(page);
  await page.getByRole('button', { name: 'Manage preferences', exact: true }).click();
  await savePreference(page, 'Off');
  await page.reload();
  await page.getByRole('button', { name: 'Analytics preferences', exact: true }).waitFor();
  assert.equal(await page.getByRole('region', { name: 'Optional analytics' }).count(), 0);
  assert.equal(sends.length, 0);
  await page.getByRole('button', { name: 'Analytics preferences', exact: true }).click();
  assert.equal(await page.getByRole('radio', { name: 'Off', exact: true }).isChecked(), true);
  await savePreference(page, 'Accept all');
  await until(() => sends.length === 1);
  await page.getByRole('button', { name: 'Analytics preferences', exact: true }).click();
  assert.equal(await page.getByRole('radio', { name: 'Full', exact: true }).isChecked(), true);
  await page.getByRole('button', { name: 'Close', exact: true }).click();
});

test('DNT blocks saved and new Full consent even with GPC', async (t) => {
  const { page, context, sends } = await fixture(t, { saved: accepted(), signal: 'dnt' });
  await context.addInitScript(() => Object.defineProperty(navigator, 'globalPrivacyControl', { value: true }));
  await open(page);
  await page.getByRole('button', { name: 'Analytics preferences', exact: true }).click();
  await savePreference(page, 'Accept all');
  await settle(page);
  assert.equal(sends.length, 0);
});

for (const choice of ['Basic', 'Full']) for (const entry of ['notice', 'preferences']) test(`GPC allows explicit ${choice} from ${entry} until Off`, async (t) => {
  const { page, sends } = await fixture(t, { signal: 'gpc' });
  await open(page);
  await page.getByRole('region', { name: 'Optional analytics' }).waitFor();
  assert.equal(sends.length, 0);
  assert.equal(await page.getByRole('region', { name: 'Page feedback' }).count(), 0);
  if (entry === 'notice') {
    await page.getByRole('button', { name: choice === 'Basic' ? 'Reject non-essentials' : 'Accept all', exact: true }).click();
  } else {
    await page.getByRole('button', { name: 'Manage preferences', exact: true }).click();
    await savePreference(page, choice);
  }
  await until(() => sends.length === 1);
  await page.getByRole('button', { name: 'Yes, this page was helpful', exact: true }).click();
  await until(() => sends.length === 2);
  assert.equal(sends[1].payload.payload.name, 'page_helpful');
  await page.reload();
  await until(() => sends.length === 3);
  await page.getByRole('button', { name: 'Analytics preferences', exact: true }).click();
  await savePreference(page, 'Basic');
  await page.getByRole('region', { name: 'Page feedback' }).waitFor();
  await page.reload();
  await until(() => sends.length === 4);
  await page.getByRole('button', { name: 'Yes, this page was helpful', exact: true }).click();
  await until(() => sends.length === 5);
  assert.equal(sends[4].payload.payload.data, undefined);
  await page.getByRole('button', { name: 'Analytics preferences', exact: true }).click();
  await savePreference(page, 'Off');
  assert.equal(await page.getByRole('region', { name: 'Page feedback' }).count(), 0);
  await page.reload();
  await page.getByRole('button', { name: 'Analytics preferences', exact: true }).waitFor();
  await settle(page);
  assert.equal(sends.length, 5);
});

test('expired or invalid consent cannot authorize collection', async (t) => {
  const { page, sends } = await fixture(t, { saved: { choice: 'accepted', expires: Date.now() - 1 } });
  await open(page);
  await page.getByRole('button', { name: 'Accept all', exact: true }).waitFor();
  assert.equal(sends.length, 0);
});

test('active consent expires and reopens the notice', async (t) => {
  const { page, sends } = await fixture(t, { saved: { choice: 'accepted', expires: Date.now() + 5000 } });
  await open(page);
  await until(() => sends.length === 1);
  await page.clock.install();
  await page.clock.fastForward(6000);
  await page.evaluate(() => document.dispatchEvent(new Event('visibilitychange')));
  await page.getByRole('button', { name: 'Accept all', exact: true }).waitFor();
  assert.equal(sends.length, 1);
});

test('withdrawal propagates to another open tab', async (t) => {
  const { page, context, sends } = await fixture(t);
  await open(page);
  await page.getByRole('button', { name: 'Accept all', exact: true }).click();
  await until(() => sends.length === 1);
  const second = await context.newPage();
  await open(second);
  await until(() => sends.length === 2);
  await page.getByRole('button', { name: 'Analytics preferences', exact: true }).click();
  await savePreference(page, 'Off');
  await settle(second);
  await second.locator('a[href="/getting-started/quick-start/"]').first().click();
  await settle(second);
  assert.equal(sends.length, 2);
});

test('unavailable storage permits only a page-session choice', async (t) => {
  const { page, sends } = await fixture(t, { storageUnavailable: true });
  await open(page);
  await page.getByRole('button', { name: 'Accept all', exact: true }).click();
  await until(() => sends.length === 1);
  await page.reload();
  await page.getByRole('button', { name: 'Accept all', exact: true }).waitFor();
  await settle(page);
  assert.equal(sends.length, 1);
});

for (const [name, options] of Object.entries({
  disabled: { configValue: { enabled: false } },
  invalid: { configValue: { ...config, websiteId: 'invalid' } },
  'wrong hostname': { configValue: { ...config, hostname: 'another.example' } },
  unavailable: { failConfig: true },
})) test(`${name} configuration fails closed`, async (t) => {
  const { page, sends, configs } = await fixture(t, options);
  await page.goto(origin + '/');
  await until(() => configs.length === 1);
  await settle(page);
  assert.equal(await page.getByRole('button', { name: 'Analytics preferences', exact: true }).count(), 0);
  assert.equal(sends.length, 0);
});

test('collector failure does not retry or break navigation', async (t) => {
  const { page, sends } = await fixture(t, { saved: accepted(), failSend: true });
  await open(page);
  await until(() => sends.length === 1);
  await settle(page);
  assert.equal(sends.length, 1);
  await page.locator('a[href="/getting-started/quick-start/"]').first().click();
  await until(() => sends.length === 2);
});

test('unknown pages and localhost never collect', async (t) => {
  const { page, sends, configs } = await fixture(t, { saved: accepted() });
  await page.goto(origin + '/unknown-private-value/');
  await until(() => configs.length === 1);
  await settle(page);
  assert.equal(sends.length, 0);
  await page.goto(local + '/');
  await settle(page);
  assert.equal(configs.length, 1);
  assert.equal(sends.length, 0);
});

test('API reference and mobile consent controls remain accessible', async (t) => {
  const { page, sends } = await fixture(t, { viewport: { width: 390, height: 844 } });
  await page.goto(origin + '/');
  const notice = page.getByRole('region', { name: 'Optional analytics' });
  await notice.waitFor();
  const bounds = await notice.boundingBox();
  assert(bounds.x >= 0 && bounds.x + bounds.width <= 390);
  const accept = page.getByRole('button', { name: 'Accept all', exact: true });
  const decline = page.getByRole('button', { name: 'Reject non-essentials', exact: true });
  const acceptBounds = await accept.boundingBox();
  const declineBounds = await decline.boundingBox();
  assert(Math.abs(acceptBounds.width - declineBounds.width) <= 1);
  assert(acceptBounds.height >= 40 && declineBounds.height >= 40);
  assert.notEqual(
    await accept.evaluate((button) => getComputedStyle(button).backgroundColor),
    await decline.evaluate((button) => getComputedStyle(button).backgroundColor),
  );
  await page.screenshot({ path: '/tmp/citadel-docs-analytics-mobile.png' });
  await page.getByRole('button', { name: 'Manage preferences', exact: true }).click();
  await savePreference(page, 'Off');
  await page.setViewportSize({ width: 1440, height: 1000 });
  await open(page, '/reference/api/');
  await page.getByRole('button', { name: 'Toggle Theme', exact: true }).click();
  await page.getByRole('button', { name: 'Analytics preferences', exact: true }).click();
  await page.getByRole('dialog', { name: 'Analytics preferences' }).waitFor();
  await page.screenshot({ path: '/tmp/citadel-docs-analytics-dark.png' });
  await page.setViewportSize({ width: 390, height: 844 });
  assert(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
  assert.equal(sends.length, 0);
});

const customEvents = (sends) => sends.filter((send) => send.payload.payload.name).map((send) => send.payload.payload);

test('basic choice is explicit, remembers reduced measurement and excludes search events', async (t) => {
  const { page, sends } = await fixture(t);
  await open(page, '/?private=secret#bearer');
  assert.equal(sends.length, 0);
  await page.getByRole('button', { name: 'Reject non-essentials', exact: true }).click();
  await until(() => sends.length === 1);
  assert.equal(await page.evaluate((key) => JSON.parse(localStorage.getItem(key)).choice, storageKey), 'basic');
  await searchFor(page, 'zzPrivateSearch_123459');
  await page.getByText('No results found', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Close Search', exact: true }).click();
  await settle(page);
  assert.deepEqual(customEvents(sends), []);
  await page.getByRole('button', { name: 'Yes, this page was helpful', exact: true }).click();
  await until(() => customEvents(sends).length === 1);
  assert.equal(customEvents(sends)[0].name, 'page_helpful');
  assert.equal(customEvents(sends)[0].data, undefined);
  await page.reload();
  await until(() => sends.filter((s) => !s.payload.payload.name).length === 2);
  assert.equal(await page.getByRole('region', { name: 'Optional analytics' }).count(), 0);
  assert(!/secret|bearer|zzPrivateSearch/.test(JSON.stringify(sends.map((s) => s.payload))));
});

test('basic/full changes and navigation do not duplicate pageviews', async (t) => {
  const { page, sends } = await fixture(t);
  const views = () => sends.filter((s) => !s.payload.payload.name).map((s) => s.payload.payload.url);
  const choose = async (label) => {
    await page.getByRole('button', { name: 'Analytics preferences', exact: true }).click();
    await savePreference(page, label);
  };
  await open(page);
  await page.getByRole('button', { name: 'Reject non-essentials', exact: true }).click();
  await until(() => views().length === 1);
  await choose('Accept all');
  await choose('Reject non-essentials');
  await settle(page);
  assert.deepEqual(views(), ['/']);
  await page.locator('a[href="/getting-started/quick-start/"]').first().click();
  await until(() => views().length === 2);
  await page.goBack();
  await until(() => views().length === 3);
  assert.deepEqual(views(), ['/', '/getting-started/quick-start', '/']);
  await choose('Off');
  await page.getByRole('link', { name: 'Documentation privacy', exact: true }).click();
  await settle(page);
  assert.equal(views().length, 3);
});

test('basic mode excludes the privacy page and full mode measures it once', async (t) => {
  const { page, sends } = await fixture(t, { saved: { choice: 'basic', expires: Date.now() + 60_000 } });
  await open(page, '/privacy/');
  await settle(page);
  assert.equal(sends.length, 0);
  assert.equal(await page.getByRole('region', { name: 'Page feedback' }).count(), 0);
  await page.getByRole('button', { name: 'Analytics preferences', exact: true }).click();
  await savePreference(page, 'Accept all');
  await until(() => sends.length === 1);
  assert.equal(sends[0].payload.payload.url, '/privacy');
});

test('cross-tab downgrade changes event scope without recounting the article', async (t) => {
  const { page, context, sends } = await fixture(t);
  await open(page);
  await page.getByRole('button', { name: 'Accept all', exact: true }).click();
  await until(() => sends.length === 1);
  const second = await context.newPage();
  await open(second);
  await until(() => sends.length === 2);
  await second.getByRole('button', { name: 'Analytics preferences', exact: true }).click();
  await savePreference(second, 'Reject non-essentials');
  await page.waitForFunction((key) => JSON.parse(localStorage.getItem(key)).choice === 'basic', storageKey);
  await searchFor(page, 'zzPrivateSearch_123459');
  await page.getByText('No results found', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Close Search', exact: true }).click();
  await settle(page);
  assert.equal(sends.length, 2);
  await page.getByRole('button', { name: 'No, this page was not helpful', exact: true }).click();
  await until(() => customEvents(sends).length === 1);
  assert.equal(customEvents(sends)[0].name, 'page_unhelpful');
  assert.equal(customEvents(sends)[0].data, undefined);
});

test('embedded API navigation renders endpoints and survives a static-page reload', async (t) => {
  const { page } = await fixture(t);
  await page.addInitScript(() => {
    if (!localStorage.getItem('theme')) localStorage.setItem('theme', 'dark');
  });
  await open(page, '/reference/api/');
  const reference = page.getByRole('region', { name: 'OpenAPI reference', exact: true });
  await reference.getByRole('heading', { name: /^Citadel Public API/ }).waitFor();
  await page.waitForFunction(() => document.documentElement.classList.contains('dark'));
  await page.getByRole('button', { name: 'Toggle Theme', exact: true }).click();
  await page.waitForFunction(() => !document.documentElement.classList.contains('dark'));
  await reference.getByRole('link', { name: 'Activities', exact: true }).click();
  await reference.getByRole('link', { name: /List authorized activities/ }).first().click();
  await reference.getByRole('heading', { name: /List authorized activities$/ }).waitFor();
  await page.waitForURL((url) => url.hash.startsWith('#/activities/'));
  assert.equal(new URL(page.url()).pathname, '/reference/api/');
  assert(new URL(page.url()).hash.startsWith('#/activities/'));
  await page.reload();
  await reference.getByRole('heading', { name: /List authorized activities$/ }).waitFor();
  assert.equal(await page.evaluate(() => document.documentElement.classList.contains('dark')), false);
  assert.equal(await reference.getByText('No page at this address', { exact: true }).count(), 0);
});

async function searchFor(page, text) {
  await page.keyboard.press('Control+k');
  const input = page.getByRole('combobox', { name: 'Search', exact: true });
  await input.waitFor();
  await input.fill(text);
  return input;
}

test('search records final outcomes and canonical selected pages, without query text', async (t) => {
  const { page, sends } = await fixture(t, { saved: accepted() });
  await open(page, '/?private=must-not-send#private');
  await searchFor(page, 'zzPrivateToken_937256');
  await page.getByText('No results found', { exact: true }).waitFor();
  assert.equal(customEvents(sends).length, 0, 'Typing is not a completed search interaction');
  await page.getByRole('button', { name: 'Close Search', exact: true }).click();
  await until(() => customEvents(sends).length === 2);
  assert.deepEqual(customEvents(sends).map(({ name, data }) => ({ name, data })), [
    { name: 'search_used', data: { result_count: 0 } },
    { name: 'search_no_results', data: undefined },
  ]);
  assert.equal(customEvents(sends)[0].url, '/');
  const input = await searchFor(page, 'backup');
  await page.getByRole('option').first().waitFor();
  // Keyboard selection must use the same event hook as pointer selection.
  await input.press('ArrowDown');
  await input.press('Enter');
  await until(() => customEvents(sends).some((event) => event.name === 'search_result_opened'));
  const events = customEvents(sends);
  assert.equal(events.filter((event) => event.name === 'search_used').length, 2);
  assert.equal(events.filter((event) => event.name === 'search_no_results').length, 1);
  const selection = events.find((event) => event.name === 'search_result_opened');
  assert.match(selection.data.selected_page, /^\/(?:resources|guides|operations|reference|getting-started|overview)\//);
  assert(!/[?#]/.test(selection.data.selected_page));
  assert.equal(events.filter((event) => event.name === 'search_used')[1].data.result_count > 0, true);
  assert(!JSON.stringify(sends).includes('zzPrivateToken_937256'));
  for (const event of events) {
    assert.deepEqual(Object.keys(event).sort(), ['hostname', 'name', 'title', 'url', 'website', ...(event.data ? ['data'] : [])].sort());
  }
});

test('page helpfulness sends one response per page per tab session', async (t) => {
  const { page, sends } = await fixture(t);
  await open(page);
  assert.equal(await page.getByRole('region', { name: 'Page feedback' }).count(), 0);
  await page.getByRole('button', { name: 'Accept all', exact: true }).click();
  const yes = page.getByRole('button', { name: 'Yes, this page was helpful', exact: true });
  await yes.click();
  await until(() => customEvents(sends).length === 1);
  assert.equal(customEvents(sends)[0].name, 'page_helpful');
  assert.equal(customEvents(sends)[0].url, '/');
  assert.equal(customEvents(sends)[0].data, undefined);
  assert(await yes.isDisabled());
  assert(await page.getByRole('button', { name: 'No, this page was not helpful', exact: true }).isDisabled());
  await page.setViewportSize({ width: 390, height: 844 });
  await page.getByRole('region', { name: 'Page feedback' }).screenshot({ path: '/tmp/citadel-docs-helpfulness.png' });
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.locator('a[href="/getting-started/quick-start/"]').first().click();
  await page.getByRole('button', { name: 'No, this page was not helpful', exact: true }).click();
  await until(() => customEvents(sends).length === 2);
  assert.equal(customEvents(sends)[1].name, 'page_unhelpful');
  assert.equal(customEvents(sends)[1].url, '/getting-started/quick-start');
  await page.goBack();
  await yes.waitFor();
  assert(await yes.isDisabled());
  assert.equal(customEvents(sends).length, 2);
  await page.getByRole('button', { name: 'Analytics preferences', exact: true }).click();
  await savePreference(page, 'Off');
  assert.equal(await page.getByRole('region', { name: 'Page feedback' }).count(), 0);
});

test('reopening a settled search counts a new interaction once', async (t) => {
  const { page, sends } = await fixture(t, { saved: accepted() });
  await open(page);
  await searchFor(page, 'zzPrivateSearch_123459');
  await page.getByText('No results found', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Close Search', exact: true }).click();
  await until(() => customEvents(sends).length === 2);
  await page.keyboard.press('Control+k');
  await page.getByRole('combobox', { name: 'Search', exact: true }).waitFor();
  await page.getByRole('button', { name: 'Close Search', exact: true }).click();
  await until(() => customEvents(sends).length === 4);
  assert.equal(customEvents(sends).filter((event) => event.name === 'search_used').length, 2);
});

for (const [name, options] of Object.entries({
  'no consent': {},
  declined: { saved: { choice: 'declined', expires: Date.now() + 60_000 } },
  dnt: { saved: accepted(), signal: 'dnt' },
  'gpc with Basic': { saved: { choice: 'basic', expires: Date.now() + 60_000 }, signal: 'gpc' },
})) test(`${name} blocks search events while search still works`, async (t) => {
  const { page, sends } = await fixture(t, options);
  await open(page);
  assert.equal(await page.getByRole('region', { name: 'Page feedback' }).count(), name === 'gpc with Basic' ? 1 : 0);
  await searchFor(page, 'zzPrivateSearch_123459');
  await page.getByText('No results found', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Close Search', exact: true }).click();
  await settle(page);
  assert.deepEqual(customEvents(sends), []);
  if (name === 'gpc with Basic') {
    await page.getByRole('button', { name: 'Yes, this page was helpful', exact: true }).click();
    await until(() => customEvents(sends).length === 1);
    assert.equal(customEvents(sends)[0].name, 'page_helpful');
    assert.equal(customEvents(sends)[0].data, undefined);
  }
});

test('empty searches and failed search loads never become no-results events', async (t) => {
  const { page, context, sends } = await fixture(t, { saved: accepted() });
  await context.route(`${origin}/api/search`, (route) => route.fulfill({ status: 503, body: 'Unavailable' }));
  await open(page);
  await searchFor(page, '');
  await page.getByRole('button', { name: 'Close Search', exact: true }).click();
  await searchFor(page, 'backup');
  await page.waitForTimeout(500);
  await page.getByRole('button', { name: 'Close Search', exact: true }).click();
  await settle(page);
  assert.deepEqual(customEvents(sends), []);
});


test('old documentation links redirect once to canonical pages and retain query and section', async (t) => {
  const { page, sends } = await fixture(t, { saved: accepted() });
  await page.goto(origin + '/docs/getting-started/install/?from=bookmark#network-access');
  await page.waitForURL(origin + '/getting-started/install/?from=bookmark#network-access');
  await page.getByRole('heading', { name: 'Install and secure Citadel', exact: true }).waitFor();
  await until(() => sends.length === 1);
  assert.equal(sends[0].payload.payload.url, '/getting-started/install');
  await page.goto(origin + '/docs/guides/builds/?from=old#build-pools');
  await page.waitForURL(origin + '/resources/build-pools/?from=old#configure-a-pool');
  await until(() => sends.length === 2);
  assert.equal(sends[1].payload.payload.url, '/resources/build-pools');
  await page.goto(origin + '/docs/');
  await page.waitForURL(origin + '/');
  await until(() => sends.length === 3);
  assert.equal(sends[2].payload.payload.url, '/');
});

test('homepage licensing fragment redirects without changing ordinary home links', async (t) => {
  const { page } = await fixture(t);
  await page.goto(origin + '/?from=bookmark#licensing');
  await page.waitForURL(origin + '/overview/licensing/?from=bookmark');
  await page.getByRole('heading', { name: 'Licensing', exact: true }).waitFor();

  await open(page, '/?from=home#start-here');
  await settle(page);
  assert.equal(page.url(), origin + '/?from=home#start-here');
  await page.evaluate(() => { location.hash = 'licensing'; });
  await page.waitForURL(origin + '/overview/licensing/?from=home');

  await open(page);
  await settle(page);
  assert.equal(page.url(), origin + '/');
});

for (const choice of ['basic', 'accepted', 'declined']) test(`installation copy success respects ${choice} consent`, async (t) => {
  const { page, sends } = await fixture(t, { saved: { choice, expires: Date.now() + 60_000 } });
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: async () => {} } });
  });
  await open(page, '/getting-started/quick-start/');
  await page.getByRole('button', { name: 'Copy Text', exact: true }).first().click();
  await page.getByRole('button', { name: 'Copied Text', exact: true }).waitFor();
  if (choice !== 'declined') {
    await until(() => sends.some(e => e.payload.payload.name === 'install_command_copied'));
    const event = sends.find(e => e.payload.payload.name === 'install_command_copied').payload.payload;
    assert.equal(event.url, '/getting-started/quick-start');
    assert.deepEqual(event.data, choice === 'accepted' ? { step: 'download' } : undefined);
  } else assert.equal(sends.length, 0);
  const count = sends.length;
  await page.evaluate(() => { navigator.clipboard.writeText = async () => { throw new Error('Denied'); }; });
  await page.getByRole('button', { name: 'Copy Text', exact: true }).last().click();
  await page.getByRole('button', { name: 'Copy failed. Select the commands to copy them.', exact: true }).waitFor();
  await settle(page);
  assert.equal(sends.length, count);
  assert(!/docker compose|PG_PASSWORD|curl/.test(JSON.stringify(sends)));
});

for (const choice of ['basic', 'accepted']) test(`OpenAPI download records ${choice} intent without schema contents`, async (t) => {
  const { page, sends } = await fixture(t, { saved: { choice, expires: Date.now() + 60_000 } });
  await open(page, '/reference/api/');
  await page.getByRole('link', { name: 'Download OpenAPI schema' }).click();
  await until(() => sends.some(e => e.payload.payload.name === 'openapi_download_requested'));
  const event = sends.find(e => e.payload.payload.name === 'openapi_download_requested').payload.payload;
  assert.equal(event.url, '/reference/api');
  assert.equal(event.data, undefined);
});
