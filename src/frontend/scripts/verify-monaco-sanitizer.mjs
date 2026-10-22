import { fileURLToPath } from 'node:url';
import { JSDOM } from 'jsdom';
import { build } from 'vite';
import assert from 'node:assert/strict';
import { test } from 'node:test';

test('bundles Monaco with the patched sanitizer and keeps safe hover markup', async () => {
  const entry = fileURLToPath(
    new URL('../node_modules/monaco-editor/esm/vs/base/browser/domSanitize.js', import.meta.url),
  );
  const result = await build({
    configFile: fileURLToPath(new URL('../vite.config.ts', import.meta.url)),
    logLevel: 'silent',
    build: {
      write: false,
      minify: false,
      lib: { entry, name: 'MonacoSanitizerFixture', formats: ['iife'] },
    },
  });
  if (!Array.isArray(result)) throw new Error('Expected an in-memory library build');
  const chunk = result[0].output.find((output) => output.type === 'chunk');
  if (!chunk || chunk.type !== 'chunk') throw new Error('Missing sanitizer fixture');

  const modules = Object.keys(chunk.modules).map((id) => id.replaceAll('\\', '/'));
  assert.ok(modules.some((id) => id.endsWith('/src/lib/monaco/sanitizer.ts')));
  assert.ok(modules.some((id) => id.includes('/dompurify/dist/purify.es.mjs')));
  assert.ok(!modules.some((id) => id.endsWith('/browser/dompurify/dompurify.js')));

  const dom = new JSDOM('', { runScripts: 'outside-only' });
  try {
    dom.window.eval(chunk.code);
    const fixture = dom.window.MonacoSanitizerFixture;
    const element = dom.window.document.createElement('div');
    element.innerHTML = fixture.sanitizeHtml(
      '<strong>Help</strong><code>echo hello</code><a href="javascript:alert(1)">link</a><img onerror="alert(1)">',
    );
    assert.equal(element.querySelector('strong')?.textContent, 'Help');
    assert.equal(element.querySelector('code')?.textContent, 'echo hello');
    assert.equal(element.querySelector('a')?.getAttribute('href'), null);
    assert.equal(element.querySelector('img')?.getAttribute('onerror'), null);
  } finally {
    dom.window.close();
  }
});
