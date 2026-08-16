import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';

const contentRoot = path.resolve('content/docs');
const pages = new Map();
const errors = [];

for (const file of await walk(contentRoot)) {
  if (!/\.mdx?$/.test(file)) continue;
  const relative = path.relative(contentRoot, file).replaceAll('\\', '/');
  const route = relative
    .replace(/\.(?:md|mdx)$/, '')
    .replace(/(^|\/)index$/, '$1')
    .replace(/\/$/, '');
  const text = await readFile(file, 'utf8');
  pages.set(`/docs${route ? `/${route}` : ''}`, {
    relative,
    anchors: headingAnchors(text),
  });
}

for (const [route, page] of pages) {
  const file = path.join(contentRoot, page.relative);
  const text = await readFile(file, 'utf8');
  for (const match of text.matchAll(/\[[^\]]*\]\(([^)]+)\)/g)) {
    const target = match[1].trim().replace(/^<|>$/g, '');
    if (/^(?:https?:|mailto:)/i.test(target)) continue;
    if (target.startsWith('#')) {
      validateAnchor(page, route, target.slice(1));
      continue;
    }
    if (!target.startsWith('/docs')) continue;
    const [targetRouteRaw, anchor] = target.split('#', 2);
    const targetRoute = targetRouteRaw.replace(/\/$/, '') || '/docs';
    const destination = pages.get(targetRoute);
    if (!destination) {
      errors.push(`${page.relative}: missing internal page ${targetRoute}`);
      continue;
    }
    if (anchor) validateAnchor(destination, targetRoute, anchor, page.relative);
  }
}

if (errors.length > 0) {
  console.error(errors.join('\n'));
  process.exitCode = 1;
} else {
  console.log(`Documentation link validation passed for ${pages.size} pages.`);
}

function validateAnchor(page, route, anchor, source = page.relative) {
  if (!page.anchors.has(anchor.toLowerCase())) {
    errors.push(`${source}: missing heading #${anchor} on ${route}`);
  }
}

function headingAnchors(text) {
  const seen = new Map();
  const anchors = new Set();
  for (const match of text.matchAll(/^#{2,6}\s+(.+)$/gm)) {
    const base = match[1]
      .replace(/<[^>]+>/g, '')
      .replace(/[`*_~]/g, '')
      .toLowerCase()
      .replace(/[^\p{L}\p{N}\s-]/gu, '')
      .trim()
      .replace(/\s+/g, '-');
    const count = seen.get(base) ?? 0;
    seen.set(base, count + 1);
    anchors.add(count === 0 ? base : `${base}-${count}`);
  }
  return anchors;
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
