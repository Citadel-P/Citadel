import { useEffect, useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { configureHub, IHubConfig, startConnectionWithRetry } from '@/lib/signalr.helpers';
import { ContainerInfoView, ContainersInfoView, ContainerStatView } from '@/api/_generated';
import { useContextSelector } from 'use-context-selector';
import { AuthContext } from '@/features/auth/AuthProvider';

const useContainersHub = (platformId: string) => {
  const [isLoading, setIsLoading] = useState(false);
  const [containersInfo, setContainersInfo] = useState<ContainersInfoView | undefined>();
  const accessToken = useContextSelector(AuthContext, (v) => v?.accessToken);
  const groupName = `ContainersInfo/${platformId}`;
  const baseUrl = import.meta.env.VITE_API_BASE_URL;

  const handleContainersInfoUpdated = useCallback((containers: ContainersInfoView) => {
    setContainersInfo(containers);
  }, []);

  const handleContainersStatsUpdated = useCallback((stats: ContainerStatView[]) => {
    setContainersInfo((currentInfo) => {
      if (!currentInfo || !currentInfo.containers) return;
      const statsMap = new Map(stats.map((stat) => [stat.containerId, stat]));

      const updatedContainers = currentInfo.containers.map((container) => {
        const stat = statsMap.get(container.id);
        return stat ? { ...container, lastStats: stat } : container;
      });

      return { ...currentInfo, containers: updatedContainers };
    });
  }, []);

  const handleContainerEventReceived = useCallback((containerInfo: ContainerInfoView, eventType: string) => {
    setContainersInfo((currentInfo) => {
      // If we don't have any current info, initialize with empty containers array
      if (!currentInfo) {
        return;
      }
      const updatedContainers = [...(currentInfo.containers ?? [])];

      const existingIndex = updatedContainers.findIndex(
        (container) => container.containerId === containerInfo.containerId,
      );

      switch (eventType) {
        case 'create':
          // Add container if it doesn't exist
          if (existingIndex === -1) {
            return {
              ...currentInfo,
              containers: [containerInfo, ...updatedContainers],
            };
          }
          updatedContainers[existingIndex] = containerInfo;
          return { ...currentInfo, containers: updatedContainers };

        case 'destroy':
          if (existingIndex !== -1) {
            updatedContainers.splice(existingIndex, 1);
            return { ...currentInfo, containers: updatedContainers };
          }
          // No change if container doesn't exist
          return currentInfo;

        default:
          // Update container for all other event types (stop, start, etc.)
          if (existingIndex !== -1) {
            updatedContainers[existingIndex] = containerInfo;
            return { ...currentInfo, containers: updatedContainers };
          }
      }
    });
  }, []);

  const getContainersList = useCallback(
    async (hubConnection: HubConnection) => {
      setIsLoading(true);
      const containers = await hubConnection.invoke<ContainersInfoView>('GetContainers', platformId);
      if (containers) {
        setContainersInfo(containers);
      }
      setIsLoading(false);
    },
    [platformId],
  );

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.onreconnecting(() => console.log('Reconnecting...'));
      hubConnection.onreconnected(() => {
        console.log('Reconnected');
        getContainersList(hubConnection);
      });

      hubConnection.on('ContainersInfoUpdated', handleContainersInfoUpdated);
      hubConnection.on('ContainersStatsUpdated', handleContainersStatsUpdated);
      hubConnection.on('ContainerEventReceived', handleContainerEventReceived);
    },
    [handleContainerEventReceived, handleContainersStatsUpdated, handleContainersInfoUpdated, getContainersList],
  );

  const removeEventListeners = useCallback((hubConnection: HubConnection) => {
    hubConnection.off('ContainersInfoUpdated');
    hubConnection.off('ContainersStatsUpdated');
    hubConnection.off('ContainerEventReceived');
  }, []);

  useEffect(() => {
    if (!accessToken) return;

    let hubConnection: HubConnection;
    let isCanceled = false;

    const initHub = () => {
      const config: IHubConfig = {
        url: `${baseUrl}/hubs/container`,
        accessToken,
      };
      hubConnection = configureHub(config);
    };

    const onConnected = () => {
      hubConnection
        .send('JoinGroup', groupName)
        .then(() => getContainersList(hubConnection))
        .catch((error) => console.error('Failed to join group:', error));
    };

    const connect = async () => {
      setupEventListeners(hubConnection);
      await startConnectionWithRetry(hubConnection, onConnected, isCanceled);
    };

    const cleanup = () => {
      isCanceled = true;
      if (hubConnection) {
        removeEventListeners(hubConnection);
        hubConnection.stop();
      }
    };

    initHub();
    connect();

    return cleanup;
  }, [accessToken, baseUrl, groupName, setupEventListeners, removeEventListeners, getContainersList]);

  return { containersInfo, isLoading };
};

export default useContainersHub;
