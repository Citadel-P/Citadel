import { Eye, FolderInput, PackagePlus, Trash } from 'lucide-react';
import {
  type ContainerView,
  type ContainerDataView,
  ContainerStateStatus,
  PlatformType,
  StackImportKind,
  ResourceControlState,
  StackReleaseStatus,
} from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { isUnmanagedContainer } from '@/lib/utils';
import { useNavigate } from 'react-router';
import { useAppContext } from '@/lib/context/app-context';
import { CommandAction, ToggleAction } from '@/components/custom/actions-builder';
import { Ban, Pause, Play, RotateCcw, StepForward } from 'lucide-react';

interface BaseContainerResource {
  name: string;
  state: ContainerStateStatus;
  controlState: ResourceControlState;
  isSystem: boolean;
  isSwarmTask?: boolean;
  projectionStaleSince?: number | string | null;
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
  isSystem: boolean;
  systemRole: null;
  isSwarmTask: boolean;
  imageView?: null;
  displayStatus: StackReleaseStatus;
  capabilities?: ContainerView['capabilities'];
  projectionStaleSince?: number | string | null;
  containers: ContainerView[];
  isStackGroup: true;
};

export type ContainerActionResource = ContainerView | ContainerStackGroupResource;

export const isContainerStackGroup = (
  resource: ContainerActionResource | ContainerDataView,
): resource is ContainerStackGroupResource => 'isStackGroup' in resource && resource.isStackGroup;

type ContainerActionKey = 'start' | 'stop' | 'pause' | 'unpause' | 'restart';
type ContainerVariablesFactory<T extends BaseContainerResource> = (
  resources: T | T[],
  action: ContainerActionKey,
) => any;

const isProcessing = (r: BaseContainerResource) => r.controlState === ResourceControlState.Processing;
const isStale = (r: BaseContainerResource) => Boolean(r.projectionStaleSince);

const canStart = (x: BaseContainerResource) =>
  !x.isSystem &&
  !x.isSwarmTask &&
  !isStale(x) &&
  x.state !== ContainerStateStatus.Running &&
  x.state !== ContainerStateStatus.Offline &&
  x.state !== ContainerStateStatus.Paused &&
  !isProcessing(x);

const canStop = (x: BaseContainerResource) =>
  !x.isSystem &&
  !x.isSwarmTask &&
  !isStale(x) &&
  (x.state === ContainerStateStatus.Running || x.state === ContainerStateStatus.Paused) &&
  !isProcessing(x);

const canPause = (x: BaseContainerResource) =>
  !x.isSystem && !x.isSwarmTask && !isStale(x) && x.state === ContainerStateStatus.Running && !isProcessing(x);

const canUnpause = (x: BaseContainerResource) =>
  !x.isSystem && !x.isSwarmTask && !isStale(x) && x.state === ContainerStateStatus.Paused && !isProcessing(x);

const canRestart = (x: BaseContainerResource) =>
  !x.isSystem &&
  !x.isSwarmTask &&
  !isStale(x) &&
  (x.state === ContainerStateStatus.Running || x.state === ContainerStateStatus.Paused) &&
  !isProcessing(x);

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
    if (selected.some((resource) => getActionTargets(resource, action).some((target) => target.isSystem))) {
      return false;
    }
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

export const isAdoptableContainer = (resource: ContainerActionResource | ContainerDataView) =>
  !isContainerStackGroup(resource) && !resource.stack && isUnmanagedContainer(resource);

export const isContainerAdoptionAvailable = (
  resource: ContainerActionResource | ContainerDataView,
  platformType?: PlatformType,
) => platformType === PlatformType.Docker && isAdoptableContainer(resource);

export const isImportableStack = (resource: ContainerActionResource | ContainerDataView) => {
  const containers = isContainerStackGroup(resource) ? resource.containers : [resource];
  return (
    !!resource.stack &&
    !resource.stackId &&
    containers.length > 0 &&
    containers.every((container) => !container.isSystem && !container.deploymentId && !container.stackId)
  );
};

export const isImportableStackSelection = (resources: ContainerActionResource[]) => {
  const first = resources[0];
  if (!first?.stack) return false;

  return resources.every(
    (resource) =>
      resource.platformId === first.platformId && resource.stack === first.stack && isImportableStack(resource),
  );
};

export const isSwarmStackGroup = (resource: ContainerActionResource | ContainerDataView) =>
  isContainerStackGroup(resource) &&
  resource.containers.length > 0 &&
  resource.containers.every((container) => container.isSwarmTask);

const getStackImportKind = (resources: ContainerActionResource | ContainerActionResource[]) => {
  const selection = Array.isArray(resources) ? resources : [resources];
  const containers = selection.flatMap((resource) =>
    isContainerStackGroup(resource) ? resource.containers : [resource],
  );
  return containers.length > 0 && containers.every((container) => container.isSwarmTask)
    ? StackImportKind.SwarmStack
    : StackImportKind.ComposeProject;
};

export const containsSystemContainer = (resources: { isSystem: boolean } | { isSystem: boolean }[]) => {
  const selected = Array.isArray(resources) ? resources : [resources];
  return selected.some((resource) => resource.isSystem);
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
        .map((r) => r.id),
    ),
  );

const getActionTargets = (resource: ContainerActionResource) =>
  isContainerStackGroup(resource) ? resource.containers : [resource];

const { startAction, stopAction, pauseAction, restartAction } = createContainerActions<ContainerActionResource>(
  useVariables,
  getActionTargets,
);

const { dropdown: baseContainerDropdownActions, group: baseContainerGroupActions } =
  createActionsBuilder<ContainerActionResource>()
    .addAction(startAction)
    .addAction(stopAction)
    .addAction(pauseAction)
    .addAction(restartAction)
    .addAction({
      key: 'adopt',
      title: 'Adopt Container',
      type: 'command',
      separatorBefore: true,
      icon: PackagePlus,
      requiredCapabilities: ['canInspect'],
      useHandler: ({ resources }) => {
        const navigate = useNavigate();
        const { currentPlatform } = useAppContext();
        const selected = Array.isArray(resources) ? resources[0] : resources;
        const canExecute =
          !!selected && isContainerAdoptionAvailable(selected, currentPlatform?.type) && !isProcessing(selected);

        return {
          canExecute,
          isPending: false,
          run: () => {
            if (!canExecute || !selected || isContainerStackGroup(selected)) return;
            navigate(`/deployments/add?adoptFrom=${selected.id}`);
          },
        };
      },
    })
    .addAction({
      key: 'importStack',
      title: 'Import Stack',
      type: 'command',
      separatorBefore: true,
      icon: FolderInput,
      requiredCapabilities: ['canInspect'],
      useHandler: ({ resources }) => {
        const navigate = useNavigate();
        const { currentPlatform } = useAppContext();
        const selection = Array.isArray(resources) ? resources : [resources];
        const selected = selection[0];
        const projectName = selected?.stack;
        const projectContainers = selection.flatMap((resource) =>
          isContainerStackGroup(resource) ? resource.containers : [resource],
        );
        const canExecute =
          (currentPlatform?.type === PlatformType.Docker || currentPlatform?.type === PlatformType.DockerSwarm) &&
          !!selected &&
          isImportableStackSelection(selection) &&
          !isProcessing(selected) &&
          projectContainers.every((container) => !isProcessing(container));

        return {
          canExecute,
          isPending: false,
          run: () => {
            if (!canExecute || !selected || !projectName) return;
            const query = new URLSearchParams({
              importPlatform: selected.platformId,
              importProject: projectName,
              importKind: getStackImportKind(selection),
            });
            navigate(`/stacks/add?${query.toString()}`);
          },
        };
      },
    })
    .addAction({
      key: 'details',
      type: 'command',
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
            navigate(`/platforms/${currentPlatform?.id}/containers/${selected.id}`);
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
        const can = (x: ContainerView) =>
          !x.isSwarmTask && x.state !== ContainerStateStatus.Offline && !isProcessing(x);
        const containers = getContainers(r);
        if (containsSystemContainer(containers)) return false;
        return containers.some(can);
      },
      separatorBefore: true,
      confirm: true,
      destructive: true,
      resourceType: 'Container',
      useVariables: (resources) => {
        const can = (x: ContainerView) =>
          !x.isSwarmTask && x.state !== ContainerStateStatus.Offline && !isProcessing(x);
        const containerIds = Array.from(
          new Set(
            getContainers(resources)
              .filter(can)
              .map((r) => r.id),
          ),
        );
        return { force: true, containerIds };
      },
    })
    .build();

const AdoptAction = baseContainerDropdownActions.adopt;
const ImportStackAction = baseContainerDropdownActions.importStack;
const AdoptGroupAction = baseContainerGroupActions.adopt;
const ImportStackGroupAction = baseContainerGroupActions.importStack;

const useContainerImportPlatform = () => useAppContext().currentPlatform?.type;

const ContainerAdoptDropdownAction: typeof AdoptAction = (props) =>
  useContainerImportPlatform() === PlatformType.Docker && isAdoptableContainer(props.resource) ? (
    <AdoptAction {...props} />
  ) : null;

const ContainerImportStackDropdownAction: typeof ImportStackAction = (props) => {
  const platformType = useContainerImportPlatform();
  const supported =
    platformType === PlatformType.Docker ||
    (platformType === PlatformType.DockerSwarm && (!props.resource.isSwarmTask || isSwarmStackGroup(props.resource)));
  return supported && isImportableStack(props.resource) ? <ImportStackAction {...props} /> : null;
};

const ContainerAdoptGroupAction: typeof AdoptGroupAction = (props) =>
  useContainerImportPlatform() === PlatformType.Docker &&
  props.resources.length === 1 &&
  isAdoptableContainer(props.resources[0]) ? (
    <AdoptGroupAction {...props} />
  ) : null;

const ContainerImportStackGroupAction: typeof ImportStackGroupAction = (props) => {
  const platformType = useContainerImportPlatform();
  const supported = platformType === PlatformType.Docker || platformType === PlatformType.DockerSwarm;
  return supported && isImportableStackSelection(props.resources) ? <ImportStackGroupAction {...props} /> : null;
};

export const ContainerDropdownActions = {
  ...baseContainerDropdownActions,
  adopt: ContainerAdoptDropdownAction,
  importStack: ContainerImportStackDropdownAction,
} satisfies typeof baseContainerDropdownActions;

export const ContainerGroupActions = {
  ...baseContainerGroupActions,
  adopt: ContainerAdoptGroupAction,
  importStack: ContainerImportStackGroupAction,
} satisfies typeof baseContainerGroupActions;
