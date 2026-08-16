import { access, readFile, stat } from 'node:fs/promises';
import path from 'node:path';

const requiredFiles = [
  'out/index.html',
  'out/docs/index.html',
  'out/docs/getting-started/quick-start/index.html',
  'out/docs/getting-started/install/index.html',
  'out/docs/operations/control-plane-recovery/index.html',
  'out/404.html',
  'out/robots.txt',
  'out/sitemap.xml',
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
