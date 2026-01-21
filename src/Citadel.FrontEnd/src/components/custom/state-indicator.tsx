import { memo } from 'react';
import { ContainerStateStatus, DeploymentStatus, RegistryStatus } from '@/api/generated/api.types';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

type StateValue = boolean | ContainerStateStatus | RegistryStatus | DeploymentStatus;

const getStatusStyle = (value: StateValue) => {
  // Boolean-based statuses
  if (typeof value === 'boolean') {
    return {
      colorClass: value ? 'bg-green-500' : 'bg-gray-500',
      tooltip: value ? 'In use' : 'Unused',
    };
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
    // Deployment
    case DeploymentStatus.Created:
      return { colorClass: 'bg-blue-400', tooltip: 'Created' };
    case DeploymentStatus.Healthy:
      return { colorClass: 'bg-green-500', tooltip: 'Healthy' };
    case DeploymentStatus.Failed:
      return { colorClass: 'bg-red-500', tooltip: 'Failed' };
    case DeploymentStatus.Stopped:
      return { colorClass: 'bg-gray-500', tooltip: 'Stopped' };
    case DeploymentStatus.Degraded:
      return { colorClass: 'bg-orange-500', tooltip: 'Degraded' };
    case DeploymentStatus.Applying:
    case DeploymentStatus.Pending:
      return { colorClass: 'bg-yellow-500', tooltip: 'Pending' };
    // Containers
    case ContainerStateStatus.Created:
      return { colorClass: 'bg-blue-400', tooltip: 'Created' };
    case ContainerStateStatus.Exited:
      return { colorClass: 'bg-gray-500', tooltip: 'Exited' };
    case ContainerStateStatus.Paused:
      return { colorClass: 'bg-orange-500', tooltip: 'Paused' };
    case ContainerStateStatus.Running:
      return { colorClass: 'bg-green-500', tooltip: 'Running' };
    case ContainerStateStatus.Offline:
      return { colorClass: 'bg-red-500', tooltip: 'Offline' };
    default:
      return { colorClass: 'bg-gray-400', tooltip: String(value) };
  }
};

export const StateIndicator = memo(({ value }: { value: StateValue }) => {
  const { colorClass, tooltip } = getStatusStyle(value);

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
});

StateIndicator.displayName = 'StateIndicator';
