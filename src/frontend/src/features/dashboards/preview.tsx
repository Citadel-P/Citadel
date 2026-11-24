import { createContext, useContext, useState, useSyncExternalStore, type ReactNode } from 'react';
import type { DashboardWidget } from '@/api/generated/api.types';

function previewStore() {
  const values = new Map<string, { key: string; value: ReactNode }>();
  const listeners = new Set<() => void>();
  return {
    capture(id: string, key: string, value: ReactNode) {
      values.set(id, { key, value });
    },
    read(id: string, key: string) {
      const entry = values.get(id);
      return entry?.key === key ? entry.value : undefined;
    },
    clear() {
      values.clear();
      for (const listener of listeners) listener();
    },
    subscribe(listener: () => void) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}
const PreviewContext = createContext<ReturnType<typeof previewStore> | undefined>(undefined);
export function PreviewProvider({ children }: { children: ReactNode }) {
  const [store] = useState(previewStore);
  return <PreviewContext.Provider value={store}>{children}</PreviewContext.Provider>;
}
export const usePreviewStore = () => useContext(PreviewContext);
export const previewKey = (widget: DashboardWidget, tags: string[], minutes: number) =>
  JSON.stringify([widget.settings, tags, minutes]);
const none = () => () => {};
export function FrozenPreview({ widget, tags, minutes }: { widget: DashboardWidget; tags: string[]; minutes: number }) {
  const store = usePreviewStore();
  const key = previewKey(widget, tags, minutes);
  const value = useSyncExternalStore(store?.subscribe ?? none, () => store?.read(widget.id, key));
  return value ? (
    <div inert aria-hidden="true" className="pointer-events-none text-sm opacity-70">
      {value}
    </div>
  ) : null;
}
