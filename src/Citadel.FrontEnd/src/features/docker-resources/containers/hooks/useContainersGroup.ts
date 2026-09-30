import { useState, useCallback, useEffect, useRef } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { ContainersResponse, ContainerView, ContainerStatView } from '@/api/generated/api.types';
import { useDockerDaemonGroup, ContainerEvent } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { applyContainerChange, applyContainerStatePatches, reconcileContainerOrder } from './container-order';
import type { ContainerStatePatch } from './container-order';

export const useContainersGroup = (platformId?: string) => {
  const { data, isLoading, error, refetch, isFetching } = useRead('listContainers', { id: platformId });
  const [containersInfo, setContainersInfo] = useState<ContainersResponse | undefined>();
  const capabilities = data?.data.capabilities;

  const lastSynchronizedDataRef = useRef<any>(null);

  useEffect(() => {
    if (data?.data && data !== lastSynchronizedDataRef.current) {
      lastSynchronizedDataRef.current = data;
      setContainersInfo(data.data);
    }
  }, [data]);

  const onContainerEvent = useCallback((containerEvent: ContainerEvent) => {
    setContainersInfo((currentInfo) => {
      if (!currentInfo) {
        return currentInfo;
      }

      const updatedContainers = [...(currentInfo.containers ?? [])];
      const existingIndex = updatedContainers.findIndex((c) => c.id === containerEvent?.container.id);

      switch (containerEvent?.eventType) {
        case 'create':
          if (existingIndex === -1) {
            return { ...currentInfo, containers: [containerEvent.container, ...updatedContainers] };
          }
          if (JSON.stringify(updatedContainers[existingIndex]) !== JSON.stringify(containerEvent.container)) {
            updatedContainers[existingIndex] = containerEvent.container;
            return { ...currentInfo, containers: updatedContainers };
          }
          break;

        case 'destroy':
          if (existingIndex !== -1) {
            updatedContainers.splice(existingIndex, 1);
            return { ...currentInfo, containers: updatedContainers };
          }
          break;

        default:
          if (
            existingIndex !== -1 &&
            JSON.stringify(updatedContainers[existingIndex]) !== JSON.stringify(containerEvent?.container)
          ) {
            if (containerEvent?.container) {
              updatedContainers[existingIndex] = containerEvent?.container;
            }
            return { ...currentInfo, containers: updatedContainers };
          }
          break;
      }

      return currentInfo;
    });
  }, []);

  useDockerDaemonGroup(platformId, { onContainerEvent });

  const handleContainersInfoUpdated = useCallback((containers: ContainersResponse) => {
    setContainersInfo((currentInfo) => ({
      ...containers,
      containers: reconcileContainerOrder(currentInfo?.containers, containers.containers),
    }));
  }, []);

  const handleContainersStatsUpdated = useCallback((stats: ContainerStatView[]) => {
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
  }, []);

  const handleContainersChanged = useCallback(
    (change: { platformId: string; containerId: string; containers: ContainerView[] }) => {
      if (change.platformId !== platformId) return;
      setContainersInfo((current) =>
        current
          ? {
              ...current,
              containers: applyContainerChange(current.containers ?? [], change.containerId, change.containers),
            }
          : current,
      );
    },
    [platformId],
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
    [platformId],
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
