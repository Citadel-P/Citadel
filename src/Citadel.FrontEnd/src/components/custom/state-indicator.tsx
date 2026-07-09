import { memo } from 'react';
import {
  AlertRuleStatus,
  ActionRunStatus,
  ContainerStateStatus,
  DeploymentStatus,
  GitReposStatus,
  RegistryStatus,
  StackReleaseStatus,
} from '@/api/generated/api.types';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { LoaderCircle } from 'lucide-react';

type StateValue =
  | boolean
  | ContainerStateStatus
  | RegistryStatus
  | DeploymentStatus
  | StackReleaseStatus
  | AlertRuleStatus
  | GitReposStatus
  | ActionRunStatus;

type StateIndicatorKind = 'automationActionRun' | 'container';

type StatusStyle = {
  colorClass: string;
  tooltip: string;
};

const getAutomationActionRunStatusStyle = (value: StateValue): StatusStyle | undefined => {
  switch (value) {
    case ActionRunStatus.Queued:
      return { colorClass: 'bg-yellow-500', tooltip: 'Queued' };
    case ActionRunStatus.Running:
      return { colorClass: 'bg-blue-500', tooltip: 'Running' };
    case ActionRunStatus.Succeeded:
      return { colorClass: 'bg-green-500', tooltip: 'Succeeded' };
    case ActionRunStatus.Failed:
    case ActionRunStatus.TimedOut:
      return { colorClass: 'bg-red-500', tooltip: String(value) };
    case ActionRunStatus.Cancelled:
    case ActionRunStatus.Rejected:
      return { colorClass: 'bg-gray-500', tooltip: String(value) };
    default:
      return undefined;
  }
};

const getContainerStatusStyle = (value: StateValue): StatusStyle | undefined => {
  switch (value) {
    case ContainerStateStatus.Unknown:
      return { colorClass: 'bg-gray-400', tooltip: 'Unknown' };
    case ContainerStateStatus.Created:
      return { colorClass: 'bg-blue-400', tooltip: 'Created' };
    case ContainerStateStatus.Running:
      return { colorClass: 'bg-green-500', tooltip: 'Running' };
    case ContainerStateStatus.Paused:
      return { colorClass: 'bg-orange-500', tooltip: 'Paused' };
    case ContainerStateStatus.Restarting:
    case ContainerStateStatus.Removing:
      return { colorClass: 'bg-yellow-500', tooltip: String(value) };
    case ContainerStateStatus.Exited:
      return { colorClass: 'bg-gray-500', tooltip: 'Exited' };
    case ContainerStateStatus.Dead:
    case ContainerStateStatus.Offline:
      return { colorClass: 'bg-red-500', tooltip: String(value) };
    default:
      return undefined;
  }
};

const getStatusStyle = (value: StateValue, enableLabel?: boolean, kind?: StateIndicatorKind) => {
  // Boolean-based statuses
  if (typeof value === 'boolean') {
    return {
      colorClass: value ? 'bg-green-500' : 'bg-gray-500',
      tooltip: value ? (enableLabel ? 'Enabled' : 'In use') : enableLabel ? 'Disabled' : 'Unused',
    };
  }

  if (kind === 'automationActionRun') {
    return getAutomationActionRunStatusStyle(value) ?? { colorClass: 'bg-gray-400', tooltip: String(value) };
  }

  if (kind === 'container') {
    return getContainerStatusStyle(value) ?? { colorClass: 'bg-gray-400', tooltip: String(value) };
  }

  // Enum-based statuses
  switch (value) {
    // Registries
    case RegistryStatus.Active:
      return { colorClass: 'bg-green-500', tooltip: 'Active' };
    case RegistryStatus.Disabled:
      return { colorClass: 'bg-gray-500', tooltip: 'Disabled' };
    case RegistryStatus.Deprecated:
      return { colorClass: 'bg-orange-500', tooltip: 'Deprecated' };
    // Deployment/Stack
    case DeploymentStatus.Unknown:
    case StackReleaseStatus.Unknown:
      return { colorClass: 'bg-violet-400', tooltip: 'Unknown' };
    case DeploymentStatus.Created:
    case StackReleaseStatus.Created:
      return { colorClass: 'bg-blue-400', tooltip: 'Created' };
    case DeploymentStatus.Healthy:
    case StackReleaseStatus.Healthy:
      return { colorClass: 'bg-green-500', tooltip: 'Healthy' };
    case DeploymentStatus.Failed:
    case StackReleaseStatus.Failed:
      return { colorClass: 'bg-red-500', tooltip: 'Failed' };
    case DeploymentStatus.Stopped:
    case StackReleaseStatus.Stopped:
      return { colorClass: 'bg-gray-500', tooltip: 'Stopped' };
    case DeploymentStatus.Degraded:
    case StackReleaseStatus.Degraded:
      return { colorClass: 'bg-orange-500', tooltip: 'Degraded' };
    case StackReleaseStatus.Paused:
      return { colorClass: 'bg-orange-500', tooltip: 'Paused' };
    case DeploymentStatus.Applying:
    case DeploymentStatus.Pending:
    case StackReleaseStatus.Applying:
    case StackReleaseStatus.Pending:
      return { colorClass: 'bg-yellow-500', tooltip: 'Pending' };
    // Alerters
    case AlertRuleStatus.Enabled:
      return { colorClass: 'bg-green-500', tooltip: 'Enabled' };
    case AlertRuleStatus.Disabled:
      return { colorClass: 'bg-gray-500', tooltip: 'Disabled' };
    // Automation action runs
    case ActionRunStatus.Queued:
    case ActionRunStatus.Running:
    case ActionRunStatus.Succeeded:
    case ActionRunStatus.Failed:
    case ActionRunStatus.TimedOut:
    case ActionRunStatus.Cancelled:
    case ActionRunStatus.Rejected:
      return getAutomationActionRunStatusStyle(value) ?? { colorClass: 'bg-gray-400', tooltip: String(value) };
    // Git Repos
    case GitReposStatus.Unknown:
      return { colorClass: 'bg-gray-400', tooltip: 'Unknown' };
    // Containers
    case ContainerStateStatus.Created:
    case ContainerStateStatus.Exited:
    case ContainerStateStatus.Paused:
    case ContainerStateStatus.Running:
    case ContainerStateStatus.Offline:
      return getContainerStatusStyle(value) ?? { colorClass: 'bg-gray-400', tooltip: String(value) };
    default:
      return { colorClass: 'bg-gray-400', tooltip: String(value) };
  }
};

export const StateIndicator = memo(
  ({
    value,
    isProcessing,
    enableLabel,
    kind,
  }: {
    value: StateValue;
    isProcessing?: boolean;
    enableLabel?: boolean;
    kind?: StateIndicatorKind;
  }) => {
    const { colorClass, tooltip } = getStatusStyle(value, enableLabel, kind);
    if (isProcessing) return <LoaderCircle className="mr-1 h-3 w-3 animate-spin" />;
    return (
      <TooltipProvider delayDuration={200}>
        <Tooltip>
          <TooltipTrigger asChild>
            <div className={`${colorClass} mr-2 h-2 w-2 rounded-full`} />
          </TooltipTrigger>
          <TooltipContent>
            <span>{tooltip}</span>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  },
);

StateIndicator.displayName = 'StateIndicator';
