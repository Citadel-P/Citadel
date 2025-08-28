import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { memo } from 'react';

export const ImageSateIndicator = memo(({ inUse }: { inUse: boolean }) => {
  const getStatusClass = () => (inUse ? 'bg-green-500' : 'bg-gray-500');

  return (
    <TooltipProvider delayDuration={200}>
      <Tooltip>
        <TooltipTrigger asChild>
          <div className={`${getStatusClass()} p-1 mr-2 h-2 w-2 rounded-full`} />
        </TooltipTrigger>
        <TooltipContent>
          <span>{inUse ? 'In use' : 'Unused'}</span>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
});

ImageSateIndicator.displayName = 'ImageStatusTooltip';
