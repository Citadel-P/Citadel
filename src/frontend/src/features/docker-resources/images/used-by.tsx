import type { ImageView } from '@/api/generated/api.types';
import { Link } from 'react-router';
import { Badge } from '@/components/ui/badge';
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
      className="grid w-max grid-cols-2 items-center gap-1"
      aria-label="View workloads using this image">
      {counts.some(([count]) => count > 0) ? (
        counts
          .filter(([count]) => count > 0)
          .map(([count, label]) => (
            <Badge variant="secondary" key={label}>
              {count} {label}
              {count === 1 ? '' : 's'}
            </Badge>
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
  ) : link;
}
