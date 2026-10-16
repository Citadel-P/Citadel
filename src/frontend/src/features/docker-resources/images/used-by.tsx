import type { ImageView } from '@/api/generated/api.types';
import { Link } from 'react-router';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { formatId } from '@/lib/utils';
export function ImageUsedBy({ image }: { image: ImageView }) {
  const usage = image.usedBy;
  if (!usage) return <span className="text-muted-foreground">Unavailable</span>;
  const counts = [
    [usage.containers, 'container'],
    [usage.deployments, 'deployment'],
    [usage.stacks, 'stack'],
    [usage.swarmServices, 'Swarm Service'],
  ] as const;
  const node = image.dockerNodeId ? `?dockerNodeId=${encodeURIComponent(image.dockerNodeId)}` : '';
  const link = (
    <Link
      to={`/platforms/${image.platformId}/images/${formatId(image.dockerImageId)}/${node}#workloads`}
      className="grid w-max grid-cols-2 items-center gap-x-3 gap-y-0.5 rounded-sm text-xs text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
      aria-label="View workloads using this image">
      {counts.some(([count]) => count > 0) ? (
        counts
          .filter(([count]) => count > 0)
          .map(([count, label]) => (
            <span className="whitespace-nowrap tabular-nums" key={label}>
              {count} {label}
              {count === 1 ? '' : 's'}
            </span>
          ))
      ) : (
        <span className="col-span-2 text-muted-foreground">None found</span>
      )}
    </Link>
  );
  return usage.isPartial ? (
    <Tooltip>
      <TooltipTrigger asChild>{link}</TooltipTrigger>
      <TooltipContent>
        Shows references Citadel can find in resources you can access. Some references may be missing.
      </TooltipContent>
    </Tooltip>
  ) : (
    link
  );
}
