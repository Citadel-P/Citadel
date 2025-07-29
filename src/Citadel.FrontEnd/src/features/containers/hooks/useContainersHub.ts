import { useEffect, useState, useCallback, useRef } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { configureHub, IHubConfig, startConnectionWithRetry } from '@/lib/signalr.helpers';
import { ContainerView, ContainersView, ContainerStatView } from '@/api/_generated';
import { useAuthContext } from '@/features/auth/AuthProvider';

const useContainersHub = (platformId?: string) => {
  const [isLoading, setIsLoading] = useState(false);
  const [containersInfo, setContainersInfo] = useState<ContainersView | undefined>();
  const { accessToken } = useAuthContext();
  const groupName = `ContainersInfo/${platformId}`;
  const baseUrl = import.meta.env.VITE_API_BASE_URL;
  const isCanceledRef = useRef(false);

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

  const handleContainerEventReceived = useCallback((containerInfo: ContainerView, eventType: string) => {
    setContainersInfo((currentInfo) => {
      if (!currentInfo) {
        return currentInfo;
      }

      const updatedContainers = [...(currentInfo.containers ?? [])];
      const existingIndex = updatedContainers.findIndex((c) => c.containerId === containerInfo.containerId);

      switch (eventType) {
        case 'create':
          if (existingIndex === -1) {
            return { ...currentInfo, containers: [containerInfo, ...updatedContainers] };
          }
          if (JSON.stringify(updatedContainers[existingIndex]) !== JSON.stringify(containerInfo)) {
            updatedContainers[existingIndex] = containerInfo;
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
          if (existingIndex !== -1 && JSON.stringify(updatedContainers[existingIndex]) !== JSON.stringify(containerInfo)) {
            updatedContainers[existingIndex] = containerInfo;
            return { ...currentInfo, containers: updatedContainers };
          }
          break;
      }

      return currentInfo;
    });
  }, []);

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
    if (!platformId || !accessToken) return;

    let hubConnection: HubConnection;

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
      await startConnectionWithRetry(hubConnection, onConnected, isCanceledRef);
    };

    const cleanup = () => {
      isCanceledRef.current = true;
      if (hubConnection) {
        hubConnection.send('LeaveGroup', groupName);
        removeEventListeners(hubConnection);
        hubConnection.stop();
      }
    };

    initHub();
    connect();

    return cleanup;
  }, [accessToken, baseUrl, groupName, platformId, setupEventListeners, removeEventListeners, getContainersList]);

  return { containersInfo, isLoading };
};

export default useContainersHub;
