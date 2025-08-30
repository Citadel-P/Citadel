import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { memo } from 'react';

export const NetworkStateIndicator = memo(({ inUse }: { inUse: boolean }) => {
  const getStatusClass = () => (inUse ? 'bg-green-500' : 'bg-gray-500');

  return (
    <TooltipProvider delayDuration={200}>
      <Tooltip>
        <TooltipTrigger asChild>
          <div className={`${getStatusClass()} mr-2 h-2 w-2 rounded-full`} />
        </TooltipTrigger>
        <TooltipContent>
          <span>{inUse ? 'In use' : 'Unused'}</span>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
});

NetworkStateIndicator.displayName = 'NetworkStateIndicator';
