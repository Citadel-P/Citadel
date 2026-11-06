import { statsChartData, statsValueAxis } from '@/lib/stats-chart';
import { StatsChartRange, useStatsChartBounds } from '@/components/custom/stats-chart-range';
import { useStatsRange } from '@/hooks/useStatsRange';
import { filterStatsRange, statsAxis, statsBucketSeconds } from '@/lib/stats-range';
import {
  PlatformBackupSummary,
  PlatformConnectorType,
  PlatformStatView,
  PlatformStatus,
  PlatformType,
  PlatformView,
  SwarmOverviewView,
} from '@/api/generated/api.types';
import { IoUsage } from '@/components/custom/stats-io-usage';
import { appendPlatformIo, platformIoFields, type PlatformIoSeries } from '../platform-io';
import { StatsPanelHeader, StatsSummaryItem } from '@/components/custom/common';
import {
  CHART_ACTIVE_DOT,
  CHART_HOVER_CURSOR,
  ChartConfig,
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
} from '@/components/ui/chart';
import { Card, CardContent } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { TooltipProvider } from '@/components/ui/tooltip';
import { byteTransform } from '@/lib/bytes.helper';
import { useRead } from '@/lib/hooks';
import { STREAMED_STATS_QUERY_OPTIONS } from '@/lib/live-stats';
import { appendLiveStatBucket, mergeStatsBuckets } from '@/lib/stats-buckets';
import { cn, toFixedNumber } from '@/lib/utils';
import dayjs from 'dayjs';
import {
  Box,
  ArchiveRestore,
  Cpu,
  Database,
  HardDrive,
  ImageIcon,
  Layers,
  MemoryStick,
  Network,
  PlugZap,
  Rocket,
  Server,
} from 'lucide-react';
import { ReactNode, useMemo, useState } from 'react';
import { Link } from 'react-router';
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from 'recharts';
import {
  getContainerStates,
  getDeploymentStates,
  getStackStates,
  PLATFORM_WORKLOAD_ICON_CLASS_NAMES,
  WorkloadState,
  WorkloadStatusBreakdown,
} from '../platform-workload-status';
import { getBackupMetric, getBackupSourceStates } from '../platform-backups';
import { useSwarmOverview } from '@/features/swarm/hooks/useSwarmOverview';

type PlatformStatsDatum = Pick<PlatformStatView, (typeof platformIoFields)[number]> & {
  created: number;
  cpuUsage: number | null;
  memoryUsage: number | null;
  rxBytes: number | null;
  txBytes: number | null;
  diskUsedBytes: number | null;
  diskTotalBytes: number | null;
  diskUsage: number | null;
};

export const PlatformStatsTab = ({ platform }: { platform: PlatformView }) => {
  const range = useStatsRange();
  const liveStats = useLiveStats(platform, statsBucketSeconds(range.bounds), range.hours * 3600);
  const readArgs = useMemo(() => ({ id: platform.id, query: range.query }), [platform.id, range.query]);
  const { data, isLoading, error } = useRead('getPlatformStats', readArgs, {
    ...STREAMED_STATS_QUERY_OPTIONS,
    enabled: Boolean(platform.id),
  });
  const combined = useCombinedStats(
    data?.data?.stats ?? [],
    range.live ? liveStats : [],
    statsBucketSeconds(range.bounds),
  );
  const stats = filterStatsRange(combined, range.bounds);
  const chartPlatform = range.live
    ? platform
    : { ...platform, status: PlatformStatus.Online, stats: stats.length ? [stats[stats.length - 1]] : [] };
  return (
    <div className="flex flex-col gap-4">
      {platform.type === PlatformType.DockerSwarm && (
        <p className="text-sm text-muted-foreground">
          Network and disk I/O include containers on the connected host and Swarm nodes reporting through node Agents.
          Missing or disconnected node Agents leave gaps. CPU, memory, and disk capacity describe the connected host.
        </p>
      )}
      <StatsChartRange range={range} error={error}>
        {!isLoading && stats.length === 0 ? (
          <div className="rounded-md border border-dashed px-6 py-12 text-center text-sm text-muted-foreground">
            No metrics recorded in this range. Try another time range.
          </div>
        ) : (
          <>
            <CpuUsageChart platform={chartPlatform} stats={stats} isLoading={isLoading} />
            <MemoryUsageChart platform={chartPlatform} stats={stats} isLoading={isLoading} />
            <DiskUsageChart platform={chartPlatform} stats={stats} isLoading={isLoading} />
            <IoUsage kind="network" stats={stats} isLoading={isLoading} />
            <IoUsage kind="disk" stats={stats} isLoading={isLoading} />
          </>
        )}
      </StatsChartRange>
    </div>
  );
};

type PlatformResourceSummaryProps = {
  platform: PlatformView;
  backupSummary?: PlatformBackupSummary;
  isBackupSummaryLoading?: boolean;
  isBackupSummaryError?: boolean;
};

export const PlatformResourceSummary = (props: PlatformResourceSummaryProps) =>
  props.platform.type === PlatformType.DockerSwarm ? (
    <SwarmPlatformResourceSummary {...props} />
  ) : (
    <PlatformResourceSummaryContent {...props} />
  );

const SwarmPlatformResourceSummary = (props: PlatformResourceSummaryProps) => {
  const { overview } = useSwarmOverview(props.platform.id);
  return <PlatformResourceSummaryContent {...props} swarmOverview={overview} />;
};

const PlatformResourceSummaryContent = ({
  platform,
  backupSummary,
  isBackupSummaryLoading = false,
  isBackupSummaryError = false,
  swarmOverview,
}: PlatformResourceSummaryProps & { swarmOverview?: SwarmOverviewView }) => {
  const descriptor = platform.platformDescriptor?.$type !== 'Kubernetes' ? platform.platformDescriptor : null;
  const currentDisk = getCurrentDiskUsage(platform);
  const resourceMetrics: ResourceMetric[] = [
    {
      icon: Box,
      label: 'Containers',
      value: descriptor?.containerCount ?? '-',
      to: `/platforms/${platform.id}/containers`,
      iconClassName: PLATFORM_WORKLOAD_ICON_CLASS_NAMES.containers,
      states: getContainerStates({
        running: descriptor?.containersRunning,
        stopped: descriptor?.containersStopped,
        paused: descriptor?.containersPaused,
      }),
    },
    {
      icon: Rocket,
      label: 'Deployments',
      value: platform.deploymentStatusCounts?.total ?? platform.deploymentCount ?? 0,
      to: `/deployments?platformId=${platform.id}`,
      iconClassName: PLATFORM_WORKLOAD_ICON_CLASS_NAMES.deployments,
      states: getDeploymentStates(platform.deploymentStatusCounts),
    },
    {
      icon: Layers,
      label: 'Stacks',
      value: platform.stackStatusCounts?.total ?? platform.stackCount ?? 0,
      to: `/stacks?platformId=${platform.id}`,
      iconClassName: PLATFORM_WORKLOAD_ICON_CLASS_NAMES.stacks,
      states: getStackStates(platform.stackStatusCounts),
    },
    {
      icon: ImageIcon,
      label: 'Images',
      value: swarmOverview?.imageCount ?? platform.imageCount ?? '-',
      to: `/platforms/${platform.id}/images`,
      iconClassName: PLATFORM_WORKLOAD_ICON_CLASS_NAMES.images,
      detail: formatStorageUsage(descriptor?.imageUsedBytes),
      detailTitle: 'Disk space used by Docker image layers',
    },
    {
      icon: HardDrive,
      label: 'Volumes',
      value: swarmOverview?.volumeCount ?? platform.volumeCount ?? '-',
      to: `/platforms/${platform.id}/volumes`,
      iconClassName: PLATFORM_WORKLOAD_ICON_CLASS_NAMES.volumes,
      detail: formatStorageUsage(descriptor?.volumeUsedBytes),
      detailTitle: 'Disk space used by Docker local volumes',
    },
  ];

  if (platform.type !== PlatformType.DockerSwarm) {
    const backupMetric = getBackupMetric(backupSummary, isBackupSummaryLoading, isBackupSummaryError);
    resourceMetrics.push(
      {
        icon: Network,
        label: 'Networks',
        value: platform.networkCount ?? '-',
        to: `/platforms/${platform.id}/networks`,
        iconClassName: PLATFORM_WORKLOAD_ICON_CLASS_NAMES.networks,
      },
      {
        icon: ArchiveRestore,
        label: 'Backups',
        value: backupMetric.value,
        to: '/backup-policies',
        iconClassName: PLATFORM_WORKLOAD_ICON_CLASS_NAMES.backups,
        states: getBackupSourceStates(backupSummary),
        detail: backupMetric.detail,
        detailTitle: backupMetric.detailTitle,
      },
    );
  }

  return (
    <TooltipProvider delayDuration={200}>
      <div className="space-y-4">
        <div
          className={cn(
            'grid gap-px overflow-hidden rounded-lg border bg-border shadow-xs sm:grid-cols-2 lg:grid-cols-4',
            platform.type === PlatformType.DockerSwarm ? 'xl:grid-cols-5' : 'xl:grid-cols-7',
          )}>
          {resourceMetrics.map((metric) => (
            <PlatformResourceMetric key={metric.label} {...metric} />
          ))}
        </div>
        <div className="grid gap-4 rounded-lg border bg-card p-(--surface-padding) shadow-xs sm:grid-cols-2 xl:grid-cols-5">
          <SystemMetric icon={Cpu} label="CPU" value={`${platform.cpuCount ?? '-'} cores`} />
          <SystemMetric icon={MemoryStick} label="Memory" value={byteTransform(platform.memTotal, 2)} />
          <SystemMetric
            icon={Database}
            label="Disk"
            value={
              currentDisk
                ? `${byteTransform(currentDisk.usedBytes, 2)} / ${byteTransform(currentDisk.totalBytes, 2)}`
                : '-'
            }
            title={
              currentDisk
                ? `Docker storage filesystem, ${formatPercent(currentDisk.usagePercent)} used`
                : 'Disk metrics unavailable. Expose the Docker host filesystem through CITADEL_HOST_ROOT (default /host). In Core or Agent containers, mount /:/host:ro. Native Core connected to Docker Desktop needs an Agent on that host.'
            }
          />
          <SystemMetric icon={PlugZap} label="Connector" value={formatPlatformConnector(platform)} />
          <SystemMetric icon={Server} label="Docker" value={platform.serverVersion ?? '-'} />
        </div>
      </div>
    </TooltipProvider>
  );
};

const CpuUsageChart = ({
  platform,
  stats,
  isLoading,
  controls,
}: {
  platform: PlatformView;
  stats: PlatformStatsDatum[];
  isLoading: boolean;
  controls?: ReactNode;
}) => {
  // Summary values are current samples; chart history contains bucket averages.
  const latest = platform.stats?.at(0);
  const chartConfig = useMemo(
    () =>
      ({
        cpuUsage: {
          label: 'CPU Usage',
          color: 'var(--chart-1)',
        },
      }) satisfies ChartConfig,
    [],
  );

  return (
    <StatsChartCard
      title="CPU Usage"
      description="Showing container CPU usage as a share of platform capacity over the selected period"
      controls={controls}
      chartConfig={chartConfig}
      stats={stats}
      isLoading={isLoading}
      areas={[
        {
          key: 'cpuUsage',
          gradientId: 'fillPlatformCpuUsage',
          formatter: (value) => formatPercent(Number(value)),
        },
      ]}>
      <StatsSummaryItem
        label="Usage"
        value={
          platform.status === PlatformStatus.Online && latest?.cpuUsage != null ? formatPercent(latest.cpuUsage) : '-'
        }
      />
    </StatsChartCard>
  );
};

const MemoryUsageChart = ({
  platform,
  stats,
  isLoading,
  controls,
}: {
  platform: PlatformView;
  stats: PlatformStatsDatum[];
  isLoading: boolean;
  controls?: ReactNode;
}) => {
  // Summary values are current samples; chart history contains bucket averages.
  const latest = platform.stats?.at(0);
  const chartConfig = useMemo(
    () =>
      ({
        memoryUsage: {
          label: 'Memory Usage',
          color: 'var(--chart-2)',
        },
      }) satisfies ChartConfig,
    [],
  );

  return (
    <StatsChartCard
      title="Memory Usage"
      description="Showing container working-set memory over the selected period"
      controls={controls}
      chartConfig={chartConfig}
      stats={stats}
      isLoading={isLoading}
      areas={[
        {
          key: 'memoryUsage',
          gradientId: 'fillPlatformMemoryUsage',
          formatter: (value) => formatPercent(Number(value)),
        },
      ]}>
      <StatsSummaryItem
        label="Usage"
        value={
          platform.status === PlatformStatus.Online && latest?.memoryUsage != null
            ? formatPercent(latest.memoryUsage)
            : '-'
        }
      />
      <StatsSummaryItem
        label="Used"
        value={
          platform.status === PlatformStatus.Online && latest?.memoryUsage != null && platform.memTotal > 0
            ? byteTransform((latest.memoryUsage / 100) * platform.memTotal, 2)
            : '-'
        }
      />
      <StatsSummaryItem label="Total" value={byteTransform(platform.memTotal, 2)} />
    </StatsChartCard>
  );
};

export const DiskUsageChart = ({
  platform,
  stats,
  isLoading,
  controls,
}: {
  platform: PlatformView;
  stats: PlatformStatsDatum[];
  isLoading: boolean;
  controls?: ReactNode;
}) => {
  const current = getCurrentDiskUsage(platform);
  const hasHistoricalData = stats.some((stat) => stat.diskUsage !== null);
  const chartConfig = useMemo(
    () =>
      ({
        diskUsage: {
          label: 'Disk Usage',
          color: 'var(--chart-3)',
        },
      }) satisfies ChartConfig,
    [],
  );
  const unavailable = (
    <DiskChartState compact={hasHistoricalData} title="Disk metrics unavailable">
      Expose the Docker host filesystem through CITADEL_HOST_ROOT (default /host). In Core or Agent containers, mount
      /:/host:ro. Native Core connected to Docker Desktop needs an Agent on that host.
    </DiskChartState>
  );
  const noHistory = (
    <DiskChartState compact={false} title="No historical disk readings yet.">
      Current disk metrics are available.
    </DiskChartState>
  );

  return (
    <StatsChartCard
      title="Disk Usage"
      description="Showing Docker storage filesystem usage over the selected period"
      controls={controls}
      chartConfig={chartConfig}
      stats={stats}
      isLoading={isLoading}
      notice={!current && hasHistoricalData ? unavailable : undefined}
      emptyState={!hasHistoricalData ? (current ? noHistory : unavailable) : undefined}
      areas={[
        {
          key: 'diskUsage',
          gradientId: 'fillPlatformDiskUsage',
          formatter: (value) => formatPercent(Number(value)),
        },
      ]}>
      <StatsSummaryItem label="Usage" value={current ? formatPercent(current.usagePercent) : '-'} />
      <StatsSummaryItem label="Used" value={current ? byteTransform(current.usedBytes, 2) : '-'} />
      <StatsSummaryItem label="Total" value={current ? byteTransform(current.totalBytes, 2) : '-'} />
      <StatsSummaryItem label="Available" value={current ? byteTransform(current.availableBytes, 2) : '-'} />
    </StatsChartCard>
  );
};

type StatsChartArea = {
  key: keyof Omit<PlatformStatsDatum, 'created'>;
  gradientId: string;
  formatter: (value: unknown) => ReactNode;
};

const StatsChartCard = ({
  title,
  description,
  controls,
  chartConfig,
  stats,
  isLoading,
  areas,
  children,
  notice,
  emptyState,
}: {
  title: string;
  description: string;
  controls?: ReactNode;
  chartConfig: ChartConfig;
  stats: PlatformStatsDatum[];
  isLoading: boolean;
  areas: StatsChartArea[];
  children?: ReactNode;
  notice?: ReactNode;
  emptyState?: ReactNode;
}) => {
  const bounds = useStatsChartBounds();
  const gradientDefs = useMemo(
    () => (
      <defs>
        {areas.map((area) => (
          <linearGradient key={area.gradientId} id={area.gradientId} x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor={`var(--color-${area.key})`} stopOpacity={0.18} />
            <stop offset="95%" stopColor={`var(--color-${area.key})`} stopOpacity={0.02} />
          </linearGradient>
        ))}
      </defs>
    ),
    [areas],
  );

  const memoizedChart = useMemo(
    () => (
      <AreaChart data={statsChartData(stats, bounds)} accessibilityLayer>
        {gradientDefs}
        <CartesianGrid vertical={false} strokeDasharray="3 3" />
        <YAxis {...statsValueAxis('percent')} />
        <XAxis
          {...statsAxis(bounds)}
          dataKey="created"
          tickLine={false}
          axisLine={false}
          tickMargin={8}
          minTickGap={32}
          tickFormatter={(timestamp) =>
            dayjs(Number(timestamp) * 1000).format(bounds && bounds.to - bounds.from > 86400 ? 'MMM D HH:mm' : 'HH:mm')
          }
        />
        <ChartTooltip
          cursor={CHART_HOVER_CURSOR}
          content={
            <ChartTooltipContent
              indicator="dot"
              labelFormatter={formatTooltipLabel}
              valueFormatter={(value, name) => {
                const key = name as StatsChartArea['key'];
                const area = areas.find((item) => item.key === key);
                return area?.formatter(value) ?? String(value);
              }}
            />
          }
        />
        {areas.map((area) => (
          <Area
            key={area.key}
            dataKey={area.key}
            type="linear"
            isAnimationActive={false}
            connectNulls={false}
            activeDot={CHART_ACTIVE_DOT}
            fill={`url(#${area.gradientId})`}
            stroke={`var(--color-${area.key})`}
          />
        ))}
        <ChartLegend content={<ChartLegendContent />} />
      </AreaChart>
    ),
    [areas, gradientDefs, stats, bounds],
  );

  return isLoading ? (
    <Skeleton className="h-56.25 w-full rounded-xl" />
  ) : (
    <Card className="bg-background rounded-sm shadow-xs py-0">
      <StatsPanelHeader title={title} description={description} controls={controls}>
        {children}
      </StatsPanelHeader>
      {notice}
      <CardContent className="px-2 sm:px-6">
        {emptyState ?? (
          <ChartContainer config={chartConfig} className="aspect-auto h-62.5 w-full">
            {memoizedChart}
          </ChartContainer>
        )}
      </CardContent>
    </Card>
  );
};

export const PlatformResourceMetric = ({
  icon: Icon,
  label,
  value,
  to,
  iconClassName,
  states,
  detail,
  detailTitle,
}: ResourceMetric) => {
  const content = (
    <div className="flex min-w-0 items-start gap-3">
      <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-accent/60">
        <Icon className={cn('h-4 w-4 text-muted-foreground', iconClassName)} />
      </span>
      <div className="min-w-0">
        <div className="truncate text-[11px] font-medium uppercase text-muted-foreground">{label}</div>
        <div className="mt-0.5 flex min-w-0 flex-wrap items-center gap-1.5">
          <span className="truncate text-xl font-semibold tabular-nums text-foreground">{value}</span>
          {detail !== undefined && (
            <>
              <span className="h-3 w-px shrink-0 bg-border" />
              <span className="truncate text-xs font-medium tabular-nums text-muted-foreground" title={detailTitle}>
                {detail}
              </span>
            </>
          )}
          {states && <WorkloadStatusBreakdown states={states} />}
        </div>
      </div>
    </div>
  );

  return (
    <Link
      to={to}
      className="min-w-0 bg-card p-4 transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
      {content}
    </Link>
  );
};

export type ResourceMetric = {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: React.ReactNode;
  to: string;
  iconClassName?: string;
  states?: WorkloadState[];
  detail?: React.ReactNode;
  detailTitle?: string;
};

const formatStorageUsage = (value: number | string | null | undefined): string => {
  if (value == null) return 'N/A';

  const bytes = Number(value);
  return Number.isFinite(bytes) && bytes >= 0 ? byteTransform(bytes, 1) : 'N/A';
};

export const formatPlatformConnector = (platform: PlatformView): string => {
  if (platform.connectorType === PlatformConnectorType.Local) return 'Local';

  const name = platform.connectorType === PlatformConnectorType.EdgeAgent ? 'Edge agent' : 'Agent';
  return platform.agentVersion ? `${name} v${platform.agentVersion}` : name;
};

const SystemMetric = ({
  icon: Icon,
  label,
  value,
  title,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: React.ReactNode;
  title?: string;
}) => (
  <div className="min-w-0 space-y-2" title={title}>
    <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
      <Icon className="size-3.5 shrink-0" />
      <span>{label}</span>
    </div>
    <div className="text-sm font-medium tabular-nums text-foreground [overflow-wrap:anywhere]">{value}</div>
  </div>
);

const useLiveStats = (platform: PlatformView, seconds: number, windowSeconds: number): PlatformStatsDatum[] => {
  const latestStat = platform.stats?.at(0);
  const nextState = (previous?: { gauges: PlatformStatsDatum[]; io: PlatformIoSeries }) => {
    const normalized =
      latestStat && (latestStat.cpuUsage != null || latestStat.memoryUsage != null)
        ? normalizePlatformStats([latestStat])[0]
        : undefined;
    const gauges = normalized
      ? appendLiveStatBucket(previous?.gauges ?? [], normalized, seconds, windowSeconds)
      : (previous?.gauges ?? []);
    const { series: io, buckets } = appendPlatformIo(previous?.io, latestStat, seconds, windowSeconds);
    const byBucket = new Map(buckets.map((point) => [Math.floor(point.created / seconds), point]));
    const combined = new Map(gauges.map((point) => [Math.floor(point.created / seconds), point]));
    for (const point of buckets) {
      const bucket = Math.floor(point.created / seconds);
      if (!combined.has(bucket))
        combined.set(bucket, {
          created: point.created,
          cpuUsage: null,
          memoryUsage: null,
          rxBytes: null,
          txBytes: null,
          diskUsage: null,
          diskUsedBytes: null,
          diskTotalBytes: null,
        });
    }
    return {
      platformId: platform.id,
      latestStat,
      seconds,
      gauges,
      io,
      stats: [...combined.values()]
        .sort((a, b) => a.created - b.created)
        .map((point) => {
          const measured = byBucket.get(Math.floor(point.created / seconds));
          return { ...point, ...Object.fromEntries(platformIoFields.map((key) => [key, measured?.[key] ?? null])) };
        }),
    };
  };
  const [live, setLive] = useState(() => nextState());
  if (live.platformId !== platform.id || live.latestStat !== latestStat || live.seconds !== seconds) {
    const next = nextState(live.platformId === platform.id && live.seconds === seconds ? live : undefined);
    setLive(next);
    return next.stats;
  }
  return live.stats;
};

const useCombinedStats = (baseStats: PlatformStatView[], liveStats: PlatformStatsDatum[], seconds: number) => {
  const normalizedBaseStats = useMemo(() => normalizePlatformStats(baseStats), [baseStats]);
  return useMemo(
    () => mergeStatsBuckets(normalizedBaseStats, liveStats, seconds),
    [liveStats, normalizedBaseStats, seconds],
  );
};

export const normalizePlatformStats = (stats?: PlatformStatView[] | null): PlatformStatsDatum[] =>
  (stats ?? [])
    .map((stat) => {
      const diskUsage = stat.diskUsage == null ? null : Number(stat.diskUsage);
      const diskUsedBytes = stat.diskUsedBytes == null ? null : Number(stat.diskUsedBytes);
      const diskTotalBytes = stat.diskTotalBytes == null ? null : Number(stat.diskTotalBytes);
      const hasCompleteDiskSample =
        diskUsage != null &&
        Number.isFinite(diskUsage) &&
        diskUsage >= 0 &&
        diskUsage <= 100 &&
        diskUsedBytes != null &&
        Number.isFinite(diskUsedBytes) &&
        diskUsedBytes >= 0 &&
        diskTotalBytes != null &&
        Number.isFinite(diskTotalBytes) &&
        diskTotalBytes > 0 &&
        diskUsedBytes <= diskTotalBytes;

      return {
        ...Object.fromEntries(platformIoFields.map((key) => [key, stat[key] ?? null])),
        // Explicit unknown deltas prevent deriving rates from aggregate lifetime counters.
        rxBytesDelta: stat.rxBytesDelta ?? null,
        txBytesDelta: stat.txBytesDelta ?? null,
        rxBytesPerSecond: stat.rxBytesPerSecond ?? null,
        txBytesPerSecond: stat.txBytesPerSecond ?? null,
        created: Number(stat.created ?? 0),
        cpuUsage: stat.cpuUsage == null ? null : Number(stat.cpuUsage),
        memoryUsage: stat.memoryUsage == null ? null : Number(stat.memoryUsage),
        rxBytes: stat.rxBytes == null ? null : Number(stat.rxBytes),
        txBytes: stat.txBytes == null ? null : Number(stat.txBytes),
        diskUsedBytes: hasCompleteDiskSample ? diskUsedBytes : null,
        diskTotalBytes: hasCompleteDiskSample ? diskTotalBytes : null,
        diskUsage: hasCompleteDiskSample ? diskUsage : null,
      };
    })
    .filter((stat) => stat.created > 0)
    .sort((a, b) => a.created - b.created);

export const getCurrentDiskUsage = (platform: PlatformView) => {
  const stat = platform.stats?.at(0);
  if (
    platform.status !== PlatformStatus.Online ||
    stat?.diskUsage == null ||
    stat.diskUsedBytes == null ||
    stat.diskTotalBytes == null
  ) {
    return null;
  }

  const usagePercent = Number(stat.diskUsage);
  const usedBytes = Number(stat.diskUsedBytes);
  const totalBytes = Number(stat.diskTotalBytes);
  if (
    !Number.isFinite(usagePercent) ||
    !Number.isFinite(usedBytes) ||
    !Number.isFinite(totalBytes) ||
    usagePercent < 0 ||
    usagePercent > 100 ||
    usedBytes < 0 ||
    totalBytes <= 0 ||
    usedBytes > totalBytes
  ) {
    return null;
  }

  return {
    usagePercent,
    usedBytes,
    totalBytes,
    availableBytes: Math.max(totalBytes - usedBytes, 0),
  };
};

const DiskChartState = ({ compact, title, children }: { compact: boolean; title: string; children: ReactNode }) => (
  <div
    className={
      compact
        ? 'border-t px-6 py-3 text-xs text-muted-foreground'
        : 'flex h-62.5 flex-col items-center justify-center px-6 text-center'
    }>
    <div className="font-medium text-foreground">{title}</div>
    <div className="mt-1 text-muted-foreground">{children}</div>
  </div>
);

const formatPercent = (value: number) => `${toFixedNumber(value, undefined, 2)}%`;
const formatTooltipLabel = (_: unknown, payload: readonly any[]) => {
  const created = Number(payload.at(0)?.payload?.created ?? 0);
  return created ? dayjs(created * 1000).format('MMM D, h:mm:ss A') : '-';
};
