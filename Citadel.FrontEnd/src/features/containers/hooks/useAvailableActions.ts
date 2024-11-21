import { ContainerInfoView } from '@/api/_generated';
import { actionType, usePATCHContainers } from './usePATCHContainers';

export const useAvailableActions = (containers: ContainerInfoView[]) => {
  const { mutate, isPending } = usePATCHContainers();

  const actions: ContainerActionsState = {
    canStart: false,
    canStop: false,
    canRestart: false,
    canPause: false,
    canDelete: containers.length > 0,
  };

  containers.forEach((container) => {
    actions.canStart ||= container.state === 'exited';
    actions.canPause ||= container.state === 'running';
    actions.canStop ||= container.state === 'running' || container.state === 'paused';
    actions.canRestart ||= container.state === 'running' || container.state === 'paused';
  });

  const requestPatch = (action: actionType) => {
    if (isPending) return;
    mutate({
      action,
      containersId: containers.map((s) => s.containerId!),
    });
  };

  return { availableActions: actions, requestPatch, isPending };
};

type ContainerActionsState = {
  canStart: boolean;
  canStop: boolean;
  canRestart: boolean;
  canPause: boolean;
  canDelete: boolean;
};
