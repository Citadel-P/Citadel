import { useState, useCallback, useMemo } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { ContainersView, ContainerStatView } from '@/api/generated/api.types';
import { useDockerDaemonGroup, ContainerEvent } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';

export const useContainersGroup = (platformId?: string) => {
  const { data, isLoading } = useRead('listContainers', { id: platformId });
  const [realtimeContainersInfo, setRealtimeContainersInfo] = useState<ContainersView>();

  const containersInfo = useMemo<ContainersView | undefined>(() => {
    if (realtimeContainersInfo) {
      return realtimeContainersInfo;
    }

    if (data?.data) {
      return {
        containers: data.data.containers,
      };
    }

    return undefined;
  }, [realtimeContainersInfo, data]);

  const onContainerEvent = useCallback(
    (containerEvent: ContainerEvent) => {
      setRealtimeContainersInfo((currentInfo) => {
        const source =
          currentInfo ??
          (data?.data
            ? {
                containers: data.data.containers,
              }
            : undefined);

        if (!source?.containers) {
          return source;
        }

        const updatedContainers = [...source.containers];

        const existingIndex = updatedContainers.findIndex(
          (c) => c.containerId === containerEvent.container.containerId,
        );

        switch (containerEvent.eventType) {
          case 'create': {
            if (existingIndex === -1) {
              return {
                ...source,
                containers: [containerEvent.container, ...updatedContainers],
              };
            }

            if (JSON.stringify(updatedContainers[existingIndex]) !== JSON.stringify(containerEvent.container)) {
              updatedContainers[existingIndex] = containerEvent.container;

              return {
                ...source,
                containers: updatedContainers,
              };
            }

            break;
          }

          case 'destroy': {
            if (existingIndex !== -1) {
              updatedContainers.splice(existingIndex, 1);

              return {
                ...source,
                containers: updatedContainers,
              };
            }

            break;
          }

          default: {
            if (
              existingIndex !== -1 &&
              JSON.stringify(updatedContainers[existingIndex]) !== JSON.stringify(containerEvent.container)
            ) {
              updatedContainers[existingIndex] = containerEvent.container;

              return {
                ...source,
                containers: updatedContainers,
              };
            }

            break;
          }
        }

        return source;
      });
    },
    [data],
  );

  useDockerDaemonGroup(platformId, { onContainerEvent });

  const handleContainersInfoUpdated = useCallback((containers: ContainersView) => {
    setRealtimeContainersInfo(containers);
  }, []);

  const handleContainersStatsUpdated = useCallback(
    (stats: ContainerStatView[]) => {
      setRealtimeContainersInfo((currentInfo) => {
        const source =
          currentInfo ??
          (data?.data
            ? {
                containers: data.data.containers,
              }
            : undefined);

        if (!source?.containers) {
          return source;
        }

        const statsMap = new Map(stats.map((stat) => [stat.containerId, stat]));

        let hasChanged = false;

        const updatedContainers = source.containers.map((container) => {
          const stat = statsMap.get(container.containerId);

          if (!stat) {
            return container;
          }

          hasChanged = true;

          return {
            ...container,
            lastStats: stat,
          };
        });

        if (!hasChanged) {
          return source;
        }

        return {
          ...source,
          containers: updatedContainers,
        };
      });
    },
    [data],
  );

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('ContainersInfoUpdated', handleContainersInfoUpdated);

      hubConnection.on('ContainersStatsUpdated', handleContainersStatsUpdated);
    },
    [handleContainersInfoUpdated, handleContainersStatsUpdated],
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

  return {
    containersInfo,
    isLoading,
  };
};
