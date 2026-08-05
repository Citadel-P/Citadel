import {
  PlatformConnectorType,
  PlatformDescriptorDockerPlatformDescriptor,
  PlatformDescriptorDockerSwarmPlatformDescriptor,
  PlatformStatus,
  PlatformType,
  PlatformView,
  TagSummaryView,
} from '@/api/generated/api.types';
import DockerIcon from '@/assets/docker.svg';
import { Link } from 'react-router';
import { Box, Cpu, HardDrive, Layers, MemoryStick, Network, PlugZap, Rocket, Workflow } from 'lucide-react';
import { cn, toFixedNumber } from '@/lib/utils';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { fromNow } from '@/lib/dayjs.helper';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { ContentCard } from '@/components/custom/content-card';
import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import { getTagTextColor } from '@/features/tags/tag-colors';
import { OverflowCountBadge } from '@/components/custom/common';
import { byteTransform } from '@/lib/bytes.helper';
import {
  getContainerStates,
  getDeploymentStates,
  getStackStates,
  PLATFORM_WORKLOAD_ICON_CLASS_NAMES,
  WorkloadState,
  WorkloadStatusBreakdown,
} from './platform-workload-status';

export const DockerPlatform = ({
  platform,
  actions,
}: {
  platform: PlatformView;
  actions: Record<
    string,
    React.FC<{ resource: PlatformView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const descriptor = platform.platformDescriptor as PlatformDescriptorDockerPlatformDescriptor;
  const isSwarm = platform.type === PlatformType.DockerSwarm;
  const swarmDescriptor = isSwarm
    ? (platform.platformDescriptor as PlatformDescriptorDockerSwarmPlatformDescriptor)
    : undefined;
  const isOnline = platform.status === PlatformStatus.Online;
  const lastSnapshot = platform.stats?.at(0)?.created
    ? new Date(platform.stats[0].created! * 1000).getTime()
    : undefined;
  const cpuUsage = platform.stats && isOnline ? Number(toFixedNumber(platform.stats[0]?.cpuUsage)) || 0 : 0;
  const memUsage = platform.stats && isOnline ? Number(toFixedNumber(platform.stats[0]?.memoryUsage)) || 0 : 0;
  const currentStat = platform.stats?.[0];
  const rawDiskUsage = currentStat?.diskUsage;
  const parsedDiskUsage = rawDiskUsage == null ? null : Number(rawDiskUsage);
  const parsedDiskUsedBytes = currentStat?.diskUsedBytes == null ? null : Number(currentStat.diskUsedBytes);
  const parsedDiskTotalBytes = currentStat?.diskTotalBytes == null ? null : Number(currentStat.diskTotalBytes);
  const diskUsage =
    isOnline &&
    parsedDiskUsage != null &&
    Number.isFinite(parsedDiskUsage) &&
    parsedDiskUsage >= 0 &&
    parsedDiskUsage <= 100 &&
    parsedDiskUsedBytes != null &&
    Number.isFinite(parsedDiskUsedBytes) &&
    parsedDiskUsedBytes >= 0 &&
    parsedDiskTotalBytes != null &&
    Number.isFinite(parsedDiskTotalBytes) &&
    parsedDiskTotalBytes > 0 &&
    parsedDiskUsedBytes <= parsedDiskTotalBytes
      ? Number(toFixedNumber(parsedDiskUsage))
      : null;
  const diskUsageDetails =
    diskUsage != null && parsedDiskUsedBytes != null && parsedDiskTotalBytes != null
      ? `${byteTransform(parsedDiskUsedBytes, 1)} of ${byteTransform(parsedDiskTotalBytes, 1)} used`
      : 'Disk usage is unavailable';

  return (
    <ContentCard className="overflow-hidden">
      <TooltipProvider delayDuration={200}>
        <div className="relative grid grid-cols-[auto_minmax(0,1fr)] items-start gap-x-3 gap-y-3 px-4 py-3 pr-12 2xl:grid-cols-[auto_minmax(15rem,1fr)_minmax(18rem,0.9fr)_minmax(17rem,0.9fr)_auto] 2xl:items-center 2xl:gap-x-5 2xl:pr-4">
          {/* Logo + status */}
          <div className="relative shrink-0">
            <div className="h-12 w-12 rounded-full border border-border/60 p-0.5">
              {isSwarm ? (
                <div
                  aria-label="Docker Swarm"
                  className="flex h-full w-full items-center justify-center rounded-full bg-primary/10 text-primary">
                  <Network className="h-7 w-7" />
                </div>
              ) : (
                <DockerIcon aria-label="Docker" />
              )}
            </div>
            <Tooltip>
              <TooltipTrigger asChild>
                <div
                  className={`absolute top-0 right-0 h-3 w-3 rounded-full border-2 border-card ${isOnline ? 'bg-emerald-500' : 'bg-rose-500'}`}
                />
              </TooltipTrigger>
              <TooltipContent>
                {isOnline ? 'Online' : 'Offline'}
                {lastSnapshot ? ` - ${fromNow(lastSnapshot)}` : ''}
              </TooltipContent>
            </Tooltip>
          </div>

          {/* Identity + resource links */}
          <div className="min-w-0 space-y-1 overflow-hidden self-center">
            <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
              <Link
                to={`/platforms/edit/${platform.id}`}
                className="min-w-0 truncate text-[15px] font-medium text-foreground hover:underline">
                {platform.name}
              </Link>
            </div>

            <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
              <span className="min-w-0 truncate text-xs text-muted-foreground">
                {descriptor.operatingSystem} v{platform.serverVersion}
              </span>
              <span className="hidden h-3 w-px bg-border sm:block" />
              <span className="inline-flex min-w-0 items-center gap-1 truncate text-xs text-muted-foreground">
                <PlugZap className="h-3.5 w-3.5" /> {getConnectorLabel(platform.connectorType, platform.agentVersion)}
              </span>
            </div>

            <div className="flex flex-wrap items-center gap-x-3 gap-y-0.5 text-xs text-muted-foreground">
              {swarmDescriptor ? (
                <>
                  <Link to={`/platforms/${platform.id}/nodes`} className="hover:text-foreground hover:underline">
                    {formatCount(swarmDescriptor.nodes, 'node')}
                  </Link>
                  <Link to={`/platforms/${platform.id}/nodes`} className="hover:text-foreground hover:underline">
                    {formatCount(swarmDescriptor.managers, 'manager')}
                  </Link>
                  <Link to={`/platforms/${platform.id}/services`} className="hover:text-foreground hover:underline">
                    {formatCount(swarmDescriptor.serviceCount, 'service')}
                  </Link>
                  <Link to={`/platforms/${platform.id}/tasks`} className="hover:text-foreground hover:underline">
                    {formatCount(swarmDescriptor.runningTaskCount, 'running task')}
                  </Link>
                </>
              ) : (
                <>
                  <Link to={`/platforms/${platform.id}/containers`} className="hover:text-foreground hover:underline">
                    {descriptor.containerCount ?? '-'} containers
                  </Link>
                  <Link to={`/platforms/${platform.id}/images`} className="hover:text-foreground hover:underline">
                    {platform.imageCount ?? '-'} images
                  </Link>
                  <Link to={`/platforms/${platform.id}/volumes`} className="hover:text-foreground hover:underline">
                    {platform.volumeCount ?? '-'} volumes
                  </Link>
                  <Link to={`/platforms/${platform.id}/networks`} className="hover:text-foreground hover:underline">
                    {platform.networkCount ?? '-'} networks
                  </Link>
                </>
              )}
              <CompactTagSummary tags={platform.tags} />
            </div>
          </div>

          <div className="absolute right-2 top-2 2xl:static 2xl:col-start-5 2xl:row-start-1 2xl:justify-self-end 2xl:self-center">
            <RowActionMenu resource={platform} actions={actions} />
          </div>

          <section
            aria-label={isSwarm ? 'Swarm workloads' : 'Platform workloads'}
            className="col-span-full grid min-w-0 grid-cols-3 gap-2 border-t pt-3 2xl:col-span-1 2xl:col-start-3 2xl:row-start-1 2xl:border-l 2xl:border-t-0 2xl:pl-5 2xl:pt-0">
            <WorkloadMetric
              icon={Rocket}
              label="Deployments"
              total={platform.deploymentStatusCounts.total}
              to={`/deployments?platformId=${platform.id}`}
              iconClassName={PLATFORM_WORKLOAD_ICON_CLASS_NAMES.deployments}
              states={getDeploymentStates(platform.deploymentStatusCounts)}
            />
            <WorkloadMetric
              icon={Layers}
              label="Stacks"
              total={platform.stackStatusCounts.total}
              to={`/stacks?platformId=${platform.id}`}
              iconClassName={PLATFORM_WORKLOAD_ICON_CLASS_NAMES.stacks}
              states={getStackStates(platform.stackStatusCounts)}
            />
            {swarmDescriptor ? (
              <WorkloadMetric
                icon={Workflow}
                label="Running tasks"
                total={swarmDescriptor.runningTaskCount}
                to={`/platforms/${platform.id}/tasks`}
                iconClassName="text-sky-500"
              />
            ) : (
              <WorkloadMetric
                icon={Box}
                label="Containers"
                total={descriptor.containerCount}
                to={`/platforms/${platform.id}/containers`}
                iconClassName={PLATFORM_WORKLOAD_ICON_CLASS_NAMES.containers}
                states={getContainerStates({
                  running: descriptor.containersRunning,
                  stopped: descriptor.containersStopped,
                  paused: descriptor.containersPaused,
                })}
              />
            )}
          </section>

          <section
            aria-label={isSwarm ? 'Connected manager utilization' : 'Platform utilization'}
            className="col-span-full grid min-w-0 grid-cols-1 gap-1.5 border-t pt-3 sm:grid-cols-3 2xl:col-span-1 2xl:col-start-4 2xl:row-start-1 2xl:grid-cols-1 2xl:border-l 2xl:border-t-0 2xl:pl-5 2xl:pt-0">
            <UsageMetric
              icon={Cpu}
              label="CPU"
              value={cpuUsage}
              online={isOnline}
              colorClassName="bg-sky-500"
              iconClassName="text-sky-500"
              tooltip={`Container CPU usage normalized across ${platform.cpuCount} ${isSwarm ? 'connected manager' : 'platform'} cores`}
            />
            <UsageMetric
              icon={MemoryStick}
              label="RAM"
              value={memUsage}
              online={isOnline}
              colorClassName="bg-emerald-500"
              iconClassName="text-emerald-500"
              tooltip={`Container working-set memory as a share of ${byteTransform(platform.memTotal, 1)} ${isSwarm ? 'connected manager' : 'platform'} memory`}
            />
            <UsageMetric
              icon={HardDrive}
              label="Disk"
              value={diskUsage}
              online={isOnline}
              colorClassName="bg-amber-500"
              iconClassName="text-amber-500"
              tooltip={diskUsageDetails}
            />
          </section>
        </div>
      </TooltipProvider>
    </ContentCard>
  );
};

const UsageMetric = ({
  icon: Icon,
  label,
  value,
  online,
  colorClassName,
  iconClassName,
  tooltip,
}: {
  icon: React.FC<{ className?: string }>;
  label: string;
  value: number | null;
  online: boolean;
  colorClassName: string;
  iconClassName: string;
  tooltip: string;
}) => {
  const available = online && value != null;
  const displayValue = available ? `${value} %` : 'N/A';

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <div className="min-w-0 cursor-default">
          <div className="mb-0.5 flex min-w-0 items-center justify-between gap-2 text-xs">
            <span className="flex min-w-0 items-center gap-1.5 text-muted-foreground">
              <Icon className={cn('h-3.5 w-3.5 shrink-0', iconClassName)} />
              <span className="truncate">{label}</span>
            </span>
            <span className="shrink-0 tabular-nums text-foreground">{displayValue}</span>
          </div>
          <div
            role="progressbar"
            aria-label={`${label} usage`}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={available ? value : undefined}
            className="h-1 overflow-hidden rounded-full bg-muted">
            <div
              className={cn('h-full rounded-full transition-[width] duration-300', colorClassName)}
              style={{ width: available ? `${value}%` : '0%' }}
            />
          </div>
        </div>
      </TooltipTrigger>
      <TooltipContent>{tooltip}</TooltipContent>
    </Tooltip>
  );
};

const WorkloadMetric = ({
  icon: Icon,
  label,
  total,
  to,
  iconClassName,
  states = [],
}: {
  icon: React.FC<{ className?: string }>;
  label: string;
  total?: string | number | null;
  to?: string;
  iconClassName?: string;
  states?: WorkloadState[];
}) => {
  const labelContent = (
    <>
      <Icon className={cn('h-3.5 w-3.5 shrink-0', iconClassName)} />
      <span className="truncate">{label}</span>
    </>
  );

  return (
    <div className="flex min-w-0 flex-col items-center justify-center gap-1 rounded-sm px-1 py-1.5">
      {to ? (
        <Link
          to={to}
          className="flex min-w-0 items-center gap-1 text-xs text-muted-foreground hover:text-foreground hover:underline">
          {labelContent}
        </Link>
      ) : (
        <span className="flex min-w-0 items-center gap-1 text-xs text-muted-foreground">{labelContent}</span>
      )}
      <div className="flex min-w-0 items-center justify-center gap-1.5">
        {to ? (
          <Link to={to} className="tabular-nums text-[15px] font-medium text-foreground hover:underline">
            {total ?? '-'}
          </Link>
        ) : (
          <span className="tabular-nums text-[15px] font-medium text-foreground">{total ?? '-'}</span>
        )}
        {states.length > 0 && <WorkloadStatusBreakdown states={states} />}
      </div>
    </div>
  );
};

const CompactTagSummary = ({ tags }: { tags?: TagSummaryView[] | null }) => {
  const visibleTags = tags?.slice(0, 1) ?? [];
  const hiddenCount = Math.max((tags?.length ?? 0) - visibleTags.length, 0);

  if (!tags || tags.length === 0) {
    return <span className="text-xs text-muted-foreground">-</span>;
  }

  return (
    <HoverCard openDelay={150} closeDelay={150}>
      <HoverCardTrigger asChild>
        <div className="inline-flex max-w-full cursor-default items-center gap-1.5 whitespace-nowrap">
          {visibleTags.map((tag) => (
            <span
              key={tag.id}
              className="inline-flex min-w-0 max-w-24 items-center gap-1 rounded-sm px-1.5 py-0.5 text-[11px] font-medium"
              style={{ backgroundColor: tag.color, color: getTagTextColor(tag.color) }}
              title={tag.name}>
              <span className="truncate">{tag.name}</span>
            </span>
          ))}
          <OverflowCountBadge count={hiddenCount} title={`${hiddenCount} more tags`} />
        </div>
      </HoverCardTrigger>
      <HoverCardContent className="flex w-fit max-w-80 flex-col gap-2 bg-background p-3 text-xs">
        {tags.map((tag) => (
          <div key={tag.id} className="flex min-w-0 items-center gap-2">
            <span
              className="size-2.5 shrink-0 rounded-full border border-border"
              style={{ backgroundColor: tag.color }}
            />
            <span className="max-w-64 truncate">{tag.name}</span>
          </div>
        ))}
      </HoverCardContent>
    </HoverCard>
  );
};

const getConnectorLabel = (connectorType: PlatformConnectorType, agentVersion?: string | null) => {
  if (connectorType === PlatformConnectorType.Agent) {
    return <span>agent v{agentVersion ?? '-'}</span>;
  }

  if (connectorType === PlatformConnectorType.EdgeAgent) {
    return <span>edge agent v{agentVersion ?? '-'}</span>;
  }

  return 'Local';
};

const formatCount = (value: number | string | null | undefined, singular: string) =>
  `${value ?? '-'} ${Number(value) === 1 ? singular : `${singular}s`}`;
