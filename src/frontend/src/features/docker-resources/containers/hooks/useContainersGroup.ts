import { useCallback, useMemo } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { ContainersResponse, ContainerView, ContainerStatView } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { applyContainerChange, applyContainerStatePatches, reconcileContainerOrder } from './container-order';
import type { ContainerStatePatch } from './container-order';

export const useContainersGroup = (platformId?: string) => {
  const { data, isLoading, error, refetch, isFetching } = useRead('listContainers', { id: platformId });
  const queryClient = useQueryClient();
  const key = useMemo(() => ['listContainers', { id: platformId }], [platformId]);
  const containersInfo = data?.data;
  const capabilities = containersInfo?.capabilities;
  const setContainersInfo = useCallback(
    (update: (current?: ContainersResponse) => ContainersResponse | undefined) => {
      queryClient.setQueryData<{ data: ContainersResponse }>(key, (previous) => {
        const next = update(previous?.data);
        return next && next !== previous?.data ? { ...previous, data: next } : previous;
      });
    },
    [key, queryClient],
  );

  const handleContainersInfoUpdated = useCallback(
    (snapshot: ContainersResponse & { platformId: string }) => {
      if (snapshot.platformId !== platformId) return;
      void queryClient.cancelQueries({ queryKey: key, exact: true });
      setContainersInfo((previous) => ({
        ...snapshot,
        containers: reconcileContainerOrder(previous?.containers, snapshot.containers),
      }));
    },
    [platformId, key, queryClient, setContainersInfo],
  );

  const handleContainersStatsUpdated = useCallback(
    (stats: ContainerStatView[]) => {
      setContainersInfo((currentInfo) => {
        if (!currentInfo || !currentInfo.containers) {
          return currentInfo;
        }

        const statsMap = new Map(stats.map((stat) => [stat.containerId, stat]));
        let hasChanged = false;

        const updatedContainers = currentInfo.containers.map((container) => {
          const stat = statsMap.get(container.id);
          if (stat) {
            hasChanged = true;
            return { ...container, lastStats: stat };
          }
          return container;
        });

        if (hasChanged) {
          return { ...currentInfo, containers: updatedContainers };
        }

        return currentInfo;
      });
    },
    [setContainersInfo],
  );

  const handleContainersChanged = useCallback(
    (change: {
      platformId: string;
      containerId: string;
      containers: ContainerView[];
      stackIds?: string[];
      stackBackupCoverage?: ContainersResponse['stackBackupCoverage'];
    }) => {
      if (change.platformId !== platformId) return;
      setContainersInfo((current) => {
        if (!current) return current;
        const containers = applyContainerChange(current.containers, change.containerId, change.containers);
        if (!change.stackIds) return { ...current, containers };
        const represented = new Set(containers.map((c) => c.stackId));
        const coverage = { ...current.stackBackupCoverage, ...change.stackBackupCoverage };
        for (const id of change.stackIds) {
          if (!change.stackBackupCoverage?.[id]) delete coverage[id];
        }
        for (const id of Object.keys(coverage)) {
          if (!represented.has(id)) delete coverage[id];
        }
        return { ...current, containers, stackBackupCoverage: coverage };
      });
    },
    [platformId, setContainersInfo],
  );

  const handleContainerStateChanged = useCallback(
    (change: { platformId: string; patches: ContainerStatePatch[] }) => {
      if (change.platformId !== platformId) return;
      setContainersInfo((current) =>
        current
          ? {
              ...current,
              containers: applyContainerStatePatches(current.containers ?? [], change.patches),
            }
          : current,
      );
    },
    [platformId, setContainersInfo],
  );

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('ContainersInfoUpdated', handleContainersInfoUpdated);
      hubConnection.on('ContainersStatsUpdated', handleContainersStatsUpdated);
      hubConnection.on('ContainersChanged', handleContainersChanged);
      hubConnection.on('ContainersStateChanged', handleContainerStateChanged);
    },
    [handleContainersStatsUpdated, handleContainersInfoUpdated, handleContainersChanged, handleContainerStateChanged],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('ContainersInfoUpdated', handleContainersInfoUpdated);
      hubConnection.off('ContainersStatsUpdated', handleContainersStatsUpdated);
      hubConnection.off('ContainersChanged', handleContainersChanged);
      hubConnection.off('ContainersStateChanged', handleContainerStateChanged);
    },
    [handleContainersInfoUpdated, handleContainersStatsUpdated, handleContainersChanged, handleContainerStateChanged],
  );

  useRealtimeGroup({
    groupName: `containers:${platformId}`,
    setupEventListeners,
    removeEventListeners,
    skip: !platformId,
  });

  return { error, refetch, isFetching, containersInfo, capabilities, isLoading };
};
