import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';

const contentRoot = path.resolve('content/docs');
const forbidden = /\b(?:TODO|TBD)\b/;
const errors = [];

for (const file of await walk(contentRoot)) {
  if (!/\.mdx?$/.test(file)) continue;
  const relative = path.relative(contentRoot, file).replaceAll('\\', '/');
  const text = await readFile(file, 'utf8');
  const frontmatter = text.match(/^---\s*\r?\n([\s\S]*?)\r?\n---\s*\r?\n/);
  if (!frontmatter) {
    errors.push(`${relative}: missing frontmatter`);
    continue;
  }
  if (!/^title:\s*.+$/m.test(frontmatter[1])) errors.push(`${relative}: missing title`);
  if (!/^description:\s*.+$/m.test(frontmatter[1])) errors.push(`${relative}: missing description`);
  if (forbidden.test(text)) errors.push(`${relative}: contains a release placeholder (TODO/TBD)`);
  const body = text.slice(frontmatter[0].length);
  if (/^#\s+/m.test(body)) errors.push(`${relative}: body duplicates the rendered page title with an H1`);
}

if (errors.length > 0) {
  console.error(errors.join('\n'));
  process.exitCode = 1;
} else {
  console.log('Documentation content validation passed.');
}

async function walk(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const item = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await walk(item)));
    else files.push(item);
  }
  return files;
}
