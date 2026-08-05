import { ContainerStatView } from '@/api/generated/api.types';
import { StatsWindowHours } from '@/components/custom/common';
import { StatsQueryState } from '@/features/docker-resources/containers/container-info/container-stats';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';
import { appendBoundedLiveStat, STREAMED_STATS_QUERY_OPTIONS } from '@/lib/live-stats';
import { normalizeDockerId } from '@/lib/utils';
import { HubConnection } from '@microsoft/signalr';
import { useCallback, useMemo, useState } from 'react';

export type SwarmTaskStatsQueryState = StatsQueryState & {
  dockerContainerId?: string;
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
    error: query.error,
    isLoading: query.isLoading,
    windowHours,
    onWindowHoursChange: setWindowHours,
  };
};

export const useTaskStatsStream = (dockerContainerId?: string) => {
  const [stream, setStream] = useState<{ dockerContainerId?: string; stats: ContainerStatView[] }>({ stats: [] });
  const normalizedContainerId = normalizeDockerId(dockerContainerId);

  const handleStats = useCallback(
    (container: { containerStat?: ContainerStatView | null }) => {
      const current = container.containerStat;
      if (!dockerContainerId || !current) return;

      const timestamped = {
        ...current,
        created: Number(current.created) > 0 ? current.created : Math.floor(Date.now() / 1000),
      };
      setStream((previous) => ({
        dockerContainerId,
        stats: appendBoundedLiveStat(
          previous.dockerContainerId === dockerContainerId ? previous.stats : EMPTY_STATS,
          timestamped,
        ),
      }));
    },
    [dockerContainerId],
  );
  const setupEventListeners = useCallback(
    (connection: HubConnection) => connection.on('ReceiveContainerInfo', handleStats),
    [handleStats],
  );
  const removeEventListeners = useCallback(
    (connection: HubConnection) => connection.off('ReceiveContainerInfo', handleStats),
    [handleStats],
  );

  useSignalRGroup({
    groupName: normalizedContainerId ? `container-info:${normalizedContainerId}` : undefined,
    setupEventListeners,
    removeEventListeners,
    skip: !normalizedContainerId,
  });

  return stream.dockerContainerId === dockerContainerId ? stream.stats : EMPTY_STATS;
};
