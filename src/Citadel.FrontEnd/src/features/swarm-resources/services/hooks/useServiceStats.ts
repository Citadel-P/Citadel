import { ContainerStatView } from '@/api/generated/api.types';
import { StatsWindowHours } from '@/components/custom/common';
import { StatsQueryState } from '@/features/docker-resources/containers/container-info/container-stats';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { appendBoundedLiveStat, STREAMED_STATS_QUERY_OPTIONS } from '@/lib/live-stats';
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
  const [windowHours, setWindowHours] = useState<StatsWindowHours>(24);
  const args = useMemo(
    () => ({ platformId, resourceId: serviceId, query: { hours: windowHours } }),
    [platformId, serviceId, windowHours],
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
    windowHours,
    onWindowHoursChange: setWindowHours,
  };
};

export const useServiceStatsStream = (platformId: string, containerIds: string[]) => {
  const [liveStats, setLiveStats] = useState<{
    identity: string;
    latestByContainer: Map<string, ContainerStatView>;
    stats: Omit<ContainerStatView, 'containerId'>[];
  }>({
    identity: '',
    latestByContainer: new Map(),
    stats: [],
  });
  const idSet = useMemo(() => new Set(containerIds), [containerIds]);
  const identity = useMemo(() => [...containerIds].sort().join(','), [containerIds]);

  const handleStats = useCallback(
    (stats: ContainerStatView[]) => {
      setLiveStats((previous) => {
        const latestByContainer =
          previous.identity === identity ? new Map(previous.latestByContainer) : new Map<string, ContainerStatView>();
        let changed = false;
        for (const stat of stats) {
          if (!stat.containerId || !idSet.has(stat.containerId)) continue;
          latestByContainer.set(stat.containerId, stat);
          changed = true;
        }
        if (!changed) return previous;

        const aggregate = aggregateCurrentStats(latestByContainer.values());
        return {
          identity,
          latestByContainer,
          stats: appendBoundedLiveStat(previous.identity === identity ? previous.stats : [], aggregate),
        };
      });
    },
    [idSet, identity],
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

  return liveStats.identity === identity
    ? { stats: liveStats.stats, observedContainers: liveStats.latestByContainer.size }
    : { stats: EMPTY_STATS, observedContainers: 0 };
};

const aggregateCurrentStats = (stats: Iterable<ContainerStatView>): Omit<ContainerStatView, 'containerId'> => {
  const aggregate: Omit<ContainerStatView, 'containerId'> = {
    memoryActive: 0,
    memoryCache: 0,
    cpuUsage: 0,
    memoryLimit: 0,
    rxBytes: 0,
    txBytes: 0,
    created: 0,
  };

  for (const stat of stats) {
    aggregate.memoryActive = Number(aggregate.memoryActive) + Number(stat.memoryActive ?? 0);
    aggregate.memoryCache = Number(aggregate.memoryCache) + Number(stat.memoryCache ?? 0);
    aggregate.cpuUsage = Number(aggregate.cpuUsage) + Number(stat.cpuUsage ?? 0);
    aggregate.memoryLimit = Number(aggregate.memoryLimit) + Number(stat.memoryLimit ?? 0);
    aggregate.rxBytes = Number(aggregate.rxBytes) + Number(stat.rxBytes ?? 0);
    aggregate.txBytes = Number(aggregate.txBytes) + Number(stat.txBytes ?? 0);
    aggregate.created = Math.max(Number(aggregate.created), Number(stat.created ?? 0));
  }

  return aggregate;
};
