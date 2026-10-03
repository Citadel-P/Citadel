import { useCallback, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useDockerDaemonGroup, ContainerEvent } from '@/features/platforms/hooks/useDockerDaemonGroup';
import type { ContainerStatePatch } from './container-order';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { normalizeContainerReference, normalizeDockerId } from '@/lib/utils';
import {
  type ContainerRuntimeView,
  type ContainerView,
  PlatformStatus,
  type ProblemDetails,
} from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';

export type ContainerDetailsView = ContainerRuntimeView & {
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
  current: Partial<ContainerRuntimeView> | undefined,
  container: ContainerRuntimeView,
): Partial<ContainerRuntimeView> => ({
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

  const {
    data,
    isLoading,
    error: queryError,
    refetch,
    isFetching,
  } = useRead('getContainer', { id: containerReference });
  const { currentPlatform } = useAppContext();

  const containerData = useMemo(() => (data?.data ? toContainerDetailsView(data.data) : undefined), [data]);
  const dockerContainerId = normalizeDockerId(containerData?.id);

  const identity = JSON.stringify([
    platformId,
    containerReference,
    containerData?.resourceId,
    containerData?.dockerNodeId,
  ]);
  const currentIdentity = useRef(identity);
  useLayoutEffect(() => {
    currentIdentity.current = identity;
  }, [identity]);
  const [live, setLive] = useState<{ identity: string; value: Partial<ContainerRuntimeView> | undefined }>();
  const liveContainerInfo = live?.identity === identity ? live.value : undefined;
  const setLiveContainerInfo = useCallback(
    (
      update:
        | Partial<ContainerRuntimeView>
        | undefined
        | ((current: Partial<ContainerRuntimeView> | undefined) => Partial<ContainerRuntimeView>),
    ) => {
      if (currentIdentity.current !== identity) return;
      setLive((previous) => ({
        identity,
        value:
          typeof update === 'function' ? update(previous?.identity === identity ? previous.value : undefined) : update,
      }));
    },
    [identity],
  );

  const containerInfo = useMemo(
    () =>
      containerData || liveContainerInfo
        ? ({
            ...containerData,
            ...liveContainerInfo,
            resourceId: containerData?.resourceId,
            platformId: containerData?.platformId,
            id: containerData?.id,
          } as ContainerDetailsView)
        : undefined,
    [containerData, liveContainerInfo],
  );

  const error = useMemo(() => {
    if (currentPlatform?.status === PlatformStatus.Offline) {
      return {
        error: {
          detail: 'Platform is disconnected or unavailable.',
          status: 503,
          title: 'Platform unavailable',
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
        containerStat: container.lastStats ?? undefined,
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
    [containerData?.dockerNodeId, dockerContainerId, setLiveContainerInfo],
  );

  const onContainerStateChange = useCallback(
    (patches: ContainerStatePatch[]) => {
      if (!dockerContainerId) return;
      const patch = patches.find(
        (item) =>
          item.containerId.toLowerCase().startsWith(dockerContainerId.toLowerCase()) &&
          (item.dockerNodeId ?? undefined) === (containerData?.dockerNodeId ?? undefined),
      );
      if (!patch) return;
      setLiveContainerInfo((current) => ({
        ...current,
        ...(patch.state !== undefined ? { state: patch.state } : {}),
        ...(patch.controlState !== undefined ? { controlState: patch.controlState } : {}),
        ...(patch.updated !== undefined ? { updated: patch.updated } : {}),
      }));
    },
    [containerData?.dockerNodeId, dockerContainerId, setLiveContainerInfo],
  );

  useDockerDaemonGroup(platformId, { onContainerEvent, onContainerStateChange });

  const handleContainerInfoUpdated = useCallback(
    (container: ContainerRuntimeView) => {
      if (
        normalizeDockerId(container.id) !== dockerContainerId ||
        (container.dockerNodeId ?? undefined) !== (containerData?.dockerNodeId ?? undefined)
      )
        return;
      setLiveContainerInfo((current) => mergeContainerRuntimeUpdate(current, container));
    },
    [dockerContainerId, containerData?.dockerNodeId, setLiveContainerInfo],
  );

  const handleContainerInfoPatch = useCallback(
    (patch: Partial<ContainerRuntimeView> & { resourceId: string }) => {
      if (patch.resourceId !== containerData?.resourceId) return;
      setLiveContainerInfo((current) => ({ ...current, ...patch }));
    },
    [containerData?.resourceId, setLiveContainerInfo],
  );

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('ReceiveContainerInfo', handleContainerInfoUpdated);
      hubConnection.on('ReceiveContainerInfoPatch', handleContainerInfoPatch);
    },
    [handleContainerInfoUpdated, handleContainerInfoPatch],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('ReceiveContainerInfo', handleContainerInfoUpdated);
      hubConnection.off('ReceiveContainerInfoPatch', handleContainerInfoPatch);
    },
    [handleContainerInfoUpdated, handleContainerInfoPatch],
  );

  useRealtimeGroup({
    groupName: containerData?.resourceId ? `container-info:${containerData.resourceId}` : undefined,
    setupEventListeners,
    removeEventListeners,
    skip: !containerData?.resourceId,
  });

  return { refetch, isFetching, containerInfo, isLoading, error: queryError ?? error };
};
