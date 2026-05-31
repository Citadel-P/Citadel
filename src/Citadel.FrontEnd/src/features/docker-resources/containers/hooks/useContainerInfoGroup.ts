import { useCallback, useMemo, useState } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useDockerDaemonGroup, ContainerEvent } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { normalizeDockerId } from '@/lib/utils';
import { ContainerDataView, PlatformStatus, ProblemDetails } from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';

export const useContainerInfoGroup = (containerId?: string, platformId?: string) => {
  const nid = normalizeDockerId(containerId);

  const { data, isLoading } = useRead('getContainerData', { id: nid });
  const { currentPlatform } = useAppContext();

  const containerData = data?.data;

  const [liveContainerInfo, setLiveContainerInfo] = useState<Partial<ContainerDataView>>();

  const containerInfo = useMemo(
    () =>
      containerData || liveContainerInfo
        ? ({
            ...containerData,
            ...liveContainerInfo,
          } as ContainerDataView)
        : undefined,
    [containerData, liveContainerInfo],
  );

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

      if (!container.containerId.startsWith(nid)) {
        return;
      }

      if (eventType === 'destroy') {
        setLiveContainerInfo(undefined);
        return;
      }

      setLiveContainerInfo((current) => ({
        ...current,
        id: container.containerId,
        name: container.name,
        state: container.state,
        created: container.created as number,
        stack: container.stack,
        containerStat: container.lastStats ?? {},
        ports: container.ports as any,
        controlState: container.controlState,
        imageId: container.dockerImageId,
        // don't map capabilities here
      }));
    },
    [nid],
  );

  useDockerDaemonGroup(platformId, { onContainerEvent });

  const handleContainerInfoUpdated = useCallback((container: ContainerDataView) => {
    setLiveContainerInfo((current) => ({
      ...current,
      ...container,
    }));
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
    groupName: `container-info:${nid}`,
    setupEventListeners,
    removeEventListeners,
    skip: !nid,
  });

  return {
    containerInfo,
    isLoading,
    error,
  };
};
