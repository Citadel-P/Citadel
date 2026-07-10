import {
  PlatformDescriptorDockerPlatformDescriptor,
  PlatformStatView,
  PlatformStatus,
  PlatformView,
} from '@/api/generated/api.types';
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
import { ReactNode, useEffect, useMemo, useRef, useState } from 'react';
import { Area, AreaChart, CartesianGrid, XAxis } from 'recharts';

type PlatformStatsDatum = {
  created: number;
  cpuUsage: number;
  memoryUsage: number;
  rxBytes: number;
  txBytes: number;
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
  const network = usePlatformStatsWindow(platform.id);
  const cpuStats = useCombinedStats(cpu.baseStats, liveStats);
  const memoryStats = useCombinedStats(memory.baseStats, liveStats);
  const networkStats = useCombinedStats(network.baseStats, liveStats);

  return (
    <div className="flex flex-col gap-4">
      <PlatformResourceSummary platform={platform} />
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

const PlatformResourceSummary = ({ platform }: { platform: PlatformView }) => {
  const descriptor = platform.platformDescriptor as PlatformDescriptorDockerPlatformDescriptor | null;

  return (
    <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
      <Metric label="CPU" value={platform.cpuCount ?? '-'} detail="cores" />
      <Metric label="Memory" value={byteTransform(platform.memTotal, 2)} detail="total" />
      <Metric label="Containers" value={descriptor?.containerCount ?? '-'} detail="known" />
      <Metric label="Images" value={platform.imageCount ?? '-'} detail="known" />
      <Metric label="Volumes" value={platform.volumeCount ?? '-'} detail="known" />
      <Metric label="Networks" value={platform.networkCount ?? '-'} detail="known" />
      <Metric label="Agent" value={platform.agentVersion ?? '-'} detail="version" />
      <Metric label="Docker" value={platform.serverVersion ?? '-'} detail="version" />
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
          color: 'var(--chart-3)',
        },
        txBytes: {
          label: <span className="text-foreground">Data sent</span>,
          color: 'var(--chart-4)',
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
}: {
  title: string;
  description: string;
  controls?: ReactNode;
  chartConfig: ChartConfig;
  stats: PlatformStatsDatum[];
  isLoading: boolean;
  areas: StatsChartArea[];
  children?: ReactNode;
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
      <CardContent className="px-2 sm:px-6">
        <ChartContainer config={chartConfig} className="aspect-auto h-62.5 w-full">
          {memoizedChart}
        </ChartContainer>
      </CardContent>
    </Card>
  );
};

const Metric = ({ label, value, detail }: { label: string; value: React.ReactNode; detail?: string }) => (
  <div className="rounded-sm border bg-background px-4 py-3 shadow-xs">
    <div className="text-xs text-muted-foreground">{label}</div>
    <div className="mt-1 flex items-baseline gap-2">
      <span className="truncate text-sm font-medium">{value}</span>
      {detail && <span className="text-xs text-muted-foreground">{detail}</span>}
    </div>
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

const normalizeStats = (stats?: PlatformStatView[] | null): PlatformStatsDatum[] =>
  (stats ?? [])
    .map((stat) => ({
      created: Number(stat.created ?? 0),
      cpuUsage: Number(stat.cpuUsage ?? 0),
      memoryUsage: Number(stat.memoryUsage ?? 0),
      rxBytes: Number(stat.rxBytes ?? 0),
      txBytes: Number(stat.txBytes ?? 0),
    }))
    .filter((stat) => stat.created > 0)
    .sort((a, b) => a.created - b.created);

const formatPercent = (value: number) => `${toFixedNumber(value, undefined, 2)}%`;
const formatTick = (timestamp: number) => dayjs(timestamp * 1000).format('HH:mm:ss');
const formatTooltipLabel = (_: unknown, payload: any[]) => {
  const created = Number(payload.at(0)?.payload?.created ?? 0);
  return <span className="text-foreground">{created ? dayjs(created * 1000).format('HH:mm:ss') : '-'}</span>;
};
