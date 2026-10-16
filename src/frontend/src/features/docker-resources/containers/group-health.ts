import { ContainerHealthStatus as Status, type ContainerView } from '@/api/generated/api.types';
import { currentHealth } from './health';

export type ContainerGroupHealthStatus = Status | 'Partial';
export type ContainerGroupHealth = {
  status: ContainerGroupHealthStatus;
  counts: Record<Status, number>;
  total: number;
};
export type ContainerHealthItem = ContainerView & { stackHealth?: ContainerGroupHealth };

export function groupHealth(containers: ContainerView[], now: number): ContainerGroupHealth {
  const counts = Object.fromEntries(Object.values(Status).map((status) => [status, 0])) as Record<Status, number>;
  for (const container of containers) counts[currentHealth(container, now)]++;
  const total = containers.length;
  let status: ContainerGroupHealthStatus;
  if (counts.Unhealthy) status = Status.Unhealthy;
  else if (counts.Starting) status = Status.Starting;
  else if (!total || counts.Unknown === total) status = Status.Unknown;
  else if (counts.Healthy === total) status = Status.Healthy;
  else if (counts.NoCheck + counts.Disabled === total) status = Status.NoCheck;
  else if (counts.NotRunning === total) status = Status.NotRunning;
  else status = 'Partial';
  return { status, counts, total };
}

// Carry the authorized inventory summary through search and display filters.
// Group action membership still comes only from the displayed containers.
export function withContainerGroupHealth(containers: ContainerView[], now: number): ContainerHealthItem[] {
  const key = (container: ContainerView) => JSON.stringify([container.platformId, container.stack?.trim()]);
  const groups = new Map<string, ContainerView[]>();
  for (const container of containers) {
    if (!container.stack?.trim()) continue;
    const id = key(container);
    const group = groups.get(id) ?? [];
    group.push(container);
    groups.set(id, group);
  }
  const summaries = new Map([...groups].map(([id, members]) => [id, groupHealth(members, now)]));
  return containers.map((container) => {
    const summary = summaries.get(key(container));
    return summary ? { ...container, stackHealth: summary } : container;
  });
}
