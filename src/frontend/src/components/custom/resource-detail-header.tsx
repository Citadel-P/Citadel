import type { ReactNode } from 'react';

export function ResourceDetailHeader({
  icon,
  children,
  actions,
}: {
  icon: ReactNode;
  children: ReactNode;
  actions: ReactNode;
}) {
  return (
    <header className="flex min-w-0 flex-col overflow-hidden rounded-lg border bg-card shadow-xs lg:flex-row lg:items-center">
      <div className="flex min-w-0 flex-1 items-start gap-4 p-(--surface-padding)">
        <span
          aria-hidden="true"
          className="flex size-11 shrink-0 items-center justify-center rounded-lg border border-primary/15 bg-primary/5 text-primary dark:text-foreground">
          {icon}
        </span>
        <div className="flex min-w-0 flex-1 flex-col gap-1">{children}</div>
      </div>
      <div className="flex min-w-0 flex-wrap items-center gap-3 border-t bg-muted/10 px-(--surface-padding) py-3 lg:max-w-[45%] lg:shrink-0 lg:justify-end lg:border-t-0 lg:bg-transparent lg:py-(--surface-padding)">
        {actions}
      </div>
    </header>
  );
}
