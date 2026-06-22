import { ContainerDataView, ContainerStateStatus, ContainerStatView } from '@/api/generated/api.types';
import { StatsPanelHeader, StatsSummaryItem, StatsWindowHours, StatsWindowSelect } from '@/components/custom/common';
import { getContainerSeriesColor } from '@/components/custom/container-series-colors';
import { Card, CardContent } from '@/components/ui/card';
import { ChartConfig, ChartContainer, ChartLegend, ChartLegendContent, ChartTooltip } from '@/components/ui/chart';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import { byteTransform } from '@/lib/bytes.helper';
import { useRead } from '@/lib/hooks';
import { normalizeDockerId } from '@/lib/utils';
import dayjs from 'dayjs';
import { Dispatch, SetStateAction, useEffect, useMemo, useRef, useState } from 'react';
import { Area, AreaChart, CartesianGrid, XAxis } from 'recharts';

type ContainerSelection = 'all' | string;
type StatField = keyof Pick<
  ContainerStatView,
  'cpuUsage' | 'memoryActive' | 'memoryCache' | 'memoryLimit' | 'rxBytes' | 'txBytes'
>;

type StackStatsProps = {
  stackId: string;
  containers: ContainerDataView[];
};

type ContainerSeries = {
  id: string;
  name: string;
  stats: ContainerStatView[];
};

type MetricField = {
  key: StatField;
  label: string;
  color: string;
  aggregate?: 'sum' | 'max';
  formatter: (value: number) => string;
};

type MetricConfig = {
  kind: 'memory' | 'cpu' | 'network';
  title: string;
  description: string;
  fields: MetricField[];
  summaryFields: MetricField[];
  stacked?: boolean;
};

type ChartDatum = {
  created: number;
} & Partial<Record<StatField, number>>;

const ALL_CONTAINERS = 'all';

const STACK_STAT_METRICS: MetricConfig[] = [
  {
    kind: 'memory',
    title: 'Memory Usage',
    description: 'Showing memory usage for the selected range',
    stacked: true,
    fields: [
      {
        key: 'memoryActive',
        label: 'Active',
        color: 'var(--chart-1)',
        formatter: (value) => byteTransform(value, 2),
      },
      {
        key: 'memoryCache',
        label: 'Cache',
        color: 'var(--chart-2)',
        formatter: (value) => byteTransform(value, 2),
      },
    ],
    summaryFields: [
      {
        key: 'memoryActive',
        label: 'Active',
        color: 'var(--chart-1)',
        formatter: (value) => byteTransform(value, 2),
      },
      {
        key: 'memoryCache',
        label: 'Cache',
        color: 'var(--chart-2)',
        formatter: (value) => byteTransform(value, 2),
      },
      {
        key: 'memoryLimit',
        label: 'Limit',
        color: 'var(--chart-3)',
        aggregate: 'max',
        formatter: (value) => byteTransform(value, 2),
      },
    ],
  },
  {
    kind: 'cpu',
    title: 'CPU Usage',
    description: 'Showing CPU usage for the selected range',
    fields: [
      {
        key: 'cpuUsage',
        label: 'Usage',
        color: 'var(--chart-1)',
        formatter: (value) => `${value.toFixed(2)}%`,
      },
    ],
    summaryFields: [
      {
        key: 'cpuUsage',
        label: 'Usage',
        color: 'var(--chart-1)',
        formatter: (value) => `${value.toFixed(2)}%`,
      },
    ],
  },
  {
    kind: 'network',
    title: 'Network Usage',
    description: 'Showing network usage for the selected range',
    fields: [
      {
        key: 'rxBytes',
        label: 'Received',
        color: 'var(--chart-1)',
        formatter: (value) => byteTransform(value, 2),
      },
      {
        key: 'txBytes',
        label: 'Sent',
        color: 'var(--chart-2)',
        formatter: (value) => byteTransform(value, 2),
      },
    ],
    summaryFields: [
      {
        key: 'rxBytes',
        label: 'Received',
        color: 'var(--chart-1)',
        formatter: (value) => byteTransform(value, 2),
      },
      {
        key: 'txBytes',
        label: 'Sent',
        color: 'var(--chart-2)',
        formatter: (value) => byteTransform(value, 2),
      },
    ],
  },
];

export const StackStats = ({ stackId, containers }: StackStatsProps) => {
  const [liveStats, setLiveStats] = useState<Record<string, ContainerStatView[]>>({});
  const lastStatRef = useRef<Record<string, ContainerStatView | undefined>>({});

  useEffect(() => {
    const updates: { id: string; stat: ContainerStatView }[] = [];

    containers.forEach((container) => {
      const id = normalizeDockerId(container.id);
      const stat = container.containerStat;
      if (!id || !stat || container.state !== ContainerStateStatus.Running || stat === lastStatRef.current[id]) {
        return;
      }

      lastStatRef.current[id] = stat;
      updates.push({ id, stat: { ...stat, created: Math.floor(Date.now() / 1000) } });
    });

    if (updates.length > 0) {
      appendLiveStats(setLiveStats, updates);
    }
  }, [containers]);

  return (
    <div className="flex flex-col gap-4">
      {STACK_STAT_METRICS.map((metric) => (
        <StackMetricCard
          key={metric.kind}
          stackId={stackId}
          containers={containers}
          liveStats={liveStats}
          metric={metric}
        />
      ))}
    </div>
  );
};

const StackMetricCard = ({
  stackId,
  containers,
  liveStats,
  metric,
}: {
  stackId: string;
  containers: ContainerDataView[];
  liveStats: Record<string, ContainerStatView[]>;
  metric: MetricConfig;
}) => {
  const [windowHours, setWindowHours] = useState<StatsWindowHours>(24);
  const [selectedContainer, setSelectedContainer] = useState<ContainerSelection>(ALL_CONTAINERS);
  const readArgs = useMemo(() => ({ stackId, query: { hours: windowHours } }), [stackId, windowHours]);
  const { data, isLoading } = useRead('getStackStats', readArgs);

  const series = useMemo(
    () => buildContainerSeries(data?.data?.containers ?? [], containers, liveStats),
    [containers, data?.data?.containers, liveStats],
  );
  const effectiveContainer = useMemo(
    () =>
      selectedContainer === ALL_CONTAINERS || series.some((container) => container.id === selectedContainer)
        ? selectedContainer
        : ALL_CONTAINERS,
    [selectedContainer, series],
  );
  const chartData = useMemo(() => {
    const range = getTimeRange(windowHours);
    return buildMetricChartData(series, metric, effectiveContainer, range, windowHours);
  }, [effectiveContainer, metric, series, windowHours]);
  const chartConfig = useMemo(() => buildChartConfig(metric), [metric]);
  const latest = chartData.at(-1);

  return isLoading ? (
    <Skeleton className="h-72 w-full rounded-xl" />
  ) : (
    <Card className="bg-background rounded-sm shadow-xs py-0">
      <StatsPanelHeader
        title={metric.title}
        description={metric.description}
        controls={
          <StackMetricControls
            windowHours={windowHours}
            onWindowHoursChange={setWindowHours}
            selectedContainer={effectiveContainer}
            onSelectedContainerChange={setSelectedContainer}
            containers={series}
          />
        }>
        {metric.summaryFields.map((field) => (
          <StatsSummaryItem
            key={field.key}
            label={field.label}
            value={latest?.[field.key] !== undefined ? field.formatter(Number(latest[field.key])) : '-'}
          />
        ))}
      </StatsPanelHeader>
      <CardContent className="px-2 sm:px-6">
        {chartData.length > 0 ? (
          <ChartContainer config={chartConfig} className="aspect-auto h-62.5 w-full">
            <AreaChart data={chartData} accessibilityLayer>
              <defs>
                {metric.fields.map((field) => (
                  <linearGradient
                    key={field.key}
                    id={getGradientId(metric.kind, field.key)}
                    x1="0"
                    y1="0"
                    x2="0"
                    y2="1">
                    <stop offset="5%" stopColor={`var(--color-${field.key})`} stopOpacity={0.8} />
                    <stop offset="95%" stopColor={`var(--color-${field.key})`} stopOpacity={0.1} />
                  </linearGradient>
                ))}
              </defs>
              <CartesianGrid vertical={true} />
              <XAxis
                dataKey="created"
                tickLine={false}
                axisLine={false}
                tickMargin={8}
                minTickGap={32}
                tickFormatter={(timestamp) =>
                  dayjs(Number(timestamp) * 1000).format(windowHours > 24 ? 'MMM D HH:mm' : 'HH:mm')
                }
              />
              <ChartTooltip
                cursor={false}
                content={({ active, payload, label }) => {
                  if (!active || !payload?.length) return null;
                  return (
                    <div className="border-border/50 bg-background grid min-w-48 gap-1.5 rounded-lg border px-2.5 py-1.5 text-xs shadow-xl">
                      <div className="font-medium text-foreground">
                        {dayjs(Number(label) * 1000).format('MMM D, HH:mm:ss')}
                      </div>
                      <div className="grid gap-1.5">
                        {payload.map((item) => {
                          const field = metric.fields.find((candidate) => candidate.key === item.dataKey);
                          if (!field) return null;

                          return (
                            <div key={item.dataKey} className="flex items-center gap-2">
                              <span
                                className="h-2.5 w-2.5 shrink-0 rounded-xs"
                                style={{ backgroundColor: `var(--color-${field.key})` }}
                              />
                              <span className="text-muted-foreground">{field.label}</span>
                              <span className="ml-auto font-mono font-medium tabular-nums text-foreground">
                                {field.formatter(Number(item.value ?? 0))}
                              </span>
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  );
                }}
              />
              {metric.fields.map((field) => (
                <Area
                  key={field.key}
                  dataKey={field.key}
                  type="natural"
                  fill={`url(#${getGradientId(metric.kind, field.key)})`}
                  stroke={`var(--color-${field.key})`}
                  stackId={metric.stacked ? metric.kind : undefined}
                />
              ))}
              <ChartLegend content={<ChartLegendContent />} />
            </AreaChart>
          </ChartContainer>
        ) : (
          <div className="flex h-62.5 items-center justify-center text-sm text-muted-foreground">
            No data in this window
          </div>
        )}
      </CardContent>
    </Card>
  );
};

const StackMetricControls = ({
  windowHours,
  onWindowHoursChange,
  selectedContainer,
  onSelectedContainerChange,
  containers,
}: {
  windowHours: StatsWindowHours;
  onWindowHoursChange: (hours: StatsWindowHours) => void;
  selectedContainer: ContainerSelection;
  onSelectedContainerChange: (container: ContainerSelection) => void;
  containers: ContainerSeries[];
}) => {
  return (
    <div className="flex shrink-0 flex-wrap items-center gap-2 lg:justify-end">
      <StatsWindowSelect value={windowHours} onChange={onWindowHoursChange} />
      <Select value={selectedContainer} onValueChange={onSelectedContainerChange}>
        <SelectTrigger className="h-8 w-48 rounded-sm bg-background shadow-none">
          <SelectValue />
        </SelectTrigger>
        <SelectContent className="bg-background">
          <SelectItem value={ALL_CONTAINERS}>All containers</SelectItem>
          {containers.map((container) => (
            <SelectItem key={container.id} value={container.id}>
              <span className="inline-flex items-center gap-2">
                <span
                  className="h-2 w-2 shrink-0 rounded-full"
                  style={{ backgroundColor: getContainerSeriesColor(container.name).stroke }}
                />
                {container.name}
              </span>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
};

const buildContainerSeries = (
  historicalContainers: { containerId: string; containerName: string; stats: ContainerStatView[] }[],
  liveContainers: ContainerDataView[],
  liveStats: Record<string, ContainerStatView[]>,
) => {
  const byId = new Map<string, ContainerSeries>();

  historicalContainers.forEach((container) => {
    const id = normalizeDockerId(container.containerId);
    if (!id) return;

    byId.set(id, {
      id,
      name: getContainerLabel(container.containerName, container.containerId),
      stats: [...(container.stats ?? [])],
    });
  });

  liveContainers.forEach((container) => {
    const id = normalizeDockerId(container.id);
    if (!id) return;

    const existing = byId.get(id);
    byId.set(id, {
      id,
      name: getContainerLabel(container.name, container.id),
      stats: [...(existing?.stats ?? []), ...(liveStats[id] ?? [])],
    });
  });

  return Array.from(byId.values()).sort((a, b) => a.name.localeCompare(b.name));
};

const buildMetricChartData = (
  containers: ContainerSeries[],
  metric: MetricConfig,
  selectedContainer: ContainerSelection,
  range: { start: number; end: number },
  windowHours: StatsWindowHours,
) => {
  const selectedContainers =
    selectedContainer === ALL_CONTAINERS
      ? containers
      : containers.filter((container) => container.id === selectedContainer);
  const fields = uniqueFields([...metric.fields, ...metric.summaryFields]);
  const bucketSeconds = windowHours > 24 ? 300 : 60;

  if (selectedContainer === ALL_CONTAINERS) {
    return buildAggregatedMetricChartData(selectedContainers, fields, range, bucketSeconds);
  }

  const byTime = new Map<number, ChartDatum>();

  selectedContainers.forEach((container) => {
    container.stats.forEach((stat) => {
      const created = Number(stat.created ?? 0);
      if (!created || created < range.start || created > range.end) return;

      const point = byTime.get(created) ?? { created };

      fields.forEach((field) => {
        point[field.key] = Number(stat[field.key] ?? 0);
      });

      byTime.set(created, point);
    });
  });

  return Array.from(byTime.values()).sort((a, b) => a.created - b.created);
};

const buildAggregatedMetricChartData = (
  containers: ContainerSeries[],
  fields: MetricField[],
  range: { start: number; end: number },
  bucketSeconds: number,
) => {
  const latestContainerPointByBucket = new Map<string, ChartDatum & { sampleCreated: number }>();

  containers.forEach((container) => {
    container.stats.forEach((stat) => {
      const created = Number(stat.created ?? 0);
      if (!created || created < range.start || created > range.end) return;

      const pointTime = Math.floor(created / bucketSeconds) * bucketSeconds;
      const key = `${container.id}:${pointTime}`;
      const existing = latestContainerPointByBucket.get(key);

      if (existing && existing.sampleCreated > created) {
        return;
      }

      const point: ChartDatum & { sampleCreated: number } = { created: pointTime, sampleCreated: created };
      fields.forEach((field) => {
        point[field.key] = Number(stat[field.key] ?? 0);
      });

      latestContainerPointByBucket.set(key, point);
    });
  });

  const totalsByTime = new Map<number, ChartDatum>();

  latestContainerPointByBucket.forEach((containerPoint) => {
    const point = totalsByTime.get(containerPoint.created) ?? { created: containerPoint.created };

    fields.forEach((field) => {
      point[field.key] = aggregateMetricField(
        field,
        Number(point[field.key] ?? 0),
        Number(containerPoint[field.key] ?? 0),
      );
    });

    totalsByTime.set(containerPoint.created, point);
  });

  return Array.from(totalsByTime.values()).sort((a, b) => a.created - b.created);
};

const aggregateMetricField = (field: MetricField, current: number, value: number) =>
  field.aggregate === 'max' ? Math.max(current, value) : current + value;

const buildChartConfig = (metric: MetricConfig): ChartConfig =>
  Object.fromEntries(
    metric.fields.map((field) => [
      field.key,
      {
        label: <span className="text-foreground">{field.label}</span>,
        color: field.color,
      },
    ]),
  ) satisfies ChartConfig;

const appendLiveStats = (
  setLiveStats: Dispatch<SetStateAction<Record<string, ContainerStatView[]>>>,
  updates: { id: string; stat: ContainerStatView }[],
) => {
  setLiveStats((current) => {
    const next = { ...current };

    updates.forEach(({ id, stat }) => {
      next[id] = [...(next[id] ?? []), stat];
    });

    return next;
  });
};

const uniqueFields = (fields: MetricField[]) => {
  const seen = new Set<StatField>();
  return fields.filter((field) => {
    if (seen.has(field.key)) return false;
    seen.add(field.key);
    return true;
  });
};

const getTimeRange = (windowHours: StatsWindowHours) => {
  const end = Math.floor(Date.now() / 1000);
  return {
    start: end - windowHours * 60 * 60,
    end,
  };
};

const getContainerLabel = (name: string | undefined | null, id: string) => {
  const normalizedName = name?.replace(/^\//, '').trim();
  return normalizedName || id.slice(0, 12);
};

const getGradientId = (metricKind: MetricConfig['kind'], key: StatField) => `${metricKind}-${key}`;
