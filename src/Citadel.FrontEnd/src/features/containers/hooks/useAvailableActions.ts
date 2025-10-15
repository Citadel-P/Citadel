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
          canDelete: actions.canDelete || canDelete,
        };
      },
      {
        canStart: false,
        canStop: false,
        canRestart: false,
        canPause: false,
        canDelete: false,
      },
    );
  }, [containers]);

  const requestPatch = (action: actionType) => {
    if (isPending) return;
    const containersId = containers?.map((s) => s.containerId!).filter(Boolean) ?? [];

    const mutation = mutations[action];
    if (!mutation) throw new Error(`Unsupported action: ${action}`);

    mutation.mutate({ data: containersId });
  };

  return { availableActions, requestPatch, isPending };
};

type actionType = 'start' | 'stop' | 'pause' | 'restart';
type ContainerActionsState = {
  canStart: boolean;
  canStop: boolean;
  canRestart: boolean;
  canPause: boolean;
  canDelete: boolean;
};
