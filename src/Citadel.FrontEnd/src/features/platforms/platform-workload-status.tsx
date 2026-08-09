import { PlatformWorkloadStatusCountsView } from '@/api/generated/api.types';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';

export const PLATFORM_WORKLOAD_ICON_CLASS_NAMES = {
  containers: 'text-emerald-500',
  deployments: 'text-sky-500',
  stacks: 'text-violet-500',
  images: 'text-orange-500',
  volumes: 'text-amber-500',
  networks: 'text-cyan-500',
  backups: 'text-teal-500',
} as const;

export type WorkloadState = {
  label: string;
  value: string | number | null | undefined;
  colorClassName: string;
  showWhenEmpty?: boolean;
};

type ContainerWorkloadStatusCounts = {
  running?: string | number | null;
  stopped?: string | number | null;
  paused?: string | number | null;
};

export const WorkloadStatusBreakdown = ({
  states,
  showSeparator = true,
}: {
  states: WorkloadState[];
  showSeparator?: boolean;
}) => {
  const visibleStates = states.filter((item) => item.showWhenEmpty || Number(item.value ?? 0) > 0);

  if (visibleStates.length === 0) {
    return null;
  }

  return (
    <>
      {showSeparator && <span className="h-3 w-px shrink-0 bg-border" />}
      <div className="flex min-w-0 flex-wrap items-center justify-center gap-x-1.5 gap-y-0.5">
        {visibleStates.map((item) => (
          <Tooltip key={item.label}>
            <TooltipTrigger asChild>
              <span
                aria-label={`${item.label}: ${item.value ?? 0}`}
                className="inline-flex cursor-default items-center gap-1 text-[11px] text-muted-foreground">
                <span className={cn('h-1.5 w-1.5 rounded-full', item.colorClassName)} />
                <span className="tabular-nums">{item.value ?? 0}</span>
              </span>
            </TooltipTrigger>
            <TooltipContent>{item.label}</TooltipContent>
          </Tooltip>
        ))}
      </div>
    </>
  );
};

export const getContainerStates = (counts: ContainerWorkloadStatusCounts): WorkloadState[] => [
  state('Running', counts.running, 'bg-emerald-500'),
  state('Stopped', counts.stopped, 'bg-rose-500'),
  state('Paused', counts.paused, 'bg-amber-500'),
];

const getAvailabilityStates = (counts?: PlatformWorkloadStatusCountsView | null): WorkloadState[] => [
  state('Healthy', counts?.healthy, 'bg-emerald-500'),
  state('Degraded', counts?.degraded, 'bg-amber-500'),
  state('Failed', counts?.failed, 'bg-rose-500', false),
  state('Stopped', counts?.stopped, 'bg-neutral-400'),
  state('In progress', counts?.inProgress, 'bg-sky-500', false),
  state('Unknown', counts?.unknown, 'bg-muted-foreground', false),
];

export const getDeploymentStates = getAvailabilityStates;
export const getServiceStates = getAvailabilityStates;

export const getStackStates = (counts?: PlatformWorkloadStatusCountsView | null): WorkloadState[] => [
  state('Healthy', counts?.healthy, 'bg-emerald-500'),
  state('Degraded', counts?.degraded, 'bg-amber-500'),
  state('Failed', counts?.failed, 'bg-rose-500', false),
  state('Stopped', counts?.stopped, 'bg-neutral-400'),
  state('Paused', counts?.paused, 'bg-yellow-500'),
  state('In progress', counts?.inProgress, 'bg-sky-500', false),
  state('Unknown', counts?.unknown, 'bg-muted-foreground', false),
];

const state = (
  label: string,
  value: string | number | null | undefined,
  colorClassName: string,
  showWhenEmpty = true,
): WorkloadState => ({
  label,
  value,
  colorClassName,
  showWhenEmpty,
});
