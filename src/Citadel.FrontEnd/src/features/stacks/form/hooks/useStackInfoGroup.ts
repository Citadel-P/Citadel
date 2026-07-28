import { useCallback, useMemo, useState } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { ContainerDataView, PlatformStatus, ProblemDetails } from '@/api/generated/api.types';
import { useDockerDaemonGroup, ContainerEvent } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useAppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';
import { normalizeDockerId } from '@/lib/utils';

export const useStackInfoGroup = (stackId?: string, platformId?: string) => {
  const { data, isLoading } = useRead('getContainersData', { stackId });
  const { currentPlatform } = useAppContext();

  const containerData = useMemo(() => data?.data?.containers ?? [], [data?.data?.containers]);
  const [liveContainers, setLiveContainers] = useState<Record<string, Partial<ContainerDataView>>>({});
  const [removedContainerIds, setRemovedContainerIds] = useState<Set<string>>(() => new Set());

  const containersInfo = useMemo(() => {
    const merged = new Map<string, ContainerDataView>();

    containerData.forEach((container) => {
      const id = normalizeDockerId(container.id);
      if (!id) return;
      if (removedContainerIds.has(id)) return;

      merged.set(id, {
        ...container,
        ...liveContainers[id],
      });
    });

    Object.entries(liveContainers).forEach(([id, container]) => {
      if (!merged.has(id) && container.id) {
        merged.set(id, container as ContainerDataView);
      }
    });

    return Array.from(merged.values());
  }, [containerData, liveContainers, removedContainerIds]);

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

  const mergeContainers = useCallback((containers: Partial<ContainerDataView>[]) => {
    setLiveContainers((current) => {
      const next = { ...current };

      containers.forEach((container) => {
        const id = normalizeDockerId(container.id);
        if (!id) return;

        next[id] = {
          ...next[id],
          ...container,
        };
      });

      return next;
    });

    setRemovedContainerIds((current) => {
      const next = new Set(current);
      containers.forEach((container) => {
        const id = normalizeDockerId(container.id);
        if (id) next.delete(id);
      });
      return next;
    });
  }, []);

  const mergeContainerStats = useCallback((containers: ContainerDataView[]) => {
    setLiveContainers((current) => {
      const next = { ...current };

      containers.forEach((container) => {
        const id = normalizeDockerId(container.id);
        if (!id) return;

        next[id] = {
          ...next[id],
          containerStat: container.containerStat,
        };
      });

      return next;
    });
  }, []);

  const onContainerEvent = useCallback(
    (event: ContainerEvent) => {
      if (!stackId || event.container.stackId !== stackId) return;

      const id = normalizeDockerId(event.container.containerId);
      if (!id) return;

      if (event.eventType === 'destroy') {
        setLiveContainers((current) => {
          const next = { ...current };
          delete next[id];
          return next;
        });
        setRemovedContainerIds((current) => {
          const next = new Set(current);
          next.add(id);
          return next;
        });
        return;
      }

      mergeContainers([
        {
          id: event.container.containerId,
          name: event.container.name,
          imageId: event.container.dockerImageId,
          image: event.container.imageView?.name ?? '',
          state: event.container.state,
          created: event.container.created as number,
          stack: event.container.stack,
          containerStat: event.container.lastStats ?? undefined,
          ports: event.container.ports,
          controlState: event.container.controlState,
        },
      ]);
    },
    [mergeContainers, stackId],
  );

  useDockerDaemonGroup(platformId, { onContainerEvent });

  const handleStackContainersInfoUpdated = useCallback(
    (payload: ContainerDataView[] | { containers?: ContainerDataView[] }) => {
      const containers = Array.isArray(payload) ? payload : (payload.containers ?? []);
      mergeContainerStats(containers);
    },
    [mergeContainerStats],
  );

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('ReceiveStackContainersInfo', handleStackContainersInfoUpdated);
    },
    [handleStackContainersInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('ReceiveStackContainersInfo', handleStackContainersInfoUpdated);
    },
    [handleStackContainersInfoUpdated],
  );

  const { isLoading: isStreamLoading } = useSignalRGroup({
    groupName: `stack-info:${stackId}`,
    setupEventListeners,
    removeEventListeners,
    skip: !stackId,
  });

  return {
    containersInfo,
    isLoading: isLoading || isStreamLoading,
    error,
  };
};
