import type { LucideIcon } from 'lucide-react';
import { cn } from '@/lib/utils';

export type OverviewFilter<T> = {
  id: string;
  label: string;
  description: string;
  icon: LucideIcon;
  tone?: 'neutral' | 'success' | 'warning';
  matches?: (item: T) => boolean;
};

type OverviewMetric = Omit<OverviewFilter<never>, 'matches'> & { value: number };

/** Counts reflect the current search and external filters, before the selected overview filter. */
export function ResourceOverview({
  label,
  metrics,
  activeId,
  onSelect,
}: {
  label: string;
  metrics: OverviewMetric[];
  activeId: string;
  onSelect: (id: string) => void;
}) {
  return (
    <section aria-label={label} className="space-y-2">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-sm font-medium">{label}</h2>
        <span className="text-xs text-muted-foreground">Filter the current view</span>
      </div>
      <div
        className={cn(
          'grid grid-cols-2 gap-[calc(var(--section-gap)/2)]',
          metrics.length === 3 ? 'sm:grid-cols-3' : metrics.length === 5 ? 'xl:grid-cols-5' : 'xl:grid-cols-4',
        )}>
        {metrics.map(({ id, label, value, description, icon: Icon, tone = 'neutral' }) => (
          <button
            key={id}
            type="button"
            aria-label={`${label}: ${value.toLocaleString()}. ${description}`}
            aria-pressed={activeId === id}
            onClick={() => onSelect(id)}
            className={cn(
              'min-w-0 rounded-lg border bg-card px-[calc(var(--surface-padding)-0.25rem)] py-[calc(var(--surface-padding)-0.5rem)] text-left shadow-xs transition-colors hover:border-primary/50 hover:bg-primary/5 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring',
              activeId === id && 'border-primary/60 bg-primary/5 ring-1 ring-primary/20',
            )}>
            <span className="flex items-center justify-between gap-2 text-sm text-muted-foreground">
              {label}
              <Icon
                aria-hidden="true"
                className={cn(
                  'size-4 shrink-0',
                  tone === 'success'
                    ? 'text-success'
                    : tone === 'warning'
                      ? 'text-warning'
                      : 'text-primary dark:text-muted-foreground',
                )}
              />
            </span>
            <span className="mt-1 block text-xl font-semibold tracking-tight tabular-nums">
              {value.toLocaleString()}
            </span>
            <span className="mt-0.5 block text-xs leading-snug text-muted-foreground">{description}</span>
          </button>
        ))}
      </div>
    </section>
  );
}
