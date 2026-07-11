import {
  PlatformConnectorType,
  PlatformDescriptorDockerPlatformDescriptor,
  PlatformStatus,
  PlatformView,
  TagSummaryView,
} from '@/api/generated/api.types';
import DockerIcon from '@/assets/docker.svg';
import { Link } from 'react-router';
import { Power, PowerOff, CirclePause, PlugZap, Rocket, Layers } from 'lucide-react';
import { toFixedNumber } from '@/lib/utils';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { fromNow } from '@/lib/dayjs.helper';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { ContentCard } from '@/components/custom/content-card';
import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import { getTagTextColor } from '@/features/tags/tag-colors';
import { OverflowCountBadge } from '@/components/custom/common';

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
  const isOnline = platform.status === PlatformStatus.Online;
  const lastSnapshot = platform.stats?.at(0)?.created
    ? new Date(platform.stats[0].created! * 1000).getTime()
    : undefined;
  const cpuUsage = platform.stats && isOnline ? Number(toFixedNumber(platform.stats[0]?.cpuUsage)) || 0 : 0;
  const memUsage = platform.stats && isOnline ? Number(toFixedNumber(platform.stats[0]?.memoryUsage)) || 0 : 0;

  return (
    <ContentCard>
      <TooltipProvider delayDuration={200}>
        <div className="relative grid grid-cols-[auto_minmax(0,1fr)] items-start gap-x-3 gap-y-4 px-4 py-4 pr-12 xl:grid-cols-[auto_minmax(18rem,32rem)_minmax(28rem,1fr)_auto] xl:items-center xl:gap-x-5 xl:pr-4">
          {/* Logo + status */}
          <div className="relative shrink-0">
            <div className="h-16 w-16 rounded-full border border-border/60">
              <DockerIcon />
            </div>
            <Tooltip>
              <TooltipTrigger asChild>
                <div
                  className={`absolute top-0 right-0 h-3.5 w-3.5 rounded-full border-2 border-card ${isOnline ? 'bg-emerald-500' : 'bg-rose-500'}`}
                />
              </TooltipTrigger>
              <TooltipContent>
                {isOnline ? 'Online' : 'Offline'}
                {lastSnapshot ? ` · ${fromNow(lastSnapshot)}` : ''}
              </TooltipContent>
            </Tooltip>
          </div>

          {/* Identity + resource links */}
          <div className="min-w-0 overflow-hidden space-y-1.5 self-center">
            <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
              <Link
                to={`/platforms/edit/${platform.id}`}
                className="min-w-0 truncate text-sm font-medium text-foreground hover:underline">
                {platform.name}
              </Link>
              <span className="min-w-0 truncate text-xs text-muted-foreground">
                {descriptor.operatingSystem} v{platform.serverVersion}
              </span>

              <span className="inline-flex min-w-0 items-center gap-1 truncate text-xs text-muted-foreground">
                <PlugZap className="h-3.5 w-3.5" /> {getConnectorLabel(platform.connectorType, platform.agentVersion)}
              </span>
            </div>

            <div className="flex flex-wrap items-center gap-x-3 gap-y-0.5 text-xs text-muted-foreground">
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
            </div>
          </div>

          <div className="absolute right-2 top-2 xl:static xl:col-start-4 xl:row-start-1 xl:justify-self-end xl:self-center">
            <RowActionMenu resource={platform} actions={actions} />
          </div>

          <div className="col-span-full grid min-w-0 grid-cols-2 gap-x-4 gap-y-3 sm:grid-cols-3 lg:grid-cols-6 xl:col-span-1 xl:col-start-3 xl:row-start-1">
            {/* Deployments + stacks */}
            <StatSection label="Deployments">
              <LinkedStat
                icon={Rocket}
                value={platform.deploymentCount}
                tooltip="Deployments on this platform"
                to={`/deployments?platformId=${platform.id}`}
                className="text-sky-500"
              />
            </StatSection>

            <StatSection label="Stacks">
              <LinkedStat
                icon={Layers}
                value={platform.stackCount}
                tooltip="Stacks on this platform"
                to={`/stacks?platformId=${platform.id}`}
                className="text-violet-500"
              />
            </StatSection>

            {/* Containers breakdown */}
            <StatSection label="Containers">
              <div className="flex items-center justify-center gap-3">
                <ContainerStat
                  icon={Power}
                  value={descriptor.containersRunning}
                  tooltip="Running"
                  className="text-emerald-500"
                />
                <ContainerStat
                  icon={PowerOff}
                  value={descriptor.containersStopped}
                  tooltip="Stopped"
                  className="text-rose-500"
                />
                <ContainerStat
                  icon={CirclePause}
                  value={descriptor.containersPaused}
                  tooltip="Paused"
                  className="text-amber-500"
                />
              </div>
            </StatSection>

            {/* CPU + Memory */}
            <StatSection label="CPU">
              <UsageMetric value={cpuUsage} online={isOnline} />
            </StatSection>
            <StatSection label="RAM">
              <UsageMetric value={memUsage} online={isOnline} />
            </StatSection>
            <StatSection label="Tags">
              <CompactTagSummary tags={platform.tags} />
            </StatSection>
          </div>
        </div>
      </TooltipProvider>
    </ContentCard>
  );
};

const StatSection = ({ label, children }: { label: string; children: React.ReactNode }) => (
  <div className="flex min-w-0 flex-col items-center justify-center gap-0.5 text-center">
    <span className="max-w-full truncate text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
      {label}
    </span>
    <div className="min-w-0 text-sm text-foreground">{children}</div>
  </div>
);

const UsageMetric = ({ value, online }: { value: number; online: boolean }) => {
  return (
    <div className="flex items-center justify-between gap-3 text-xs text-muted-foreground">
      <span className="shrink-0 tabular-nums text-foreground">{online ? `${value} %` : 'N/A'}</span>
    </div>
  );
};

const LinkedStat = ({
  icon: Icon,
  value,
  tooltip,
  to,
  className,
}: {
  icon: React.FC<{ height?: number; width?: number; className?: string }>;
  value?: string | number | null;
  tooltip: string;
  to: string;
  className?: string;
}) => (
  <Tooltip>
    <TooltipTrigger asChild>
      <Link to={to} className="flex items-center gap-1 hover:underline">
        <Icon height={12} width={12} className={className} />
        <span className="tabular-nums text-[13px] text-muted-foreground">{value ?? 0}</span>
      </Link>
    </TooltipTrigger>
    <TooltipContent>{tooltip}</TooltipContent>
  </Tooltip>
);

const ContainerStat = ({
  icon: Icon,
  value,
  tooltip,
  className,
}: {
  icon: React.FC<{ height?: number; width?: number; className?: string }>;
  value?: string | number | null;
  tooltip: string;
  className?: string;
}) => (
  <Tooltip>
    <TooltipTrigger asChild>
      <div className="flex items-center gap-1 cursor-default">
        <Icon height={12} width={12} className={className} />
        <span className="tabular-nums text-[13px] text-muted-foreground">{value ?? '-'}</span>
      </div>
    </TooltipTrigger>
    <TooltipContent>{tooltip}</TooltipContent>
  </Tooltip>
);

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
