import { useEffect, useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { DockerContainerView } from '@/api/types';
import { useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { normalizeDockerId } from '@/lib/utils';

export const useContainerInfoGroup = (containerId?: string, platformId?: string) => {
  const nid = normalizeDockerId(containerId);

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

  const { isLoading } = useSignalRGroup({
    groupName: `container-info:${nid}`,
    setupEventListeners,
    removeEventListeners,
    skip: !nid,
  });

  useEffect(() => {
    if (nid && 
      containerEvent?.container.containerId.startsWith(nid) && 
      containerEvent?.eventType !== 'destroy'
    ) {
      const container: DockerContainerView = {
        id: containerEvent.container.containerId,
        name: containerEvent.container.name,
        state: containerEvent.container.state,
        created: containerEvent.container.created as number,
        stack: containerEvent.container.stack,
        containerStat: containerEvent.container.lastStats ?? {},
        containerPort: containerEvent.container.ports as any,
      };
      setContainerInfo(container);
    }
  }, [containerEvent, nid]);

  return { containerInfo, isLoading };
};
