import { useEffect, useState } from 'react';
import { Activity } from 'lucide-react';
import { ContainerHealthStatus as Status, type ContainerHealthView } from '@/api/generated/api.types';
import { StateBadge } from './state-badge';

export const healthLabels: Record<Status | 'Partial', string> = {
  Partial: 'Partial health',
  Healthy: 'Healthy',
  Unhealthy: 'Unhealthy',
  Starting: 'Starting',
  NoCheck: 'No check',
  Disabled: 'Disabled',
  NotRunning: 'Not running',
  Unknown: 'Unknown',
};
export type HealthContainer = {
  health?: ContainerHealthView | null;
  state?: string;
  projectionStaleSince?: number | null;
};
export function inactiveHealthCheckMessage(status: `${Status}`): string | undefined {
  if (status === Status.NoCheck) return 'No health check configured for this container.';
  if (status === Status.Disabled) return 'Health checks are disabled for this container.';
}
export function currentHealth(container: HealthContainer, now = Date.now()): Status {
  if (container.projectionStaleSince || container.state === 'Offline') return Status.Unknown;
  if (['Created', 'Exited', 'Dead', 'Paused', 'Restarting', 'Removing'].includes(container.state ?? ''))
    return Status.NotRunning;
  const health = container.health;
  if (!health?.observedAt || now - health.observedAt > 180_000 || now < health.observedAt) return Status.Unknown;
  return health.status;
}
export function useHealthClock() {
  const [, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(timer);
  }, []);
  // A newly received snapshot can be newer than the last timer tick.
  return Date.now();
}
export function healthTone(status: `${Status}` | 'Partial') {
  return status === Status.Healthy
    ? 'text-emerald-600 dark:text-emerald-400'
    : status === Status.Unhealthy
      ? 'text-destructive'
      : status === Status.Starting
        ? 'text-amber-700 dark:text-amber-400'
        : 'text-muted-foreground';
}
export function HealthLabel({ status }: { status: `${Status}` | 'Partial' }) {
  return (
    <StateBadge value={status} kind="health" label={healthLabels[status]} indicator={<Activity aria-hidden="true" />} />
  );
}
export const healthTime = (value?: string | number | null) =>
  value ? new Date(value).toLocaleString() : 'Not observed';

export const healthOrder: (Status | 'Partial')[] = [
  Status.Unhealthy,
  Status.Unknown,
  'Partial',
  Status.Starting,
  Status.Healthy,
  Status.NotRunning,
  Status.Disabled,
  Status.NoCheck,
];
