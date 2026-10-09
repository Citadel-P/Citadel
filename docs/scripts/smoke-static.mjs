import { access, readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import { getDocsBasePath } from '../site-config.mjs';

const requiredFiles = [
  'out/index.html',
  'out/docs/index.html',
  'out/docs/getting-started/quick-start/index.html',
  'out/docs/getting-started/install/index.html',
  'out/docs/resources/index.html',
  'out/docs/resources/platforms/docker-swarm/index.html',
  'out/docs/resources/stacks/swarm/index.html',
  'out/docs/resources/swarm-services/index.html',
  'out/docs/resources/build-pools/index.html',
  'out/docs/guides/access-control/index.html',
  'out/docs/overview/licensing/index.html',
  'out/docs/overview/community-commitment/index.html',
  'out/docs/operations/control-plane-recovery/index.html',
  'out/docs/reference/api/index.html',
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
const apiPage = await readFile('out/docs/reference/api/index.html', 'utf8');
if (!apiPage.includes('aria-label="OpenAPI reference"') ||
    !apiPage.includes(`href="${basePath}/api/openapi"`)) {
  throw new Error('The API page is missing its embedded reference or local schema download.');
}
const schema = JSON.parse(await readFile('out/api/openapi', 'utf8'));
const sourceSchema = JSON.parse(await readFile('../schema/public-v1.json', 'utf8'));
if (JSON.stringify(schema) !== JSON.stringify(sourceSchema)) {
  throw new Error('The embedded reference must serve the complete, current public schema.');
}
const redirects = JSON.parse(await readFile('redirects.json', 'utf8'));
for (const [legacy, rule] of Object.entries(redirects)) {
  const html = await readFile(path.join('out', legacy, 'index.html'), 'utf8');
  if (!html.includes(`href="${basePath}${rule.to}/"`)) {
    throw new Error(`${legacy} is missing its static redirect fallback link.`);
  }
}
if (!pages[0].includes(`src="${basePath}/logo.svg"`)) {
  throw new Error('The homepage logo does not use the configured publication path.');
}
if (!pages[0].includes(`href="${basePath}/docs/getting-started/quick-start/"`)) {
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

const home = pages[0];
const licensing = await readFile('out/docs/overview/licensing/index.html', 'utf8');
const commitment = await readFile('out/docs/overview/community-commitment/index.html', 'utf8');
const guideRoute = `${basePath}/docs/overview/licensing`;
const policyRoute = `${basePath}/docs/overview/community-commitment`;
const section = home.match(/<section\b[^>]*\bid="licensing"[^>]*>[\s\S]*?<\/section>/)?.[0];
assert(section, 'The homepage must expose a licensing section.');
assert.match(section, /aria-labelledby="licensing-title"/);
assert.match(section, /<h2\b[^>]*id="licensing-title"[^>]*>License and editions<\/h2>/);
assert(home.indexOf('aria-label="Everyday operations"') < home.indexOf(section));
assert(home.indexOf(section) < home.indexOf('aria-labelledby="start-title"'));
const sectionText = plainText(section);
for (const term of [/source-available/i, /Elastic License 2\.0/, /not an OSI-approved open-source license/,
  /free for personal self-hosting and internal business production use/i]) {
  assert.match(sectionText, term);
}
assert.equal((section.match(/<article\b/g) ?? []).length, 2, 'Keep Community and Team as the only edition cards.');
const communityCard = section.match(/<article\b[^>]*aria-labelledby="community-title"[^>]*>[\s\S]*?<\/article>/)?.[0];
const teamCard = section.match(/<article\b[^>]*aria-labelledby="team-title"[^>]*>[\s\S]*?<\/article>/)?.[0];
assert(communityCard && teamCard, 'Both edition cards must have accessible names.');
assert.match(plainText(communityCard), /no product-license-enforced limits/i);
assert.match(plainText(teamCard), /scheduled backups and webhook-triggered deployments require Team/i);
assert.match(plainText(teamCard), /external build pools/i);
assert.doesNotMatch(sectionText, /(?:[$€£]\s*\d|checkout|Enterprise|unlicensed)/i);
assertLink(section, `${guideRoute}#community`);
assertLink(section, `${guideRoute}#edition-comparison`);
assertLink(section, policyRoute);
const licenseLink = pageLinks(section).find(({ text }) => text === 'Read license terms');
assert(licenseLink, 'Link to the authoritative repository license.');
assert.match(licenseLink.href, /^https:\/\/github\.com\/Citadel-P\/Citadel\/blob\/[a-f0-9]{40}\/LICENSE$/,
  'Draft licensing copy must link to the reviewed source revision, not main.');
assertLink(licensing, licenseLink.href);
for (const region of [home.match(/<header\b[\s\S]*?<\/header>/)?.[0],
  home.match(/<nav\b[^>]*aria-label="Footer"[\s\S]*?<\/nav>/)?.[0]]) {
  assert(region, 'Homepage header and footer must be rendered.');
  assertLink(region, guideRoute, 'Licensing');
}
for (const html of [licensing, commitment]) {
  assertLink(html, guideRoute, 'Licensing');
  assertLink(html, policyRoute, 'Community commitment');
}
assertLink(await readFile('out/docs/index.html', 'utf8'), guideRoute, 'Licensing');
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
for (const route of ['/docs/overview/licensing', '/docs/overview/community-commitment']) {
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
