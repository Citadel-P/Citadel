import { loader } from '@monaco-editor/react';
import * as monaco from 'monaco-editor';

import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker';
import cssWorker from 'monaco-editor/esm/vs/language/css/css.worker?worker';
import htmlWorker from 'monaco-editor/esm/vs/language/html/html.worker?worker';
import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker';
import yamlWorker from './yaml.worker.js?worker';
import { configureMonacoYaml } from 'monaco-yaml';

let isPreloaded = false;

export function preloadMonaco() {
  if (isPreloaded) return;
  isPreloaded = true;
  configureMonacoYaml(monaco, {
    enableSchemaRequest: true,
    schemas: [
      {
        fileMatch: ['**/*compose.yml', '**/*compose.yaml'],
        uri: new URL('/api/schema/compose-spec.json', window.location.href).toString(),
      },
    ],
  });
  self.MonacoEnvironment = {
    getWorker(_, label) {
      if (label === 'json') {
        return new jsonWorker();
      }
      if (label === 'css' || label === 'scss' || label === 'less') {
        return new cssWorker();
      }
      if (label === 'html' || label === 'handlebars' || label === 'razor') {
        return new htmlWorker();
      }
      if (label === 'typescript' || label === 'javascript') {
        return new tsWorker();
      }
      if (label === 'yaml') {
        return new yamlWorker();
      }
      return new editorWorker();
    },
  };

  // Configure loader to use local monaco instance
  loader.config({ monaco });

  const run = async () => {
    try {
      const monacoInstance = await loader.init();

      const [{ registerYaml }, { registerKeyValue }, { registerStringList }] = await Promise.all([
        import('./yaml'),
        import('./key_value'),
        import('./string_list'),
      ]);

      registerYaml(monacoInstance);
      registerKeyValue(monacoInstance);
      registerStringList(monacoInstance);
    } catch (err) {
      console.error('Monaco setup failed:', err);
    }
  };

  if (typeof window !== 'undefined' && 'requestIdleCallback' in window) {
    (window as any).requestIdleCallback(run);
  } else {
    setTimeout(run, 2000);
  }
}
