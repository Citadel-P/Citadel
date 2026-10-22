import {
  CheckConfiguration,
  ContainerHealthStatus as Status,
  HealthStatus,
  type ContainerView,
  type HealthMember,
} from '@/api/generated/api.types';
import { currentHealth } from './health';
import type { HealthContainer } from '@/components/custom/health-status';

export type ContainerGroupHealthStatus = Status | 'Partial';
export type ContainerGroupHealth = {
  status: ContainerGroupHealthStatus;
  counts: Record<Status, number>;
  total: number;
  members?: HealthMember[];
};
export type ContainerHealthItem = ContainerView & { stackHealth?: ContainerGroupHealth };

export function groupHealth(
  containers: HealthContainer[],
  now: number,
  expected = containers.length,
): ContainerGroupHealth {
  const counts = Object.fromEntries(Object.values(Status).map((status) => [status, 0])) as Record<Status, number>;
  for (const container of containers) counts[currentHealth(container, now)]++;
  counts.Unknown += Math.max(0, expected - containers.length);
  const total = Math.max(containers.length, expected);
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
  const summaries = new Map(
    [...groups].map(([id, containers]) => [
      id,
      {
        ...groupHealth(containers, now),
        members: containers.map((container): HealthMember => ({
          containerId: container.id,
          dockerContainerId: container.containerId,
          platformId: container.platformId,
          nodeId: container.dockerNodeId,
          name: container.name,
          service: '',
          state: container.state,
          available: !container.projectionStaleSince,
          status: HealthStatus[currentHealth(container, now)],
          observation:
            container.health?.observedAt != null
              ? {
                  configuration: CheckConfiguration.Unknown,
                  observedAt: container.health.observedAt,
                  lastCheckAt: container.health.lastCheckAt,
                  failingStreak: container.health.failingStreak,
                }
              : undefined,
        })),
      },
    ]),
  );
  return containers.map((container) => {
    const summary = summaries.get(key(container));
    return summary ? { ...container, stackHealth: summary } : container;
  });
}
