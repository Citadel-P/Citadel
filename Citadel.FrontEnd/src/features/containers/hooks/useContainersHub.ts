import { useEffect, useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { configureHub, IHubConfig, startConnectionWithRetry } from '@/lib/signalr.helpers';
import { ContainerInfoView, ContainersInfoView } from '@/api/_generated';
import { useContextSelector } from 'use-context-selector';
import { AuthContext } from '@/features/auth/AuthProvider';

const useContainersHub = (platformId: string) => {
  const [containersInfo, setContainersInfo] = useState<ContainersInfoView | undefined>();
  const accessToken = useContextSelector(AuthContext, (v) => v?.accessToken);
  const groupName = `ContainersInfo/${platformId}`;
  const baseUrl = import.meta.env.VITE_API_BASE_URL;

  const handleContainersUpdated = useCallback((containers: ContainersInfoView) => {
    setContainersInfo(containers);
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

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.onreconnecting(() => console.log('Reconnecting...'));
      hubConnection.onreconnected(() => console.log('Reconnected.'));
      hubConnection.on('ContainersInfoUpdated', handleContainersUpdated);
      hubConnection.on('ContainerEventReceived', handleContainerEventReceived);
    },
    [handleContainersUpdated, handleContainerEventReceived],
  );

  const removeEventListeners = useCallback((hubConnection: HubConnection) => {
    hubConnection.off('ContainersInfoUpdated');
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
      hubConnection.send('JoinGroup', groupName).catch((error) => console.error('Failed to join group:', error));
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
  }, [accessToken, baseUrl, groupName, setupEventListeners, removeEventListeners]);

  return { containersInfo };
};

export default useContainersHub;
