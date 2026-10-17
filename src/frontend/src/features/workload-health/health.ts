import {
  ContainerHealthStatus,
  PlatformStatus,
  type WorkloadHealth,
  type HealthMember,
} from '@/api/generated/api.types';

export type HealthResource = {
  id: string;
  name: string;
  platformStatus: PlatformStatus;
  probeHealth?: WorkloadHealth;
};
export type HealthState = ContainerHealthStatus | 'Partial';
export { healthOrder } from '@/components/custom/health-status';

export function workloadHealthCoverageFresh(health: WorkloadHealth, now = Date.now()): boolean {
  const until = health.validUntil ?? (health.observedAt != null ? health.observedAt + 180_000 : null);
  return now >= health.evaluatedAt && now - health.evaluatedAt <= 180_000 && (until == null || now <= until);
}

export function workloadHealthStatus(resource: HealthResource, now = Date.now()): HealthState {
  const health = resource.probeHealth;
  if (
    resource.platformStatus !== PlatformStatus.Online ||
    !health ||
    now < health.evaluatedAt ||
    now - health.evaluatedAt > 180_000
  )
    return ContainerHealthStatus.Unknown;
  // These deadlines include every member, even when the breakdown is truncated.
  // A stale sibling cannot hide a still-current failure or starting probe.
  if (health.unhealthyUntil != null && now <= health.unhealthyUntil) return ContainerHealthStatus.Unhealthy;
  if (health.startingUntil != null && now <= health.startingUntil) return ContainerHealthStatus.Starting;
  if (!workloadHealthCoverageFresh(health, now)) return ContainerHealthStatus.Unknown;
  return health.status as HealthState;
}

export function memberHealth(member: HealthMember) {
  return {
    state: member.state,
    projectionStaleSince: member.available ? null : 1,
    health: {
      status: ContainerHealthStatus[member.status],
      observedAt: member.observation?.observedAt,
      lastCheckAt: member.observation?.lastCheckAt,
      failingStreak: member.observation?.failingStreak,
      reason: member.completedJob ? 'This applied one-shot job completed successfully.' : '',
    },
  };
}

export function matchesHealth(resource: HealthResource, selected: string[], now: number, connected = true) {
  if (!selected.length) return true;
  const status = connected ? workloadHealthStatus(resource, now) : ContainerHealthStatus.Unknown;
  if (selected.includes(status)) return true;
  // Expired snapshots cannot keep matching a formerly passing/failing observation.
  if (status === ContainerHealthStatus.Unknown) return false;
  if (resource.probeHealth && !workloadHealthCoverageFresh(resource.probeHealth, now)) {
    // Counts describe the last assessment, not which other members remain fresh.
    return (
      selected.includes('Unknown') ||
      (selected.includes('Starting') &&
        resource.probeHealth.startingUntil != null &&
        now <= resource.probeHealth.startingUntil)
    );
  }
  if (selected.includes('Unknown') && !resource.probeHealth?.coverageComplete) return true;
  const counts = resource.probeHealth?.counts;
  if (!counts) return false;
  const matches: Record<string, number> = {
    Unhealthy: counts.unhealthy,
    Starting: counts.starting,
    NoCheck: counts.noCheck,
    Disabled: counts.disabled,
    NotRunning: counts.notRunning,
    Unknown: counts.unknown,
  };
  return selected.some((state) => (matches[state] ?? 0) > 0);
}
