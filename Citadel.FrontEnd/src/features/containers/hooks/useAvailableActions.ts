import { useMemo } from 'react';
import { ContainerInfoView } from '@/api/_generated';
import { actionType, usePATCHContainers } from './usePATCHContainers';

export const useAvailableActions = (containers: ContainerInfoView[]) => {
  const { mutate, isPending } = usePATCHContainers();

  // Calculate available actions using useMemo
  const availableActions = useMemo<ContainerActionsState>(() => {
    return containers.reduce<ContainerActionsState>(
      (actions, container) => {
        const isRunningOrPaused = container.state === 'running' || container.state === 'paused';
        const canStart = container.state !== 'running' && container.state !== 'offline' && container.state !== 'paused';
        const canDelete = container.state !== 'offline';

        return {
          canStart: actions.canStart || canStart,
          canStop: actions.canStop || isRunningOrPaused,
          canRestart: actions.canRestart || isRunningOrPaused,
          canPause: actions.canPause || container.state === 'running',
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
      containersId: containers.map((s) => s.containerId!).filter(Boolean), // Ensure containerId is not null or undefined
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
