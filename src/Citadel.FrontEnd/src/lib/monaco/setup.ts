import { loader } from '@monaco-editor/react';
import * as monaco from 'monaco-editor';

import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker';
import cssWorker from 'monaco-editor/esm/vs/language/css/css.worker?worker';
import htmlWorker from 'monaco-editor/esm/vs/language/html/html.worker?worker';
import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker';
import yamlWorker from './yaml.worker.js?worker';
import { configureMonacoYaml } from 'monaco-yaml';
import composeSpecSchema from '@/api/schema/compose-spec.json';
import dockerStackComposeSchema from '@/api/schema/docker-stack-compose-v3.13.json';

let initialization: Promise<void> | undefined;

export function initializeMonaco() {
  return (initialization ??= initialize());
}

async function initialize() {
  self.MonacoEnvironment = {
    getWorker(_workerId, label) {
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
      return new editorWorker();
    },
  };

  // Monaco 0.55 changed createWebWorker to require { worker } instead of
  // { moduleId, label, createData }.  monaco-yaml still uses the old API, so
  // we intercept the call, create the yaml Worker ourselves, send the createData
  // out-of-band (yaml-init message arrives before Monaco's own bootstrap messages),
  // and inject the Worker instance as opts.worker.
  const origCreateWebWorker = (monaco.editor.createWebWorker as (...args: unknown[]) => unknown).bind(monaco.editor);
  (monaco.editor as unknown as Record<string, unknown>).createWebWorker = (opts: Record<string, unknown>) => {
    if (opts.label === 'yaml' && !opts.worker) {
      const worker = new yamlWorker();
      // Send createData before Monaco's '-please-ignore-' bootstrap message
      // (which arrives in a Promise microtask, so our synchronous postMessage wins).
      worker.postMessage({ type: 'yaml-init', createData: opts.createData ?? {} });
      return origCreateWebWorker({ worker });
    }
    return origCreateWebWorker(opts);
  };

  configureMonacoYaml(monaco, {
    enableSchemaRequest: false,
    validate: true,
    schemas: [
      {
        fileMatch: [
          'compose.yml',
          'compose.yaml',
          '/compose.yml',
          '/compose.yaml',
          '**/compose.yml',
          '**/compose.yaml',
          'file:///compose.yml',
          'file:///compose.yaml',
          'file:///**/compose.yml',
          'file:///**/compose.yaml',
        ],
        uri: 'inmemory://schema/compose-spec.json',
        schema: composeSpecSchema as any,
      },
      {
        fileMatch: [
          'swarm-compose.yml',
          'swarm-compose.yaml',
          'file:///swarm-compose.yml',
          'file:///swarm-compose.yaml',
        ],
        uri: 'inmemory://schema/docker-stack-compose-v3.13.json',
        schema: dockerStackComposeSchema as any,
      },
    ],
  });

  // Configure loader to use local monaco instance
  loader.config({ monaco });

  const [monacoInstance, { registerYaml }, { registerKeyValue }, { registerStringList }] = await Promise.all([
    loader.init(),
    import('./yaml'),
    import('./key_value'),
    import('./string_list'),
  ]);
  registerYaml(monacoInstance);
  registerKeyValue(monacoInstance);
  registerStringList(monacoInstance);
}
