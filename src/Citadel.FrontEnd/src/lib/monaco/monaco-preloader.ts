import { loader } from '@monaco-editor/react';
import * as monaco from 'monaco-editor';

let isPreloaded = false;

export function preloadMonaco() {
  if (isPreloaded) return;
  isPreloaded = true;

  loader.config({ monaco });

  const run = () => loader.init().catch(() => {});

  if (typeof window !== 'undefined') {
    if ('requestIdleCallback' in window) {
      requestIdleCallback(() => run());
    } else {
      setTimeout(run, 1000);
    }
  }
}
