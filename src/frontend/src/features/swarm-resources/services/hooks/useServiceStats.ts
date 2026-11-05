import { appendLiveStatBucket, latestLiveStat, sumStatBuckets } from '@/lib/stats-buckets';
import { ContainerStatView } from '@/api/generated/api.types';
import { useStatsRange } from '@/hooks/useStatsRange';
import { StatsQueryState } from '@/features/docker-resources/containers/container-info/container-stats';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { STREAMED_STATS_QUERY_OPTIONS } from '@/lib/live-stats';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useCallback, useMemo, useState } from 'react';

export type SwarmServiceStatsQueryState = StatsQueryState & {
  complete: boolean;
  observedTasks: number;
  expectedTasks: number;
  missingDockerNodeIds: string[];
  observedContainerProjectionIds: string[];
  error?: unknown;
};

const EMPTY_STATS: ContainerStatView[] = [];

export const useServiceStatsWindow = (platformId: string, serviceId: string): SwarmServiceStatsQueryState => {
  const range = useStatsRange();
  const args = useMemo(
    () => ({ platformId, resourceId: serviceId, query: range.query }),
    [platformId, serviceId, range.query],
  );
  const query = useRead('getSwarmServiceStats', args, { ...STREAMED_STATS_QUERY_OPTIONS, retry: false });
  const result = query.data?.data;

  return {
    baseStats: result?.stats ?? EMPTY_STATS,
    complete: result?.complete ?? false,
    observedTasks: Number(result?.observedTasks ?? 0),
    expectedTasks: Number(result?.expectedTasks ?? 0),
    missingDockerNodeIds: result?.missingDockerNodeIds ?? [],
    observedContainerProjectionIds: result?.observedContainerProjectionIds ?? [],
    error: query.error,
    isLoading: query.isLoading,
    range,
  };
};

export const useServiceStatsStream = (
  platformId: string,
  containerIds: string[],
  seconds = 10,
  windowSeconds = 3600,
) => {
  const [liveStats, setLiveStats] = useState<{
    identity: string;
    byContainer: Map<string, ContainerStatView[]>;
  }>({ identity: '', byContainer: new Map() });
  const idSet = useMemo(() => new Set(containerIds), [containerIds]);
  const identity = useMemo(() => [...containerIds].sort().join(','), [containerIds]);

  const handleStats = useCallback(
    (stats: ContainerStatView[]) => {
      setLiveStats((previous) => {
        const byContainer =
          previous.identity === identity ? new Map(previous.byContainer) : new Map<string, ContainerStatView[]>();
        let changed = false;
        for (const stat of stats) {
          if (!stat.containerId || !idSet.has(stat.containerId)) continue;
          const series = byContainer.get(stat.containerId) ?? [];
          const before = latestLiveStat(series);
          if (before && stat.created <= before.created) continue;
          byContainer.set(stat.containerId, appendLiveStatBucket(series, stat, seconds, windowSeconds));
          changed = true;
        }
        return changed ? { identity, byContainer } : previous;
      });
    },
    [idSet, identity, seconds, windowSeconds],
  );
  const setupEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.on('ContainersStatsUpdated', handleStats),
    [handleStats],
  );
  const removeEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.off('ContainersStatsUpdated', handleStats),
    [handleStats],
  );

  useRealtimeGroup({
    groupName: platformId ? `containers:${platformId}` : undefined,
    setupEventListeners,
    removeEventListeners,
    skip: !platformId,
  });

  const stats = useMemo(
    () =>
      liveStats.identity === identity ? sumStatBuckets([...liveStats.byContainer.values()], seconds) : EMPTY_STATS,
    [liveStats, identity, seconds],
  );
  return { stats, observedContainers: liveStats.identity === identity ? liveStats.byContainer.size : 0 };
};
