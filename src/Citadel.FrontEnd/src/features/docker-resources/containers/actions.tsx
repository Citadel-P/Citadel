import { Eye, Trash } from 'lucide-react';
import { ContainerView, ContainerStateStatus, ResourceControlState } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { formatId } from '@/lib/utils';
import { useNavigate } from 'react-router';
import { useAppContext } from '@/lib/context/app-context';
import { CommandAction, ToggleAction } from '@/components/custom/actions-builder';
import { Ban, Pause, Play, RotateCcw, StepForward } from 'lucide-react';

interface BaseContainerResource {
  state: ContainerStateStatus;
  controlState: ResourceControlState;
}

const isProcessing = (r: BaseContainerResource) => r.controlState === ResourceControlState.Processing;

export const createContainerActions = <T extends BaseContainerResource>(useVariables: (r: T | T[]) => any) => {
  const startAction: CommandAction<T, 'startContainers'> = {
    key: 'start',
    type: 'command',
    icon: Play,
    mutateKey: 'startContainers',
    useVariables,
    canExecute: (r) => {
      const can = (x: BaseContainerResource) =>
        x.state !== ContainerStateStatus.Running &&
        x.state !== ContainerStateStatus.Offline &&
        x.state !== ContainerStateStatus.Paused &&
        !isProcessing(x);
      return Array.isArray(r) ? r.every(can) : can(r);
    },
  };

  const stopAction: CommandAction<T, 'stopContainers'> = {
    key: 'stop',
    type: 'command',
    icon: Ban,
    mutateKey: 'stopContainers',
    useVariables,
    canExecute: (r) => {
      const can = (x: BaseContainerResource) =>
        (x.state === ContainerStateStatus.Running || x.state === ContainerStateStatus.Paused) && !isProcessing(x);
      return Array.isArray(r) ? r.every(can) : can(r);
    },
  };

  const pauseAction: ToggleAction<T, 'pauseContainers' | 'unpauseContainers'> = {
    key: 'pauseToggle',
    type: 'toggle',
    primary: {
      title: 'Pause',
      icon: Pause,
      mutateKey: 'pauseContainers',
      useVariables,
      canExecute: (r) => {
        const can = (x: BaseContainerResource) => x.state === ContainerStateStatus.Running && !isProcessing(x);
        return Array.isArray(r) ? r.every(can) : can(r);
      },
    },
    secondary: {
      title: 'Resume',
      icon: StepForward,
      mutateKey: 'unpauseContainers',
      useVariables,
      canExecute: (r) => {
        const can = (x: BaseContainerResource) => x.state === ContainerStateStatus.Paused && !isProcessing(x);
        return Array.isArray(r) ? r.every(can) : can(r);
      },
    },
  };

  const restartAction: CommandAction<T, 'restartContainers'> = {
    key: 'restart',
    type: 'command',
    icon: RotateCcw,
    mutateKey: 'restartContainers',
    useVariables,
    canExecute: (r) => {
      const can = (x: BaseContainerResource) =>
        (x.state === ContainerStateStatus.Running || x.state === ContainerStateStatus.Paused) && !isProcessing(x);
      return Array.isArray(r) ? r.every(can) : can(r);
    },
  };

  return { startAction, stopAction, pauseAction, restartAction };
};
const useVariables = (resources: ContainerView | ContainerView[]) =>
  Array.isArray(resources) ? resources.map((r) => r.containerId) : [resources.containerId];

const { startAction, stopAction, pauseAction, restartAction } = createContainerActions(useVariables);

export const { dropdown: ContainerDropdownActions, group: ContainerGroupActions } =
  createActionsBuilder<ContainerView>()
    .addAction(startAction)
    .addAction(stopAction)
    .addAction(pauseAction)
    .addAction(restartAction)
    .addAction({
      key: 'details',
      type: 'command',
      separatorBefore: true,
      icon: Eye,
      requiredCapabilities: ['canInspect'],
      useHandler: ({ resources }) => {
        const navigate = useNavigate();
        const { currentPlatform } = useAppContext();
        const selected = Array.isArray(resources) ? resources[0] : resources;
        let canExecute = !!selected;
        if (Array.isArray(resources)) {
          canExecute &&= resources.length === 1;
        }
        return {
          canExecute,
          isPending: false,
          run: () => {
            if (!canExecute || !selected) return;
            navigate(`/platforms/${currentPlatform?.id}/containers/${formatId(selected.containerId)}`);
          },
        };
      },
    })
    .addAction({
      key: 'delete',
      type: 'command',
      icon: Trash,
      mutateKey: 'deleteContainers',
      canExecute: (r) => {
        const can = (x: ContainerView) => x.state !== ContainerStateStatus.Offline && !isProcessing(x);
        return Array.isArray(r) ? r.some(can) : can(r);
      },
      separatorBefore: true,
      confirm: true,
      destructive: true,
      resourceType: 'Container',
      useVariables: (resources) => {
        const selected = Array.isArray(resources) ? resources : [resources];
        return { force: true, containerIds: selected.map((r) => r.containerId) };
      },
    })
    .build();
