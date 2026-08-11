import { SwarmQuorumState, SwarmQuorumView } from '@/api/generated/api.types';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';

const stateStyles: Record<SwarmQuorumState, string> = {
  [SwarmQuorumState.Healthy]: 'bg-emerald-500',
  [SwarmQuorumState.Degraded]: 'bg-amber-500',
  [SwarmQuorumState.Lost]: 'bg-rose-500',
  [SwarmQuorumState.Unknown]: 'bg-muted-foreground',
};

export const SwarmQuorumStatus = ({
  quorum,
  managerCount,
  showCounts = true,
  className,
}: {
  quorum: SwarmQuorumView;
  managerCount: number | string;
  showCounts?: boolean;
  className?: string;
}) => {
  const total = Number(managerCount) || 0;
  const reachable = Number(quorum.reachableManagers) || 0;
  const required = Number(quorum.requiredManagers) || 0;
  const label = quorum.state.toLowerCase();
  const countLabel = total > 0 ? `${reachable}/${total}` : null;
  const noRedundancy = quorum.state === SwarmQuorumState.Healthy && total === 1;
  const detail = getQuorumDetail(quorum.state, reachable, total, required, quorum.hasLeader, noRedundancy);

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span
          role="status"
          aria-label={`Swarm quorum ${label}${countLabel ? `, ${countLabel} managers reachable` : ''}`}
          className={cn('inline-flex min-w-0 cursor-default items-center gap-1.5 text-xs text-muted-foreground', className)}>
          <span className={cn('h-2 w-2 shrink-0 rounded-full', stateStyles[quorum.state])} />
          <span className="whitespace-nowrap">Quorum {label}</span>
          {showCounts && countLabel && <span className="tabular-nums">{countLabel}</span>}
        </span>
      </TooltipTrigger>
      <TooltipContent>{detail}</TooltipContent>
    </Tooltip>
  );
};

const getQuorumDetail = (
  state: SwarmQuorumState,
  reachable: number,
  total: number,
  required: number,
  hasLeader: boolean,
  noRedundancy: boolean,
) => {
  if (state === SwarmQuorumState.Unknown) {
    return 'Quorum cannot be confirmed because manager inventory is unavailable or stale.';
  }

  const leader = hasLeader ? 'A leader is elected.' : 'No leader is elected.';
  const majority = `${reachable} of ${total} managers are reachable; ${required} are required for quorum.`;
  const redundancy = noRedundancy ? ' This cluster has no manager failure tolerance.' : '';
  return `${leader} ${majority}${redundancy}`;
};
