import { useEffect, useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { DockerContainerView } from '@/api/types';
import { useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';

export const useContainerInfoGroup = (containerId?: string, platformId?: string) => {
  const { containerEvent } = useDockerDaemonGroup(platformId);

  const [containerInfo, setContainerInfo] = useState<DockerContainerView | undefined>();

  const handleContainerInfoUpdated = useCallback((container: DockerContainerView) => {
    setContainerInfo(container);
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('ReceiveContainerInfo', handleContainerInfoUpdated);
    },
    [handleContainerInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('ReceiveContainerInfo', handleContainerInfoUpdated);
    },
    [handleContainerInfoUpdated],
  );

  useSignalRGroup({
    groupName: `container-info:${containerId}`,
    setupEventListeners,
    removeEventListeners,
    skip: !containerId,
  });

  useEffect(() => {
    if (
      containerId &&
      containerEvent?.container.containerId.startsWith(containerId) &&
      containerEvent?.eventType !== 'destroy'
    ) {
      const container: DockerContainerView = {
        containerId: containerEvent.container.containerId,
        name: containerEvent.container.name,
        state: containerEvent.container.state,
        created: containerEvent.container.created as number,
        image: containerEvent.container.image,
        stack: containerEvent.container.stack,
        containerStat: containerEvent.container.lastStats,
        containerPort: containerEvent.container.ports as any,
      };
      setContainerInfo(() => container);
    }
  }, [containerEvent, containerId]);

  return { containerInfo };
};
