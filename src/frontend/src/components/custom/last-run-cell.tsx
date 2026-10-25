import { isFailedRunStatus } from '@/components/custom/state-badge';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { PlayCircle } from 'lucide-react';
import type { ReactNode } from 'react';

export function LastRunCell({ status, children }: { status?: string; children?: ReactNode }) {
  if (!status) return <span className="text-sm text-muted-foreground">No runs</span>;

  const label = status.replace(/([a-z])([A-Z])/g, '$1 $2').toLowerCase();
  const displayStatus = label.charAt(0).toUpperCase() + label.slice(1);
  const iconColor = isFailedRunStatus(status)
    ? 'text-red-500'
    : status === 'Succeeded'
      ? 'text-green-500'
      : status === 'SucceededWithWarnings'
        ? 'text-orange-500'
        : 'text-muted-foreground';

  return (
    <div className="inline-flex items-center gap-2 text-sm">
      <Tooltip>
        <TooltipTrigger asChild>
          <span role="img" aria-label={`Last run: ${displayStatus}`} className="inline-flex">
            <PlayCircle aria-hidden="true" className={`size-3.5 ${iconColor}`} />
          </span>
        </TooltipTrigger>
        <TooltipContent>{displayStatus}</TooltipContent>
      </Tooltip>
      {children}
    </div>
  );
}
