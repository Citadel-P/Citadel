import { useMemo } from 'react';
import { ContainerView, ContainerStateStatus } from '@/api/generated/api.types';
import { DockerContainerView } from '@/api/types';
import { useMutate } from '@/lib/hooks';

export const useAvailableActions = (containers: (ContainerView | DockerContainerView)[] | undefined) => {
  const mutations = {
    start: useMutate('startContainers'),
    stop: useMutate('stopContainers'),
    restart: useMutate('restartContainers'),
    pause: useMutate('pauseContainers'),
    unpause: useMutate('unpauseContainers'),
  };

  const isPending = Object.values(mutations).some((m) => m.isPending);

  const availableActions = useMemo<ContainerActionsState | undefined>(() => {
    if (!containers) return undefined;

    return containers.reduce<ContainerActionsState>(
      (actions, container) => {
        const { state } = container;
        const isRunningOrPaused = state === ContainerStateStatus.Running || state === ContainerStateStatus.Paused;

        const canStart =
          state !== ContainerStateStatus.Running &&
          state !== ContainerStateStatus.Offline &&
          state !== ContainerStateStatus.Paused;

        const canDelete = state !== ContainerStateStatus.Offline;

        return {
          canStart: actions.canStart || canStart,
          canStop: actions.canStop || isRunningOrPaused,
          canRestart: actions.canRestart || isRunningOrPaused,
          canPause: actions.canPause || state === ContainerStateStatus.Running,
          canUnpause: actions.canUnpause || state === ContainerStateStatus.Paused,
          canDelete: actions.canDelete || canDelete,
        };
      },
      {
        canStart: false,
        canStop: false,
        canRestart: false,
        canPause: false,
        canUnpause: false,
        canDelete: false,
      },
    );
  }, [containers]);

  const requestPatch = (action: actionType) => {
    if (isPending) return;
    const containersId = containers?.map((s) => s.containerId!).filter(Boolean) ?? [];

    const mutation = mutations[action];
    if (!mutation) throw new Error(`Unsupported action: ${action}`);

    if (containersId.length > 0) mutation.mutate({ data: containersId });
  };

  return { availableActions, requestPatch, isPending };
};

type actionType = 'start' | 'stop' | 'pause' | 'restart' | 'unpause';
type ContainerActionsState = {
  canStart: boolean;
  canStop: boolean;
  canRestart: boolean;
  canPause: boolean;
  canUnpause: boolean;
  canDelete: boolean;
};
