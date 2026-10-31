import { access, readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import { getDocsBasePath } from '../site-config.mjs';
import { legacyDocRedirects } from '../src/lib/doc-redirects.mjs';

const requiredFiles = [
  'out/index.html',
  'out/getting-started/quick-start/index.html',
  'out/getting-started/install/index.html',
  'out/resources/index.html',
  'out/resources/platforms/docker-swarm/index.html',
  'out/resources/stacks/swarm/index.html',
  'out/resources/swarm-services/index.html',
  'out/resources/build-pools/index.html',
  'out/guides/access-control/index.html',
  'out/overview/licensing/index.html',
  'out/overview/community-commitment/index.html',
  'out/operations/control-plane-recovery/index.html',
  'out/reference/api/index.html',
  'out/api/openapi',
  'out/404.html',
  'out/robots.txt',
  'out/sitemap.xml',
  'out/api/search',
];

for (const file of requiredFiles) await access(path.resolve(file));

const pages = await Promise.all(
  requiredFiles.filter((file) => file.endsWith('.html')).map((file) => readFile(file, 'utf8')),
);
for (const [index, html] of pages.entries()) {
  if (!html.includes('<html') || !html.includes('</html>')) {
    throw new Error(`${requiredFiles[index]} is not a complete HTML document.`);
  }
  if (/cit_sa_[A-Za-z0-9_-]{8,}/.test(html)) {
    throw new Error(`${requiredFiles[index]} contains a token-shaped value.`);
  }
}

const searchFiles = await findFiles(path.resolve('out'), (file) => /search|orama/i.test(file));
if (searchFiles.length === 0) throw new Error('The static search index was not exported.');
const basePath = getDocsBasePath();
const apiPage = await readFile('out/reference/api/index.html', 'utf8');
if (!apiPage.includes('aria-label="OpenAPI reference"') ||
    !apiPage.includes(`href="${basePath}/api/openapi"`)) {
  throw new Error('The API page is missing its embedded reference or local schema download.');
}
const schema = JSON.parse(await readFile('out/api/openapi', 'utf8'));
const sourceSchema = JSON.parse(await readFile('../schema/public-v1.json', 'utf8'));
if (JSON.stringify(schema) !== JSON.stringify(sourceSchema)) {
  throw new Error('The embedded reference must serve the complete, current public schema.');
}
const sitemap = await readFile('out/sitemap.xml', 'utf8');
const siteUrl = (process.env.NEXT_PUBLIC_DOCS_URL || 'http://localhost:3000').replace(/\/+$/, '');
const canonicalUrls = [...sitemap.matchAll(/<loc>([^<]+)<\/loc>/g)].map((match) => match[1]);
assert(canonicalUrls.includes(`${siteUrl}/`), 'The sitemap must contain the documentation homepage.');
const canonicalRoutes = canonicalUrls.map((url) => {
  assert(url.startsWith(`${siteUrl}/`), `Unexpected sitemap origin: ${url}`);
  const route = url.slice(siteUrl.length).replace(/\/$/, '') || '/';
  assert(route !== '/docs' && !route.startsWith('/docs/'), `Legacy URL in sitemap: ${url}`);
  return route;
});
const redirects = legacyDocRedirects(canonicalRoutes, JSON.parse(await readFile('redirects.json', 'utf8')));
for (const [legacy, rule] of Object.entries(redirects)) {
  const html = await readFile(path.join('out', legacy, 'index.html'), 'utf8');
  assert.match(html, /name="robots" content="noindex/);
  if (!html.includes(`href="${basePath}${rule.to.replace(/\/$/, '')}/"`)) {
    throw new Error(`${legacy} is missing its static redirect fallback link.`);
  }
}
if (!pages[0].includes(`src="${basePath}/logo.svg"`)) {
  throw new Error('The documentation logo does not use the configured publication path.');
}
if (!pages[0].includes(`href="${basePath}/getting-started/quick-start/"`)) {
  throw new Error('The quick-start link does not use the configured publication path.');
}
for (const html of pages) {
  for (const match of html.matchAll(/src="(\/[^"]+)"/g)) {
    if (basePath && !match[1].startsWith(`${basePath}/`)) {
      throw new Error(`Asset ${match[1]} is missing the configured publication path.`);
    }
  }
  for (const { href } of pageLinks(html)) {
    if (!href.startsWith('/') || href.startsWith('//')) continue;
    const pathname = new URL(href, 'https://docs.example.test').pathname;
    assert(!basePath || pathname === basePath || pathname.startsWith(`${basePath}/`),
      `Internal link ${href} is missing the configured publication path.`);
    assert(!/\.mdx?$/.test(pathname), `Internal page link ${href} points to repository Markdown.`);
  }
}

const introduction = pages[0];
assert.doesNotMatch(pages[0], /name="robots" content="noindex/);
assert(pages[0].includes(`rel="canonical" href="${siteUrl}/"`), 'The homepage must have a root canonical URL.');
assert.doesNotMatch(plainText(pages[0]), /Your containers|Under your control/);
assert.match(plainText(introduction), /Citadel Docs/);
assert.match(introduction, /<h1[^>]*>What is Citadel\?<\/h1>/);
assertLink(introduction, `${basePath}/`, 'Citadel Docs');
assertLink(introduction, 'https://citadelplane.com/', 'Website');
for (const route of ['/getting-started/quick-start', '/getting-started/install',
  '/resources', '/operations/troubleshooting']) {
  assertLink(introduction, `${basePath}${route}`);
}
const licensing = await readFile('out/overview/licensing/index.html', 'utf8');
const commitment = await readFile('out/overview/community-commitment/index.html', 'utf8');
const guideRoute = `${basePath}/overview/licensing`;
const policyRoute = `${basePath}/overview/community-commitment`;
const licenseLink = pageLinks(licensing).find(({ href }) =>
  /^https:\/\/github\.com\/Citadel-P\/Citadel\/blob\/[a-f0-9]{40}\/LICENSE$/.test(href));
assert(licenseLink, 'The licensing guide must link to the reviewed source license revision.');
for (const html of [licensing, commitment]) {
  assertLink(html, guideRoute, 'Licensing');
  assertLink(html, policyRoute, 'Community commitment');
}
assertLink(await readFile('out/index.html', 'utf8'), guideRoute, 'Licensing');
for (const anchor of ['community', 'team', 'edition-comparison', 'source-licensing-and-product-entitlements',
  'community-commitment', 'open-the-license-page', 'request-a-license', 'install-or-replace-a-license',
  'expiration-and-grace-period', 'removing-a-license', 'troubleshooting']) {
  assert(licensing.includes(`id="${anchor}"`), `The licensing guide lost #${anchor}.`);
}
for (const anchor of ['source-licensing-and-product-entitlements', 'edition-comparison',
  'community-commitment', 'install-or-replace-a-license']) {
  assertLink(licensing, `#${anchor}`);
}
const overview = JSON.parse(await readFile('content/docs/overview/meta.json', 'utf8'));
for (const name of ['licensing', 'community-commitment']) {
  assert.equal(overview.pages.filter((entry) => entry === name).length, 1,
    `${name} must appear exactly once in the Overview sidebar group.`);
}
assert.equal(overview.pages.indexOf('community-commitment'), overview.pages.indexOf('licensing') + 1);
const searchIndex = JSON.parse(await readFile('out/api/search', 'utf8'));
// Search records use app routes; Fumadocs/Next applies basePath when navigating.
for (const route of ['/overview/licensing', '/overview/community-commitment']) {
  assert(Object.values(searchIndex.docs.docs).some((doc) => doc.url?.replace(/\/$/, '') === route),
    `${route} must be indexed for static search.`);
}
const searchBytes = (
  await Promise.all(searchFiles.map(async (file) => (await stat(file)).size))
).reduce((sum, size) => sum + size, 0);
console.log(`Static documentation smoke test passed. Search assets: ${searchBytes} bytes.`);

function plainText(html) {
  return html.replace(/<script\b[^>]*>[\s\S]*?<\/script>/g, '').replace(/<[^>]+>/g, ' ')
    .replace(/&amp;/g, '&').replace(/\s+/g, ' ').trim();
}

function pageLinks(html) {
  const markup = html.replace(/<script\b[^>]*>[\s\S]*?<\/script>/g, '');
  return [...markup.matchAll(/<a\b[^>]*\bhref="([^"]+)"[^>]*>([\s\S]*?)<\/a>/g)]
    .map((match) => ({ href: match[1].replace(/&amp;/g, '&'), text: plainText(match[2]) }));
}

function assertLink(html, href, text) {
  const normalize = (value) => value.replace(/\/(?=#|$)/, '');
  assert(pageLinks(html).some((link) => normalize(link.href) === normalize(href) && (!text || link.text === text)),
    `Missing ${text ?? 'link'} to ${href}.`);
}

async function findFiles(directory, predicate) {
  const { readdir } = await import('node:fs/promises');
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const item = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await findFiles(item, predicate)));
    else if (predicate(item)) files.push(item);
  }
  return files;
}
