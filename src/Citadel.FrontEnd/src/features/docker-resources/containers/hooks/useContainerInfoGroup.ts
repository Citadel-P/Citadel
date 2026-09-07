import { useCallback, useMemo, useState } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useDockerDaemonGroup, ContainerEvent } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { normalizeContainerReference, normalizeDockerId } from '@/lib/utils';
import {
  type ContainerDataView,
  type ContainerView,
  PlatformStatus,
  type ProblemDetails,
} from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';

export type ContainerDetailsView = ContainerDataView & {
  resourceId: string;
  platformId: string;
};

export const toContainerDetailsView = (container: ContainerView): ContainerDetailsView => ({
  name: container.name,
  image: container.imageView?.name ?? '',
  id: container.containerId,
  imageId: container.dockerImageId,
  state: container.state,
  controlState: container.controlState,
  isSystem: container.isSystem,
  systemRole: container.systemRole,
  hasCitadelOwnershipLabels: container.hasCitadelOwnershipLabels,
  isSwarmTask: container.isSwarmTask,
  dockerNodeId: container.dockerNodeId,
  created: container.created,
  stack: container.stack,
  containerStat: container.lastStats,
  ports: container.ports,
  deploymentId: container.deploymentId,
  stackId: container.stackId,
  capabilities: container.capabilities,
  resourceId: container.id,
  platformId: container.platformId,
});

export const mergeContainerRuntimeUpdate = (
  current: Partial<ContainerDataView> | undefined,
  container: ContainerDataView,
): Partial<ContainerDataView> => ({
  ...current,
  id: container.id,
  name: container.name,
  image: container.image,
  imageId: container.imageId,
  state: container.state,
  dockerNodeId: container.dockerNodeId ?? current?.dockerNodeId,
  created: container.created,
  stack: container.stack,
  containerStat: container.containerStat,
  ports: container.ports,
});

export const useContainerInfoGroup = (containerId?: string, platformId?: string) => {
  const containerReference = normalizeContainerReference(containerId);

  const { data, isLoading } = useRead('getContainer', { id: containerReference });
  const { currentPlatform } = useAppContext();

  const containerData = useMemo(() => (data?.data ? toContainerDetailsView(data.data) : undefined), [data]);
  const dockerContainerId = normalizeDockerId(containerData?.id);

  const [liveContainerInfo, setLiveContainerInfo] = useState<Partial<ContainerDataView>>();

  const containerInfo = useMemo(
    () =>
      containerData || liveContainerInfo
        ? ({
            ...containerData,
            ...liveContainerInfo,
          } as ContainerDetailsView)
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
      if (!dockerContainerId) return;

      const { container, eventType } = event;

      if (!container.containerId.toLowerCase().startsWith(dockerContainerId)) {
        return;
      }
      if (containerData?.dockerNodeId !== container.dockerNodeId) {
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
        isSystem: container.isSystem,
        systemRole: container.systemRole,
        hasCitadelOwnershipLabels: container.hasCitadelOwnershipLabels,
        isSwarmTask: container.isSwarmTask,
        dockerNodeId: container.dockerNodeId,
        // don't map capabilities here
      }));
    },
    [containerData?.dockerNodeId, dockerContainerId],
  );

  useDockerDaemonGroup(platformId, { onContainerEvent });

  const handleContainerInfoUpdated = useCallback((container: ContainerDataView) => {
    setLiveContainerInfo((current) => mergeContainerRuntimeUpdate(current, container));
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('ReceiveContainerInfo', handleContainerInfoUpdated);
    },
    [handleContainerInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('ReceiveContainerInfo', handleContainerInfoUpdated);
    },
    [handleContainerInfoUpdated],
  );

  useRealtimeGroup({
    groupName: containerInfo?.resourceId ? `container-info:${containerInfo.resourceId}` : undefined,
    setupEventListeners,
    removeEventListeners,
    skip: !containerInfo?.resourceId,
  });

  return {
    containerInfo,
    isLoading,
    error,
  };
};
