// Replacement for monaco-worker-manager/worker.js, compatible with Monaco 0.55+.
//
// Monaco 0.55 dropped the old createWebWorker({ moduleId, label, createData }) API
// and stopped forwarding createData through the $initialize protocol message.
// This patch uses Monaco's start() directly and gets createData from the
// out-of-band 'yaml-init' message sent by the main thread before Monaco's own
// bootstrap messages ('-please-ignore-' and '$initialize') arrive.

// @ts-expect-error – internal Monaco path has no .d.ts; Vite bundles it correctly.
import { start } from 'monaco-editor/esm/vs/editor/editor.worker.start.js';

export function initialize(fn: (ctx: unknown, createData: unknown) => Record<string, unknown>): void {
  // Phase 1 – '-please-ignore-' bootstrap ping from Monaco's WebWorker constructor.
  self.onmessage = () => {
    // Phase 2 – '$initialize' protocol message.
    self.onmessage = (m: MessageEvent) => {
      const createData = (globalThis as Record<string, unknown>)._yamlCreateData ?? {};
      start((ctx: unknown) => Object.create(fn(ctx, createData)));
      // start() set globalThis.onmessage to WebWorkerServer's handler but did not
      // process this message. Replay it so the server replies to $initialize and
      // _onModuleLoaded resolves on the main thread.
      (globalThis.onmessage as ((m: MessageEvent) => void) | null)?.(m);
    };
  };
}
