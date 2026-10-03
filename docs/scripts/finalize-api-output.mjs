import { copyFile, mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const docsRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const output = path.join(docsRoot, 'api-out');
await mkdir(output, { recursive: true });
await copyFile(
  path.resolve(docsRoot, '..', 'schema', 'public-v1.json'),
  path.join(output, 'public-openapi.json'),
);

const htmlPath = path.join(output, 'index.html');
const html = await readFile(htmlPath, 'utf8');
const localHtml = html.replace(
  /<script src="https:\/\/cdn\.redocly\.com\/redoc\/v[^\"]+\/bundles\/redoc\.standalone\.js"[^>]*><\/script>/,
  '<script src="./redoc.standalone.js"></script>',
);
if (localHtml === html) {
  throw new Error('The expected ReDoc CDN script was not found in the generated output.');
}

await writeFile(htmlPath, localHtml);
await copyFile(
  path.join(docsRoot, 'node_modules', 'redoc', 'bundles', 'redoc.standalone.js'),
  path.join(output, 'redoc.standalone.js'),
);
console.log('Vendored ReDoc and copied the downloadable schema into the static output.');
