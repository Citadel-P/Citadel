import { ContainerStatView } from '@/api/generated/api.types';
import { StatsWindowHours } from '@/components/custom/common';
import { StatsQueryState } from '@/features/docker-resources/containers/container-info/container-stats';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';
import { appendBoundedLiveStat, STREAMED_STATS_QUERY_OPTIONS } from '@/lib/live-stats';
import { HubConnection } from '@microsoft/signalr';
import { useCallback, useMemo, useState } from 'react';

export type SwarmTaskStatsQueryState = StatsQueryState & {
  dockerContainerId?: string;
  containerProjectionId?: string;
  error?: unknown;
};

const EMPTY_STATS: ContainerStatView[] = [];

export const useTaskStatsWindow = (platformId: string, taskId: string): SwarmTaskStatsQueryState => {
  const [windowHours, setWindowHours] = useState<StatsWindowHours>(24);
  const args = useMemo(
    () => ({ platformId, resourceId: taskId, query: { hours: windowHours } }),
    [platformId, taskId, windowHours],
  );
  const query = useRead('getSwarmTaskStats', args, { ...STREAMED_STATS_QUERY_OPTIONS, retry: false });

  return {
    baseStats: query.data?.data?.stats ?? EMPTY_STATS,
    dockerContainerId: query.data?.data?.dockerContainerId,
    containerProjectionId: query.data?.data?.containerProjectionId,
    error: query.error,
    isLoading: query.isLoading,
    windowHours,
    onWindowHoursChange: setWindowHours,
  };
};

export const useTaskStatsStream = (platformId: string, containerProjectionId?: string) => {
  const [stream, setStream] = useState<{ containerProjectionId?: string; stats: ContainerStatView[] }>({ stats: [] });

  const handleStats = useCallback(
    (stats: ContainerStatView[]) => {
      if (!containerProjectionId) return;
      const current = stats.find((stat) => stat.containerId === containerProjectionId);
      if (!current) return;

      const timestamped = {
        ...current,
        created: Number(current.created) > 0 ? current.created : Math.floor(Date.now() / 1000),
      };
      setStream((previous) => ({
        containerProjectionId,
        stats: appendBoundedLiveStat(
          previous.containerProjectionId === containerProjectionId ? previous.stats : EMPTY_STATS,
          timestamped,
        ),
      }));
    },
    [containerProjectionId],
  );
  const setupEventListeners = useCallback(
    (connection: HubConnection) => connection.on('ContainersStatsUpdated', handleStats),
    [handleStats],
  );
  const removeEventListeners = useCallback(
    (connection: HubConnection) => connection.off('ContainersStatsUpdated', handleStats),
    [handleStats],
  );

  useSignalRGroup({
    groupName: platformId && containerProjectionId ? `containers:${platformId}` : undefined,
    setupEventListeners,
    removeEventListeners,
    skip: !platformId || !containerProjectionId,
  });

  return stream.containerProjectionId === containerProjectionId ? stream.stats : EMPTY_STATS;
};
