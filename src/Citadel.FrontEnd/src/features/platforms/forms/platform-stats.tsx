import {
  PlatformDescriptorDockerPlatformDescriptor,
  PlatformStatView,
  PlatformStatus,
  PlatformView,
} from '@/api/generated/api.types';
import { NETWORK_CHART_COLORS } from '@/components/custom/chart-series-colors';
import { StatsPanelHeader, StatsSummaryItem, StatsWindowHours, StatsWindowSelect } from '@/components/custom/common';
import {
  ChartConfig,
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
} from '@/components/ui/chart';
import { Card, CardContent } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { byteTransform } from '@/lib/bytes.helper';
import { useRead } from '@/lib/hooks';
import { toFixedNumber } from '@/lib/utils';
import dayjs from 'dayjs';
import {
  Boxes,
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
import { ReactNode, useEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router';
import { Area, AreaChart, CartesianGrid, XAxis } from 'recharts';

type PlatformStatsDatum = {
  created: number;
  cpuUsage: number;
  memoryUsage: number;
  rxBytes: number;
  txBytes: number;
  diskUsedBytes: number | null;
  diskTotalBytes: number | null;
  diskUsage: number | null;
};

type StatsQueryState = {
  baseStats: PlatformStatView[];
  isLoading: boolean;
  windowHours: StatsWindowHours;
  onWindowHoursChange: (hours: StatsWindowHours) => void;
};

export const PlatformStatsTab = ({ platform }: { platform: PlatformView }) => {
  const liveStats = useLiveStats(platform);
  const cpu = usePlatformStatsWindow(platform.id);
  const memory = usePlatformStatsWindow(platform.id);
  const disk = usePlatformStatsWindow(platform.id);
  const network = usePlatformStatsWindow(platform.id);
  const cpuStats = useCombinedStats(cpu.baseStats, liveStats);
  const memoryStats = useCombinedStats(memory.baseStats, liveStats);
  const diskStats = useCombinedStats(disk.baseStats, liveStats);
  const networkStats = useCombinedStats(network.baseStats, liveStats);

  return (
    <div className="flex flex-col gap-4">
      <CpuUsageChart
        platform={platform}
        stats={cpuStats}
        isLoading={cpu.isLoading}
        windowHours={cpu.windowHours}
        controls={<StatsWindowSelect value={cpu.windowHours} onChange={cpu.onWindowHoursChange} />}
      />
      <MemoryUsageChart
        platform={platform}
        stats={memoryStats}
        isLoading={memory.isLoading}
        windowHours={memory.windowHours}
        controls={<StatsWindowSelect value={memory.windowHours} onChange={memory.onWindowHoursChange} />}
      />
      <DiskUsageChart
        platform={platform}
        stats={diskStats}
        isLoading={disk.isLoading}
        windowHours={disk.windowHours}
        controls={<StatsWindowSelect value={disk.windowHours} onChange={disk.onWindowHoursChange} />}
      />
      <NetworkUsageChart
        platform={platform}
        stats={networkStats}
        isLoading={network.isLoading}
        windowHours={network.windowHours}
        controls={<StatsWindowSelect value={network.windowHours} onChange={network.onWindowHoursChange} />}
      />
    </div>
  );
};

export const PlatformResourceSummary = ({ platform }: { platform: PlatformView }) => {
  const descriptor = platform.platformDescriptor as PlatformDescriptorDockerPlatformDescriptor | null;
  const currentDisk = getCurrentDiskUsage(platform);
  const resourceMetrics = [
    {
      icon: Boxes,
      label: 'Containers',
      value: descriptor?.containerCount ?? '-',
      to: `/platforms/${platform.id}/containers`,
    },
    {
      icon: Rocket,
      label: 'Deployments',
      value: platform.deploymentCount ?? 0,
      to: `/deployments?platformId=${platform.id}`,
    },
    {
      icon: Layers,
      label: 'Stacks',
      value: platform.stackCount ?? 0,
      to: `/stacks?platformId=${platform.id}`,
    },
    {
      icon: ImageIcon,
      label: 'Images',
      value: platform.imageCount ?? '-',
      to: `/platforms/${platform.id}/images`,
    },
    {
      icon: HardDrive,
      label: 'Volumes',
      value: platform.volumeCount ?? '-',
      to: `/platforms/${platform.id}/volumes`,
    },
    {
      icon: Network,
      label: 'Networks',
      value: platform.networkCount ?? '-',
      to: `/platforms/${platform.id}/networks`,
    },
  ];

  return (
    <div className="space-y-3 py-3">
      <div className="grid gap-px overflow-hidden rounded-md border bg-border sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6">
        {resourceMetrics.map((metric) => (
          <Metric key={metric.label} {...metric} />
        ))}
      </div>
      <div className="flex flex-wrap items-center gap-x-5 gap-y-2 px-0.5 text-xs text-muted-foreground">
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
              : 'Disk metrics unavailable. Mount the host root at /host as read-only in the Citadel Core or Agent container.'
          }
        />
        <SystemMetric icon={PlugZap} label="Agent" value={platform.agentVersion ?? '-'} />
        <SystemMetric icon={Server} label="Docker" value={platform.serverVersion ?? '-'} />
      </div>
    </div>
  );
};

const CpuUsageChart = ({
  platform,
  stats,
  isLoading,
  windowHours,
  controls,
}: {
  platform: PlatformView;
  stats: PlatformStatsDatum[];
  isLoading: boolean;
  windowHours: StatsWindowHours;
  controls?: ReactNode;
}) => {
  const latest = stats.at(-1);
  const chartConfig = useMemo(
    () =>
      ({
        cpuUsage: {
          label: <span className="text-foreground">CPU Usage</span>,
          color: 'var(--chart-1)',
        },
      }) satisfies ChartConfig,
    [],
  );

  return (
    <StatsChartCard
      title="CPU Usage"
      description={`Showing total CPU usage for the past ${windowHours} hours`}
      controls={controls}
      chartConfig={chartConfig}
      stats={stats}
      isLoading={isLoading}
      areas={[
        {
          key: 'cpuUsage',
          gradientId: 'fillPlatformCpuUsage',
          stackId: 'a',
          formatter: (value) => formatPercent(Number(value)),
        },
      ]}>
      <StatsSummaryItem
        label="Usage"
        value={platform.status === PlatformStatus.Online && latest ? formatPercent(latest.cpuUsage) : '-'}
      />
    </StatsChartCard>
  );
};

const MemoryUsageChart = ({
  platform,
  stats,
  isLoading,
  windowHours,
  controls,
}: {
  platform: PlatformView;
  stats: PlatformStatsDatum[];
  isLoading: boolean;
  windowHours: StatsWindowHours;
  controls?: ReactNode;
}) => {
  const latest = stats.at(-1);
  const chartConfig = useMemo(
    () =>
      ({
        memoryUsage: {
          label: <span className="text-foreground">Memory Usage</span>,
          color: 'var(--chart-2)',
        },
      }) satisfies ChartConfig,
    [],
  );

  return (
    <StatsChartCard
      title="Memory Usage"
      description={`Showing total memory usage for the past ${windowHours} hours`}
      controls={controls}
      chartConfig={chartConfig}
      stats={stats}
      isLoading={isLoading}
      areas={[
        {
          key: 'memoryUsage',
          gradientId: 'fillPlatformMemoryUsage',
          stackId: 'a',
          formatter: (value) => formatPercent(Number(value)),
        },
      ]}>
      <StatsSummaryItem
        label="Usage"
        value={platform.status === PlatformStatus.Online && latest ? formatPercent(latest.memoryUsage) : '-'}
      />
      <StatsSummaryItem label="Total" value={byteTransform(platform.memTotal, 2)} />
    </StatsChartCard>
  );
};

const NetworkUsageChart = ({
  platform,
  stats,
  isLoading,
  windowHours,
  controls,
}: {
  platform: PlatformView;
  stats: PlatformStatsDatum[];
  isLoading: boolean;
  windowHours: StatsWindowHours;
  controls?: ReactNode;
}) => {
  const latest = stats.at(-1);
  const chartConfig = useMemo(
    () =>
      ({
        rxBytes: {
          label: <span className="text-foreground">Data received</span>,
          theme: NETWORK_CHART_COLORS.rxBytes,
        },
        txBytes: {
          label: <span className="text-foreground">Data sent</span>,
          theme: NETWORK_CHART_COLORS.txBytes,
        },
      }) satisfies ChartConfig,
    [],
  );

  return (
    <StatsChartCard
      title="Network Usage"
      description={`Showing total network usage for the past ${windowHours} hours`}
      controls={controls}
      chartConfig={chartConfig}
      stats={stats}
      isLoading={isLoading}
      areas={[
        {
          key: 'rxBytes',
          gradientId: 'fillPlatformRxBytes',
          stackId: 'a',
          formatter: (value) => byteTransform(value, 2),
        },
        {
          key: 'txBytes',
          gradientId: 'fillPlatformTxBytes',
          stackId: 'a',
          formatter: (value) => byteTransform(value, 2),
        },
      ]}>
      <StatsSummaryItem
        label="Received"
        value={platform.status === PlatformStatus.Online && latest ? byteTransform(latest.rxBytes, 2) : '-'}
      />
      <StatsSummaryItem
        label="Sent"
        value={platform.status === PlatformStatus.Online && latest ? byteTransform(latest.txBytes, 2) : '-'}
      />
    </StatsChartCard>
  );
};

export const DiskUsageChart = ({
  platform,
  stats,
  isLoading,
  windowHours,
  controls,
}: {
  platform: PlatformView;
  stats: PlatformStatsDatum[];
  isLoading: boolean;
  windowHours: StatsWindowHours;
  controls?: ReactNode;
}) => {
  const current = getCurrentDiskUsage(platform);
  const hasHistoricalData = stats.some((stat) => stat.diskUsage !== null);
  const chartConfig = useMemo(
    () =>
      ({
        diskUsage: {
          label: <span className="text-foreground">Disk Usage</span>,
          color: 'var(--chart-3)',
        },
      }) satisfies ChartConfig,
    [],
  );
  const unavailable = (
    <DiskChartState compact={hasHistoricalData} title="Disk metrics unavailable">
      Mount the host root at /host as read-only in the Citadel Core or Agent container.
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
      description={`Showing Docker storage filesystem usage for the past ${windowHours} hours`}
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
  stackId?: string;
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
  const gradientDefs = useMemo(
    () => (
      <defs>
        {areas.map((area) => (
          <linearGradient key={area.gradientId} id={area.gradientId} x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor={`var(--color-${area.key})`} stopOpacity={0.8} />
            <stop offset="95%" stopColor={`var(--color-${area.key})`} stopOpacity={0.1} />
          </linearGradient>
        ))}
      </defs>
    ),
    [areas],
  );

  const memoizedChart = useMemo(
    () => (
      <AreaChart data={stats} accessibilityLayer>
        {gradientDefs}
        <CartesianGrid vertical={true} />
        <XAxis
          dataKey="created"
          tickLine={false}
          axisLine={false}
          tickMargin={8}
          minTickGap={32}
          tickFormatter={formatTick}
        />
        <ChartTooltip
          cursor={false}
          defaultIndex={1}
          content={
            <ChartTooltipContent
              indicator="dot"
              labelFormatter={formatTooltipLabel}
              formatter={(value, name) => {
                const key = name as StatsChartArea['key'];
                const area = areas.find((item) => item.key === key);
                return (
                  <ChartValue
                    colorName={name}
                    label={chartConfig[key]?.label ?? name}
                    value={area?.formatter(value) ?? value}
                  />
                );
              }}
            />
          }
        />
        {areas.map((area) => (
          <Area
            key={area.key}
            dataKey={area.key}
            type="natural"
            fill={`url(#${area.gradientId})`}
            stroke={`var(--color-${area.key})`}
            stackId={area.stackId}
            connectNulls={false}
          />
        ))}
        <ChartLegend content={<ChartLegendContent />} />
      </AreaChart>
    ),
    [areas, chartConfig, gradientDefs, stats],
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

const Metric = ({
  icon: Icon,
  label,
  value,
  to,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: React.ReactNode;
  to: string;
}) => {
  const content = (
    <div className="flex min-w-0 items-center gap-3">
      <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-accent/60 text-muted-foreground">
        <Icon className="h-4 w-4" />
      </span>
      <div className="min-w-0">
        <div className="truncate text-[11px] font-medium uppercase text-muted-foreground">{label}</div>
        <div className="mt-0.5 truncate text-sm font-semibold tabular-nums text-foreground">{value}</div>
      </div>
    </div>
  );

  return (
    <Link
      to={to}
      className="min-w-0 bg-background p-3 transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
      {content}
    </Link>
  );
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
  <div className="inline-flex min-w-0 items-center gap-1.5" title={title}>
    <Icon className="h-3.5 w-3.5 shrink-0" />
    <span>{label}</span>
    <span className="max-w-48 truncate font-medium tabular-nums text-foreground">{value}</span>
  </div>
);

const ChartValue = ({
  colorName,
  label,
  value,
}: {
  colorName: unknown;
  label: React.ReactNode;
  value: React.ReactNode;
}) => (
  <>
    <div
      className="h-2.5 w-2.5 shrink-0 rounded-xs bg-(--color-bg)"
      style={
        {
          '--color-bg': `var(--color-${colorName})`,
        } as React.CSSProperties
      }
    />
    {label}
    <div className="ml-auto font-mono font-medium tabular-nums text-foreground">{value}</div>
  </>
);

const usePlatformStatsWindow = (platformId: string | undefined): StatsQueryState => {
  const [windowHours, setWindowHours] = useState<StatsWindowHours>(24);
  const readArgs = useMemo(() => ({ id: platformId, query: { hours: windowHours } }), [platformId, windowHours]);
  const { data, isLoading } = useRead('getPlatformStats', readArgs, { enabled: Boolean(platformId) });

  return {
    baseStats: data?.data?.stats ?? [],
    isLoading,
    windowHours,
    onWindowHoursChange: setWindowHours,
  };
};

const useLiveStats = (platform: PlatformView): PlatformStatView[] => {
  const [liveStats, setLiveStats] = useState<PlatformStatView[]>([]);
  const latestStat = platform.stats?.at(0);
  const lastStatRef = useRef<PlatformStatView | undefined>(latestStat);
  const platformIdRef = useRef<string | undefined>(platform.id);

  useEffect(() => {
    if (platformIdRef.current === platform.id) return;

    platformIdRef.current = platform.id;
    setLiveStats([]);
    lastStatRef.current = latestStat;
  }, [latestStat, platform.id]);

  useEffect(() => {
    if (platform.status !== PlatformStatus.Online || !latestStat || latestStat === lastStatRef.current) return;

    lastStatRef.current = latestStat;
    setLiveStats((prev) => [...prev, { ...latestStat, created: Math.floor(Date.now() / 1000) }]);
  }, [latestStat, platform.status]);

  return liveStats;
};

const useCombinedStats = (baseStats: PlatformStatView[], liveStats: PlatformStatView[]) =>
  useMemo(() => normalizeStats([...baseStats, ...liveStats]), [baseStats, liveStats]);

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
        created: Number(stat.created ?? 0),
        cpuUsage: Number(stat.cpuUsage ?? 0),
        memoryUsage: Number(stat.memoryUsage ?? 0),
        rxBytes: Number(stat.rxBytes ?? 0),
        txBytes: Number(stat.txBytes ?? 0),
        diskUsedBytes: hasCompleteDiskSample ? diskUsedBytes : null,
        diskTotalBytes: hasCompleteDiskSample ? diskTotalBytes : null,
        diskUsage: hasCompleteDiskSample ? diskUsage : null,
      };
    })
    .filter((stat) => stat.created > 0)
    .sort((a, b) => a.created - b.created);

const normalizeStats = normalizePlatformStats;

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
const formatTick = (timestamp: number) => dayjs(timestamp * 1000).format('HH:mm:ss');
const formatTooltipLabel = (_: unknown, payload: any[]) => {
  const created = Number(payload.at(0)?.payload?.created ?? 0);
  return <span className="text-foreground">{created ? dayjs(created * 1000).format('HH:mm:ss') : '-'}</span>;
};
