import { useCallback, useMemo, useState } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useDockerDaemonGroup, ContainerEvent } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { normalizeDockerId } from '@/lib/utils';
import { DockerContainerView } from '@/api/types';
import { PlatformStatus, ProblemDetails } from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';

export const useContainerInfoGroup = (containerId?: string, platformId?: string) => {
  const nid = normalizeDockerId(containerId);
  const { currentPlatform } = useAppContext();
  const [containerInfo, setContainerInfo] = useState<DockerContainerView | undefined>();

  const error = useMemo(() => {
    if (currentPlatform?.status === PlatformStatus.Offline) {
      return {
        error: {
          detail: 'Platform is disconnected or unavailable.',
          status: 404,
          title: 'Not Found',
        } as ProblemDetails,
      };
    }
    return undefined;
  }, [currentPlatform?.status]);

  const onContainerEvent = useCallback(
    (event: ContainerEvent) => {
      if (!nid) return;

      const { container, eventType } = event;

      if (container.containerId.startsWith(nid)) {
        if (eventType === 'destroy') {
          setContainerInfo(undefined);
        } else {
          const updatedInfo: DockerContainerView = {
            id: container.containerId,
            name: container.name,
            state: container.state,
            created: container.created as number,
            stack: container.stack,
            containerStat: container.lastStats ?? {},
            containerPort: container.ports as any,
            controlState: container.controlState,
          };
          setContainerInfo(updatedInfo);
        }
      }
    },
    [nid],
  );

  useDockerDaemonGroup(platformId, { onContainerEvent });

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

  return { containerInfo, isLoading, error };
};
