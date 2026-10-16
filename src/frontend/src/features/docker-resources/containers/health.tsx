import { useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router';
import { Activity } from 'lucide-react';
import {
  ContainerHealthStatus as Status,
  type ContainerHealthView,
  type ContainerView,
} from '@/api/generated/api.types';
import { ResourceFilterChip, ResourceFilterDropdown } from '@/components/custom/resource-filter';
import { CommandGroup, CommandItem } from '@/components/ui/command';
import { StateBadge } from '@/components/custom/state-badge';
import { cn } from '@/lib/utils';

export const healthLabels: Record<Status, string> = {
  Healthy: 'Healthy',
  Unhealthy: 'Unhealthy',
  Starting: 'Starting',
  NoCheck: 'No check',
  Disabled: 'Disabled',
  NotRunning: 'Not running',
  Unknown: 'Unknown',
};
type HealthContainer = { health?: ContainerHealthView | null; state?: string; projectionStaleSince?: number | null };
export function currentHealth(container: HealthContainer, now = Date.now()): Status {
  if (container.projectionStaleSince || container.state === 'Offline') return Status.Unknown;
  if (['Created', 'Exited', 'Dead', 'Paused', 'Restarting', 'Removing'].includes(container.state ?? ''))
    return Status.NotRunning;
  const health = container.health;
  if (!health?.observedAt || now - health.observedAt > 180_000 || now < health.observedAt) return Status.Unknown;
  return health.status;
}
export function useHealthClock() {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(timer);
  }, []);
  return now;
}
export function healthTone(status: Status) {
  return status === Status.Healthy
    ? 'text-emerald-600 dark:text-emerald-400'
    : status === Status.Unhealthy
      ? 'text-destructive'
      : status === Status.Starting
        ? 'text-amber-700 dark:text-amber-400'
        : 'text-muted-foreground';
}
export function HealthLabel({ status }: { status: Status }) {
  return (
    <StateBadge value={status} kind="health" label={healthLabels[status]} indicator={<Activity aria-hidden="true" />} />
  );
}
export const healthTime = (value?: string | number | null) =>
  value ? new Date(value).toLocaleString() : 'Not observed';
export function ContainerHealthSummary({
  container,
  layout = 'compact',
}: {
  container: HealthContainer;
  layout?: 'compact' | 'metrics';
}) {
  const status = currentHealth(container, useHealthClock());
  return (
    <div className="space-y-3 text-sm">
      <p className="text-muted-foreground">
        {status === Status.Unknown
          ? 'Current health evidence is unavailable or older than three minutes.'
          : status === Status.NotRunning
            ? 'Health checks are not running while this container is inactive.'
            : (container.health?.reason ?? 'Health reflects Docker’s configured check.')}
      </p>
      <dl
        className={cn('text-xs', layout === 'metrics' ? 'grid grid-cols-3 gap-3 border-t pt-4 sm:gap-5' : 'space-y-2')}>
        {[
          ['Observed', healthTime(container.health?.observedAt)],
          ['Last check', healthTime(container.health?.lastCheckAt)],
          ['Consecutive failures', container.health?.failingStreak ?? 'Not observed'],
        ].map(([label, value]) => (
          <div
            key={label}
            className={layout === 'metrics' ? 'min-w-0 space-y-1.5' : 'flex items-baseline justify-between gap-4'}>
            <dt className={cn('text-muted-foreground', layout === 'metrics' && 'min-h-8 sm:min-h-0')}>{label}</dt>
            <dd
              className={cn(
                'tabular-nums',
                layout === 'metrics' ? 'break-words font-medium leading-relaxed' : 'text-right',
              )}>
              {value}
            </dd>
          </div>
        ))}
      </dl>
    </div>
  );
}
function useHealthSelection() {
  const [params, setParams] = useSearchParams();
  const selected = Object.values(Status).filter((s) => params.getAll('health').includes(s));
  const setSelected = (values: Status[]) =>
    setParams((current) => {
      const next = new URLSearchParams(current);
      next.delete('health');
      Object.values(Status)
        .filter((s) => values.includes(s))
        .forEach((s) => next.append('health', s));
      return next;
    });
  return { selected, setSelected };
}
export function useContainerHealthFilter(items: ContainerView[], enabled = true) {
  const { selected } = useHealthSelection();
  const now = useHealthClock();
  const selection = enabled ? selected.join(',') : '';
  // A periodic render expires quiet evidence without issuing per-row runtime reads.
  return useMemo(() => {
    const statuses = selection.split(',');
    return items.filter((item) => !selection || statuses.includes(currentHealth(item, now)));
  }, [items, selection, now]);
}
export function ContainerHealthFilter() {
  const { selected, setSelected } = useHealthSelection();
  return (
    <div className="flex flex-wrap items-center gap-2">
      {selected.map((status) => (
        <ResourceFilterChip
          key={status}
          label={`Remove ${healthLabels[status]} health filter`}
          icon={<Activity className={cn('size-3.5 shrink-0', healthTone(status))} aria-hidden="true" />}
          onClick={() => setSelected(selected.filter((s) => s !== status))}>
          {healthLabels[status]}
        </ResourceFilterChip>
      ))}
      <ResourceFilterDropdown
        label="Health Filter"
        icon={Activity}
        searchPlaceholder="Search health states..."
        emptyMessage="No more health states.">
        <CommandGroup>
          {Object.values(Status)
            .filter((s) => !selected.includes(s))
            .map((status) => (
              <CommandItem
                key={status}
                value={healthLabels[status]}
                onSelect={() => setSelected([...selected, status])}
                className="cursor-pointer">
                <Activity className={cn('size-3.5 shrink-0', healthTone(status))} aria-hidden="true" />
                <span className="truncate">{healthLabels[status]}</span>
              </CommandItem>
            ))}
        </CommandGroup>
      </ResourceFilterDropdown>
    </div>
  );
}
