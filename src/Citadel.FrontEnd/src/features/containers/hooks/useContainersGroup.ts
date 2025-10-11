import { useEffect, useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { ContainersView, ContainerStatView } from '@/api/generated/api.types';
import { useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';

export const useContainersGroup = (platformId?: string) => {
  const [isLoading, setIsLoading] = useState(false);
  const [containersInfo, setContainersInfo] = useState<ContainersView | undefined>();
  const { containerEvent } = useDockerDaemonGroup(platformId);

  const handleContainersInfoUpdated = useCallback((containers: ContainersView) => {
    setContainersInfo(containers);
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

  useEffect(() => {
    setContainersInfo((currentInfo) => {
      if (!currentInfo) {
        return currentInfo;
      }

      const updatedContainers = [...(currentInfo.containers ?? [])];
      const existingIndex = updatedContainers.findIndex((c) => c.containerId === containerEvent?.container.containerId);

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
  }, [containerEvent]);

  const getContainersList = useCallback(
    async (hubConnection: HubConnection) => {
      setIsLoading(true);
      const containers = await hubConnection.invoke<ContainersView>('GetContainers', platformId);
      if (containers) {
        setContainersInfo(containers);
      }
      setIsLoading(false);
    },
    [platformId],
  );

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

  const onJoinedGroup = useCallback(
    (hubConnection: HubConnection) => {
      if (!hubConnection) return;
      getContainersList(hubConnection);
      hubConnection.onreconnected(() => {
        getContainersList(hubConnection);
      });
    },
    [getContainersList],
  );

  useSignalRGroup({
    groupName: `containers:${platformId}`,
    setupEventListeners,
    removeEventListeners,
    onJoinedGroup,
    skip: !platformId,
  });

  return { containersInfo, isLoading };
};
