import { access, readFile, stat } from 'node:fs/promises';
import path from 'node:path';
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
}
const searchBytes = (
  await Promise.all(searchFiles.map(async (file) => (await stat(file)).size))
).reduce((sum, size) => sum + size, 0);
console.log(`Static documentation smoke test passed. Search assets: ${searchBytes} bytes.`);

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
