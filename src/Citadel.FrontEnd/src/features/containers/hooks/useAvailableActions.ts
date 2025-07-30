import { useMemo } from 'react';
import { ContainerView, ContainerStateStatus } from '@/api/_generated';
import { actionType, usePATCHContainers } from './usePATCHContainers';

export const useAvailableActions = (containers: ContainerView[] | undefined) => {
  const { mutate, isPending } = usePATCHContainers();

  // Calculate available actions using useMemo
  const availableActions = useMemo<ContainerActionsState | undefined>(() => {
    return containers?.reduce<ContainerActionsState>(
      (actions, container) => {
        const isRunningOrPaused =
          container.state === ContainerStateStatus.Running || container.state === ContainerStateStatus.Paused;
        const canStart =
          container.state !== ContainerStateStatus.Running &&
          container.state !== ContainerStateStatus.Offline &&
          container.state !== ContainerStateStatus.Paused;
        const canDelete = container.state !== ContainerStateStatus.Offline;

        return {
          canStart: actions.canStart || canStart,
          canStop: actions.canStop || isRunningOrPaused,
          canRestart: actions.canRestart || isRunningOrPaused,
          canPause: actions.canPause || container.state === ContainerStateStatus.Running,
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

  // Request a patch action
  const requestPatch = (action: actionType) => {
    if (isPending) return; // Prevent duplicate requests if a mutation is already pending
    mutate({
      action,
      containersId: containers?.map((s) => s.containerId!).filter(Boolean) ?? [],
    });
  };

  return { availableActions, requestPatch, isPending };
};

type ContainerActionsState = {
  canStart: boolean;
  canStop: boolean;
  canRestart: boolean;
  canPause: boolean;
  canDelete: boolean;
};
