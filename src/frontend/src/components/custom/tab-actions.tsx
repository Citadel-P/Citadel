import { createContext, useContext, useState, type ComponentProps, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { Tabs } from '@/components/ui/tabs';
import { cn } from '@/lib/utils';

const TabActionsContext = createContext<{
  target: HTMLDivElement | null;
  setTarget: (target: HTMLDivElement | null) => void;
} | null>(null);

/** Scope actions to the nearest tab group, including nested resource tabs. */
export function TabsWithActions(props: ComponentProps<typeof Tabs>) {
  const [target, setTarget] = useState<HTMLDivElement | null>(null);
  return (
    <TabActionsContext.Provider value={{ target, setTarget }}>
      <Tabs {...props} />
    </TabActionsContext.Provider>
  );
}

export function TabActionsSlot({ className }: { className?: string }) {
  const context = useContext(TabActionsContext);
  return <div ref={context?.setTarget} className={cn('ml-auto shrink-0 empty:hidden', className)} />;
}

/** Mounted tab content owns its actions; switching away removes them automatically. */
export function TabActions({ children }: { children: ReactNode }) {
  const context = useContext(TabActionsContext);
  if (!context) return <div className="flex justify-end">{children}</div>;
  return context.target ? createPortal(children, context.target) : null;
}
