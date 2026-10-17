import type { ContainerHealthTransitionView } from '@/api/generated/api.types';

export type HealthChartGroup = {
  entry: ContainerHealthTransitionView;
  keys: string[];
};

// Group only observations that occupy the same small visual area. Anchor each
// group at its latest actual timestamp so a dense run cannot collapse across gaps.
export function groupHealthChartPoints(entries: ContainerHealthTransitionView[], minimumInterval: number) {
  const groups: HealthChartGroup[] = [];
  const latestByState = new Map<string, HealthChartGroup>();
  for (const entry of [...entries].sort((a, b) => b.observedAt - a.observedAt)) {
    const stateKey = JSON.stringify([
      entry.health.status,
      entry.state,
      entry.oomKilled ?? false,
      entry.exitCode ?? null,
    ]);
    const previous = latestByState.get(stateKey);
    const key = JSON.stringify(entry);
    if (previous && previous.entry.observedAt - entry.observedAt < minimumInterval) {
      previous.entry.sampleCount += entry.sampleCount;
      previous.entry.firstObservedAt = Math.min(previous.entry.firstObservedAt, entry.firstObservedAt);
      previous.keys.push(key);
    } else {
      const group = { entry: { ...entry }, keys: [key] };
      groups.push(group);
      latestByState.set(stateKey, group);
    }
  }
  return groups;
}
