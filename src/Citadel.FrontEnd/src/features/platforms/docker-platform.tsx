import {
  PlatformConnectorType,
  PlatformDescriptorDockerPlatformDescriptor,
  PlatformStatus,
  PlatformView,
} from '@/api/generated/api.types';
import DockerIcon from '@/assets/docker.svg';
import { Link } from 'react-router';
import { Power, PowerOff, CirclePause, PlugZap, Cpu, MemoryStick } from 'lucide-react';
import { toFixedNumber } from '@/lib/utils';
import { byteTransform } from '@/lib/bytes.helper';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { fromNow } from '@/lib/dayjs.helper';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { ContentCard } from '@/components/custom/content-card';

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
                to={`/platforms/${platform.id}`}
                className="truncate text-sm font-medium text-foreground hover:underline">
                {platform.name}
              </Link>
              <span className="truncate text-xs text-muted-foreground">
                {descriptor.operatingSystem} v{platform.serverVersion}
              </span>

              <span className="inline-flex items-center gap-1 text-xs text-muted-foreground truncate">
                <PlugZap className="h-3.5 w-3.5" />{' '}
                {platform.connectorType === PlatformConnectorType.Agent ? (
                  <span>agent v{platform.agentVersion} </span>
                ) : (
                  'Local'
                )}
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
            <UsageBar
              icon={Cpu}
              label="CPU"
              value={cpuUsage}
              online={isOnline}
              detail={`${platform.cpuCount ?? '-'} cores`}
            />
            <UsageBar
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

const UsageBar = ({
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
  const barColor = value > 80 ? 'bg-rose-500' : value > 60 ? 'bg-amber-500' : 'bg-sky-500';

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <span className="flex items-center gap-1">
          <Icon className="h-3 w-3" /> {label}
          {detail ? <span className="text-muted-foreground/60">({detail})</span> : null}
        </span>
        <span className="tabular-nums text-foreground">{online ? `${value} %` : 'N/A'}</span>
      </div>
      <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
        <div
          className={`h-full rounded-full transition-all duration-500 ${barColor}`}
          style={{ width: `${online ? value : 0}%` }}
        />
      </div>
    </div>
  );
};

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
