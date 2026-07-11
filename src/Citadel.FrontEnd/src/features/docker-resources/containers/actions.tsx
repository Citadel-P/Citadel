import { Eye, Trash } from 'lucide-react';
import {
  ContainerView,
  ContainerStateStatus,
  ResourceControlState,
  StackReleaseStatus,
} from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { formatId } from '@/lib/utils';
import { useNavigate } from 'react-router';
import { useAppContext } from '@/lib/context/app-context';
import { CommandAction, ToggleAction } from '@/components/custom/actions-builder';
import { Ban, Pause, Play, RotateCcw, StepForward } from 'lucide-react';

interface BaseContainerResource {
  name: string;
  state: ContainerStateStatus;
  controlState: ResourceControlState;
}

export type ContainerStackGroupResource = {
  id: string;
  platformId: string;
  name: string;
  containerId: string;
  state: ContainerStateStatus;
  controlState: ResourceControlState;
  stackId: string | null;
  stack: string | null;
  lastStats: ContainerView['lastStats'];
  ports: ContainerView['ports'];
  deploymentId: null;
  imageView?: null;
  displayStatus: StackReleaseStatus;
  capabilities?: ContainerView['capabilities'];
  containers: ContainerView[];
  isStackGroup: true;
};

export type ContainerActionResource = ContainerView | ContainerStackGroupResource;

export const isContainerStackGroup = (resource: ContainerActionResource): resource is ContainerStackGroupResource =>
  'isStackGroup' in resource && resource.isStackGroup;

type ContainerActionKey = 'start' | 'stop' | 'pause' | 'unpause' | 'restart';
type ContainerVariablesFactory<T extends BaseContainerResource> = (
  resources: T | T[],
  action: ContainerActionKey,
) => any;

const isProcessing = (r: BaseContainerResource) => r.controlState === ResourceControlState.Processing;

const canStart = (x: BaseContainerResource) =>
  x.state !== ContainerStateStatus.Running &&
  x.state !== ContainerStateStatus.Offline &&
  x.state !== ContainerStateStatus.Paused &&
  !isProcessing(x);

const canStop = (x: BaseContainerResource) =>
  (x.state === ContainerStateStatus.Running || x.state === ContainerStateStatus.Paused) && !isProcessing(x);

const canPause = (x: BaseContainerResource) => x.state === ContainerStateStatus.Running && !isProcessing(x);

const canUnpause = (x: BaseContainerResource) => x.state === ContainerStateStatus.Paused && !isProcessing(x);

const canRestart = (x: BaseContainerResource) =>
  (x.state === ContainerStateStatus.Running || x.state === ContainerStateStatus.Paused) && !isProcessing(x);

export const createContainerActions = <T extends BaseContainerResource>(
  useVariables: ContainerVariablesFactory<T>,
  getActionTargets: (resource: T, action: ContainerActionKey) => BaseContainerResource[] = (resource) => [resource],
) => {
  const canExecute = (
    resources: T | T[],
    action: ContainerActionKey,
    predicate: (resource: BaseContainerResource) => boolean,
  ) => {
    const selected = Array.isArray(resources) ? resources : [resources];
    return selected.every((resource) => getActionTargets(resource, action).some(predicate));
  };

  const startAction: CommandAction<T, 'startContainers'> = {
    key: 'start',
    type: 'command',
    icon: Play,
    mutateKey: 'startContainers',
    useVariables: (r) => useVariables(r, 'start'),
    canExecute: (r) => canExecute(r, 'start', canStart),
  };

  const stopAction: CommandAction<T, 'stopContainers'> = {
    key: 'stop',
    type: 'command',
    icon: Ban,
    mutateKey: 'stopContainers',
    useVariables: (r) => useVariables(r, 'stop'),
    canExecute: (r) => canExecute(r, 'stop', canStop),
  };

  const pauseAction: ToggleAction<T, 'pauseContainers' | 'unpauseContainers'> = {
    key: 'pauseToggle',
    type: 'toggle',
    primary: {
      title: 'Pause',
      icon: Pause,
      mutateKey: 'pauseContainers',
      useVariables: (r) => useVariables(r, 'pause'),
      canExecute: (r) => canExecute(r, 'pause', canPause),
    },
    secondary: {
      title: 'Resume',
      icon: StepForward,
      mutateKey: 'unpauseContainers',
      useVariables: (r) => useVariables(r, 'unpause'),
      canExecute: (r) => canExecute(r, 'unpause', canUnpause),
    },
  };

  const restartAction: CommandAction<T, 'restartContainers'> = {
    key: 'restart',
    type: 'command',
    icon: RotateCcw,
    mutateKey: 'restartContainers',
    useVariables: (r) => useVariables(r, 'restart'),
    canExecute: (r) => canExecute(r, 'restart', canRestart),
  };

  return { startAction, stopAction, pauseAction, restartAction };
};

const getContainers = (resources: ContainerActionResource | ContainerActionResource[]) => {
  const selected = Array.isArray(resources) ? resources : [resources];
  return selected.flatMap((resource) => (isContainerStackGroup(resource) ? resource.containers : [resource]));
};

const eligibleFor = (action: ContainerActionKey) =>
  ({
    start: canStart,
    stop: canStop,
    pause: canPause,
    unpause: canUnpause,
    restart: canRestart,
  })[action];

const useVariables = (resources: ContainerActionResource | ContainerActionResource[], action: ContainerActionKey) =>
  Array.from(
    new Set(
      getContainers(resources)
        .filter(eligibleFor(action))
        .map((r) => r.containerId),
    ),
  );

const getActionTargets = (resource: ContainerActionResource) =>
  isContainerStackGroup(resource) ? resource.containers : [resource];

const { startAction, stopAction, pauseAction, restartAction } = createContainerActions<ContainerActionResource>(
  useVariables,
  getActionTargets,
);

export const { dropdown: ContainerDropdownActions, group: ContainerGroupActions } =
  createActionsBuilder<ContainerActionResource>()
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
        const canOpenDetails = canExecute && (!isContainerStackGroup(selected) || !!selected.stackId);
        return {
          canExecute: canOpenDetails,
          isPending: false,
          run: () => {
            if (!canOpenDetails || !selected) return;
            if (isContainerStackGroup(selected) && selected.stackId) {
              navigate(`/stacks/edit/${selected.stackId}`);
              return;
            }
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
        const containers = getContainers(r);
        return containers.some(can);
      },
      separatorBefore: true,
      confirm: true,
      destructive: true,
      resourceType: 'Container',
      useVariables: (resources) => {
        const can = (x: ContainerView) => x.state !== ContainerStateStatus.Offline && !isProcessing(x);
        const containerIds = Array.from(
          new Set(
            getContainers(resources)
              .filter(can)
              .map((r) => r.containerId),
          ),
        );
        return { force: true, containerIds };
      },
    })
    .build();
