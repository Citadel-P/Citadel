import { readFile } from 'node:fs/promises';
import path from 'node:path';

const html = await readFile(path.resolve('api-out/index.html'), 'utf8');
const schema = JSON.parse(await readFile(path.resolve('api-out/public-openapi.json'), 'utf8'));

if (!html.includes('Citadel API Preview') || !html.includes('<div id="redoc"')) {
  throw new Error('The standalone ReDoc document was not generated correctly.');
}
if (/>\s*Try it\s*</i.test(html) || /id="(?:try-it|execute-request)"/i.test(html)) {
  throw new Error('The generated API reference appears to expose request execution or credential UI.');
}
if (/cdn\.redocly\.com|fonts\.googleapis\.com|fonts\.gstatic\.com/.test(html)) {
  throw new Error('The generated API reference loads a runtime third-party asset.');
}
if (Object.keys(schema.paths ?? {}).length === 0) {
  throw new Error('The downloadable public schema has no paths.');
}

console.log('Standalone ReDoc smoke test passed.');
