import { lazy, Suspense, type ComponentProps, type ComponentType } from 'react';
import Loader from '@/components/ui/loader';

export type { MonacoDiagnostic, SupportedLanguage } from './editors';
export { configureAutomationActionEditor } from './automation_action';

let editors: ReturnType<typeof initializeEditors> | undefined;
const loadEditors = () => (editors ??= initializeEditors());

async function initializeEditors() {
  const [editors, { initializeMonaco }] = await Promise.all([import('./editors'), import('./setup')]);
  await initializeMonaco();
  return editors;
}

function lazyEditor<P extends object>(load: () => Promise<{ default: ComponentType<P> }>) {
  const Editor = lazy(load);
  return function DeferredEditor(props: P) {
    return (
      <Suspense fallback={<Loader label="Loading editor…" />}>
        <Editor {...props} />
      </Suspense>
    );
  };
}

type Editors = typeof import('./editors');
export const MonacoEditor = lazyEditor<ComponentProps<Editors['MonacoEditor']>>(() =>
  loadEditors().then((m) => ({ default: m.MonacoEditor })),
);
export const MonacoDiff = lazyEditor<ComponentProps<Editors['MonacoDiff']>>(() =>
  loadEditors().then((m) => ({ default: m.MonacoDiff })),
);
export const MonacoToArrayEditor = lazyEditor<ComponentProps<Editors['MonacoToArrayEditor']>>(() =>
  loadEditors().then((m) => ({ default: m.MonacoToArrayEditor })),
);
export const MonacoToDictionaryEditor = lazyEditor<ComponentProps<Editors['MonacoToDictionaryEditor']>>(() =>
  loadEditors().then((m) => ({ default: m.MonacoToDictionaryEditor })),
);
