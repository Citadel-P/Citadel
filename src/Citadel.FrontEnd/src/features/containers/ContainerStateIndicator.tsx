import { ContainerStateStatus } from '@/api/_generated';
import { memo } from 'react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

export const ContainerStateIndicator = memo(({ stat }: { stat: ContainerStateStatus }) => {
  const getStatusClass = (status: ContainerStateStatus) => {
    switch (status) {
      case ContainerStateStatus.Created:
        return 'bg-blue-400';
      case ContainerStateStatus.Exited:
        return 'bg-gray-500';
      case ContainerStateStatus.Paused:
        return 'bg-orange-500';
      case ContainerStateStatus.Running:
        return 'bg-green-500';
      case ContainerStateStatus.Offline:
        return 'bg-red-500';
      default:
        return '';
    }
  };

  const statusText = stat.charAt(0).toUpperCase() + stat.slice(1);

  return (
    <TooltipProvider delayDuration={200}>
      <Tooltip>
        <TooltipTrigger asChild>
          <div className={`${getStatusClass(stat)} p-1 mr-1 h-2 w-2 rounded-full`} />
        </TooltipTrigger>
        <TooltipContent>
          <span>{statusText}</span>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
});

ContainerStateIndicator.displayName = 'ContainerStateIndicator';
