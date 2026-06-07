// Capture the yaml language-service createData that the main thread sends
// BEFORE Monaco's own bootstrap messages (`-please-ignore-` and `$initialize`).
// This is needed because Monaco 0.55 no longer forwards `createData` through
// the worker initialization protocol.
self.addEventListener(
  'message',
  /** @param {MessageEvent} e */
  function onYamlPreInit(e) {
    if (e.data && typeof e.data === 'object' && e.data.type === 'yaml-init') {
      // eslint-disable-next-line no-undef
      globalThis._yamlCreateData = e.data.createData;
      self.removeEventListener('message', onYamlPreInit);
    }
  },
);

// Import the monaco-yaml worker bundle.
// `monaco-worker-manager/worker` is aliased in vite.config.ts to our
// worker-manager-patch.ts which implements the Monaco 0.55-compatible protocol.
import 'monaco-yaml/yaml.worker.js';
