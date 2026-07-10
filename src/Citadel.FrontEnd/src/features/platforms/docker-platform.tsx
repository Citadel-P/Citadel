import {
  PlatformConnectorType,
  PlatformDescriptorDockerPlatformDescriptor,
  PlatformStatus,
  PlatformView,
} from '@/api/generated/api.types';
import DockerIcon from '@/assets/docker.svg';
import { Link } from 'react-router';
import { Power, PowerOff, CirclePause, PlugZap, Cpu, MemoryStick, Rocket, Layers } from 'lucide-react';
import { toFixedNumber } from '@/lib/utils';
import { byteTransform } from '@/lib/bytes.helper';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { fromNow } from '@/lib/dayjs.helper';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { ContentCard } from '@/components/custom/content-card';
import { TagChips } from '@/features/tags/components';

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
        <div className="flex flex-row flex-wrap items-center gap-x-6 gap-y-3 px-4 py-4">
          {/* Logo + status dot */}
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
          <div className="min-w-0 flex-1 basis-5/12 space-y-1.5">
            <div className="flex items-baseline gap-2">
              <Link
                to={`/platforms/edit/${platform.id}`}
                className="truncate text-sm font-medium text-foreground hover:underline">
                {platform.name}
              </Link>
              <span className="truncate text-xs text-muted-foreground">
                {descriptor.operatingSystem} v{platform.serverVersion}
              </span>

              <span className="inline-flex items-center gap-1 text-xs text-muted-foreground truncate">
                <PlugZap className="h-3.5 w-3.5" />{' '}
                {getConnectorLabel(platform.connectorType, platform.agentVersion)}
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
            <TagChips tags={platform.tags} />
          </div>

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
            <div className="flex items-center gap-3">
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
          <div className="flex w-full flex-col gap-3 border-t border-border/60 pt-4 lg:w-65 lg:shrink-0 lg:border-t-0 lg:border-l lg:pt-0 lg:pl-6">
            <UsageMetric
              icon={Cpu}
              label="CPU"
              value={cpuUsage}
              online={isOnline}
              detail={`${platform.cpuCount ?? '-'} cores`}
            />
            <UsageMetric
              icon={MemoryStick}
              label="RAM"
              value={memUsage}
              online={isOnline}
              detail={`${byteTransform(platform.memTotal)} total`}
            />
          </div>

          <RowActionMenu resource={platform} actions={actions} />
        </div>
      </TooltipProvider>
    </ContentCard>
  );
};

const StatSection = ({ label, children }: { label: string; children: React.ReactNode }) => (
  <div className="flex flex-auto flex-col items-center gap-0.5">
    <span className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">{label}</span>
    <div className="text-sm text-foreground">{children}</div>
  </div>
);

const UsageMetric = ({
  icon: Icon,
  label,
  value,
  online,
  detail,
}: {
  icon: React.FC<{ className?: string }>;
  label: string;
  value: number;
  online: boolean;
  detail?: string;
}) => {
  return (
    <div className="flex items-center justify-between gap-3 text-xs text-muted-foreground">
      <span className="flex min-w-0 items-center gap-1">
        <Icon className="h-3 w-3 shrink-0" /> <span>{label}</span>
        {detail ? <span className="truncate text-muted-foreground/60">({detail})</span> : null}
      </span>
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

const getConnectorLabel = (connectorType: PlatformConnectorType, agentVersion?: string | null) => {
  if (connectorType === PlatformConnectorType.Agent) {
    return <span>agent v{agentVersion ?? '-'}</span>;
  }

  if (connectorType === PlatformConnectorType.EdgeAgent) {
    return <span>edge agent v{agentVersion ?? '-'}</span>;
  }

  return 'Local';
};
