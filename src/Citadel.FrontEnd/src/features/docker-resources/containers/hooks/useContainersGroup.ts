import { useState, useCallback, useEffect, useRef } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { ContainersView, ContainerStatView } from '@/api/generated/api.types';
import { useDockerDaemonGroup, ContainerEvent } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';
import { reconcileContainerOrder } from './container-order';

export const useContainersGroup = (platformId?: string) => {
  const { data, isLoading, } = useRead('listContainers', { id: platformId });
  const [containersInfo, setContainersInfo] = useState<ContainersView | undefined>();
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

  const handleContainersInfoUpdated = useCallback((containers: ContainersView) => {
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

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('ContainersInfoUpdated', handleContainersInfoUpdated);
      hubConnection.on('ContainersStatsUpdated', handleContainersStatsUpdated);
    },
    [handleContainersStatsUpdated, handleContainersInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('ContainersInfoUpdated', handleContainersInfoUpdated);
      hubConnection.off('ContainersStatsUpdated', handleContainersStatsUpdated);
    },
    [handleContainersInfoUpdated, handleContainersStatsUpdated],
  );

  useSignalRGroup({
    groupName: `containers:${platformId}`,
    setupEventListeners,
    removeEventListeners,
    skip: !platformId,
  });

  return { containersInfo, capabilities, isLoading };
};
