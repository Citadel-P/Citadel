import { appendLiveStatBucket, averageStatBuckets } from '@/lib/stats-buckets';
import { statsChartData, statsValueAxis } from '@/lib/stats-chart';
import { StatsChartRange, useStatsChartBounds } from '@/components/custom/stats-chart-range';
import { useStatsRange } from '@/hooks/useStatsRange';
import { statsAxis, statsBucketSeconds } from '@/lib/stats-range';
import { ContainerRuntimeView, ContainerStateStatus, ContainerStatView } from '@/api/generated/api.types';
import { NETWORK_CHART_COLORS, type ThemedChartColor } from '@/components/custom/chart-series-colors';
import { StatsPanelHeader, StatsSummaryItem } from '@/components/custom/common';
import { getContainerSeriesColor } from '@/components/custom/container-series-colors';
import { Card, CardContent } from '@/components/ui/card';
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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import { byteTransform } from '@/lib/bytes.helper';
import { useRead } from '@/lib/hooks';
import { mergeStatsByCreated, STREAMED_STATS_QUERY_OPTIONS } from '@/lib/live-stats';
import { normalizeDockerId } from '@/lib/utils';
import dayjs from 'dayjs';
import { Dispatch, SetStateAction, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from 'recharts';

type ContainerSelection = 'all' | string;
type MetricKind = 'memory' | 'cpu' | 'network';
type StatField = keyof Pick<
  ContainerStatView,
  'cpuUsage' | 'memoryActive' | 'memoryCache' | 'memoryLimit' | 'rxBytes' | 'txBytes'
>;

type StackStatsProps = {
  stackId: string;
  containers: ContainerRuntimeView[];
};

type ContainerSeries = {
  id: string;
  name: string;
  stats: ContainerStatView[];
};

type MetricField = {
  key: StatField;
  label: string;
  color?: string;
  theme?: ThemedChartColor;
  aggregate?: 'sum' | 'max';
  formatter: (value: number) => string;
};

type MetricConfig = {
  kind: MetricKind;
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
type MetricViewState = {
  selectedContainer: ContainerSelection;
};

const STACK_STAT_METRICS: MetricConfig[] = [
  {
    kind: 'memory',
    title: 'Memory Usage',
    description: 'Memory usage excludes file cache. Cache is shown separately.',
    fields: [
      {
        key: 'memoryActive',
        label: 'Usage',
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
        label: 'Usage',
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
        theme: NETWORK_CHART_COLORS.rxBytes,
        formatter: (value) => byteTransform(value, 2),
      },
      {
        key: 'txBytes',
        label: 'Sent',
        theme: NETWORK_CHART_COLORS.txBytes,
        formatter: (value) => byteTransform(value, 2),
      },
    ],
    summaryFields: [
      {
        key: 'rxBytes',
        label: 'Received',
        theme: NETWORK_CHART_COLORS.rxBytes,
        formatter: (value) => byteTransform(value, 2),
      },
      {
        key: 'txBytes',
        label: 'Sent',
        theme: NETWORK_CHART_COLORS.txBytes,
        formatter: (value) => byteTransform(value, 2),
      },
    ],
  },
];

export const StackStats = (props: StackStatsProps) => <StackStatsContent key={props.stackId} {...props} />;

const StackStatsContent = ({ stackId, containers }: StackStatsProps) => {
  const [liveStats, setLiveStats] = useState<Record<string, ContainerStatView[]>>({});
  const [metricViews, setMetricViews] = useState<Record<MetricKind, MetricViewState>>({
    memory: { selectedContainer: ALL_CONTAINERS },
    cpu: { selectedContainer: ALL_CONTAINERS },
    network: { selectedContainer: ALL_CONTAINERS },
  });
  const lastStatRef = useRef<Record<string, ContainerStatView | undefined>>({});
  const range = useStatsRange();
  const longestWindow = range.hours;
  const readArgs = useMemo(() => ({ stackId, query: range.query }), [range.query, stackId]);
  const { data, isLoading, isFetching, error, refetch } = useRead(
    'getStackStats',
    readArgs,
    STREAMED_STATS_QUERY_OPTIONS,
  );
  const bucketSeconds = statsBucketSeconds(range.bounds);
  const series = useMemo(
    () => buildServiceSeries(data?.data?.services ?? [], containers, range.live ? liveStats : {}, bucketSeconds),
    [containers, data?.data?.services, liveStats, range.live, bucketSeconds],
  );

  const currentSeries = useMemo(
    () =>
      range.live
        ? buildServiceSeries(
            (data?.data?.services ?? []).map((service) => ({ ...service, stats: [] })),
            containers,
            Object.fromEntries(
              containers.flatMap((container) =>
                container.containerStat ? [[normalizeDockerId(container.id), [container.containerStat]]] : [],
              ),
            ),
            bucketSeconds,
          )
        : undefined,
    [containers, data?.data?.services, range.live, bucketSeconds],
  );

  // Replacement/scale events change membership, not the workload's history.
  const membership = containers
    .map((container) => container.id)
    .sort()
    .join(',');
  const previousMembership = useRef(membership);
  useEffect(() => {
    // An in-flight response may predate the replacement; refresh after it settles.
    if (previousMembership.current === membership || isFetching) return;
    previousMembership.current = membership;
    void refetch({ cancelRefetch: false });
  }, [membership, isFetching, refetch]);

  useEffect(() => {
    const updates: { id: string; stat: ContainerStatView }[] = [];
    const activeIds = new Set<string>();

    containers.forEach((container) => {
      const id = normalizeDockerId(container.id);
      if (id) activeIds.add(id);
      const stat = container.containerStat;
      if (!id || !stat || container.state !== ContainerStateStatus.Running || stat === lastStatRef.current[id]) {
        return;
      }

      lastStatRef.current[id] = stat;
      updates.push({
        id,
        stat: {
          ...stat,
          created: Number(stat.created) > 0 ? stat.created : Math.floor(Date.now() / 1000),
        },
      });
    });

    appendLiveStats(setLiveStats, activeIds, updates, bucketSeconds, longestWindow * 60 * 60);
    lastStatRef.current = Object.fromEntries(Object.entries(lastStatRef.current).filter(([id]) => activeIds.has(id)));
  }, [containers, longestWindow, bucketSeconds]);

  const updateMetricView = useCallback((kind: MetricKind, update: Partial<MetricViewState>) => {
    setMetricViews((current) => ({
      ...current,
      [kind]: { ...current[kind], ...update },
    }));
  }, []);

  return (
    <div className="flex flex-col gap-4">
      <StatsChartRange range={range} error={error}>
        {STACK_STAT_METRICS.map((metric) => {
          const view = metricViews[metric.kind];
          return (
            <StackMetricCard
              key={metric.kind}
              series={series}
              currentSeries={currentSeries}
              isLoading={isLoading}
              metric={metric}
              selectedContainer={view.selectedContainer}
              onSelectedContainerChange={(selectedContainer) => updateMetricView(metric.kind, { selectedContainer })}
            />
          );
        })}
      </StatsChartRange>
    </div>
  );
};

const StackMetricCard = ({
  series,
  currentSeries,
  isLoading,
  metric,
  selectedContainer,
  onSelectedContainerChange,
}: {
  series: ContainerSeries[];
  currentSeries?: ContainerSeries[];
  isLoading: boolean;
  metric: MetricConfig;
  selectedContainer: ContainerSelection;
  onSelectedContainerChange: (container: ContainerSelection) => void;
}) => {
  const effectiveContainer = useMemo(
    () =>
      selectedContainer === ALL_CONTAINERS || series.some((container) => container.id === selectedContainer)
        ? selectedContainer
        : ALL_CONTAINERS,
    [selectedContainer, series],
  );
  const bounds = useStatsChartBounds()!;
  const chartData = useMemo(
    () => buildMetricChartData(series, metric, effectiveContainer, { start: bounds.from, end: bounds.to }),
    [effectiveContainer, metric, series, bounds.from, bounds.to],
  );
  const chartConfig = useMemo(() => buildChartConfig(metric), [metric]);
  const latest = currentSeries
    ? buildMetricChartData(currentSeries, metric, effectiveContainer, { start: bounds.from, end: bounds.to }).at(-1)
    : chartData.at(-1);

  return isLoading ? (
    <Skeleton className="h-72 w-full rounded-xl" />
  ) : (
    <Card className="bg-background rounded-sm shadow-xs py-0">
      <StatsPanelHeader
        title={metric.title}
        description={metric.description}
        controls={
          <StackMetricControls
            selectedContainer={effectiveContainer}
            onSelectedContainerChange={onSelectedContainerChange}
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
            <AreaChart data={statsChartData(chartData, bounds)} accessibilityLayer>
              <defs>
                {metric.fields.map((field) => (
                  <linearGradient
                    key={field.key}
                    id={getGradientId(metric.kind, field.key)}
                    x1="0"
                    y1="0"
                    x2="0"
                    y2="1">
                    <stop offset="5%" stopColor={`var(--color-${field.key})`} stopOpacity={0.18} />
                    <stop offset="95%" stopColor={`var(--color-${field.key})`} stopOpacity={0.02} />
                  </linearGradient>
                ))}
              </defs>
              <CartesianGrid vertical={false} strokeDasharray="3 3" />
              <YAxis {...statsValueAxis(metric.kind === 'cpu' ? 'percent' : 'bytes')} />
              <XAxis
                {...statsAxis(bounds)}
                dataKey="created"
                tickLine={false}
                axisLine={false}
                tickMargin={8}
                minTickGap={32}
                tickFormatter={(timestamp) =>
                  dayjs(Number(timestamp) * 1000).format(bounds.to - bounds.from > 86400 ? 'MMM D HH:mm' : 'HH:mm')
                }
              />
              <ChartTooltip
                cursor={CHART_HOVER_CURSOR}
                content={
                  <ChartTooltipContent
                    labelFormatter={(_, payload) =>
                      dayjs(Number(payload[0]?.payload?.created) * 1000).format('MMM D, h:mm:ss A')
                    }
                    valueFormatter={(value, name) => {
                      const field = metric.fields.find((candidate) => candidate.key === name);
                      return field?.formatter(Number(value)) ?? String(value);
                    }}
                  />
                }
              />
              {metric.fields.map((field) => (
                <Area
                  key={field.key}
                  dataKey={field.key}
                  type="linear"
                  isAnimationActive={false}
                  connectNulls={false}
                  activeDot={CHART_ACTIVE_DOT}
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
  selectedContainer,
  onSelectedContainerChange,
  containers,
}: {
  selectedContainer: ContainerSelection;
  onSelectedContainerChange: (container: ContainerSelection) => void;
  containers: ContainerSeries[];
}) => {
  return (
    <div className="flex shrink-0 flex-wrap items-center gap-2 lg:justify-end">
      <Select value={selectedContainer} onValueChange={onSelectedContainerChange}>
        <SelectTrigger className="h-8 w-48 rounded-sm bg-background shadow-none">
          <SelectValue />
        </SelectTrigger>
        <SelectContent className="bg-background">
          <SelectItem value={ALL_CONTAINERS}>All services</SelectItem>
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

export const buildServiceSeries = (
  historicalServices: { serviceName: string; containerIds: string[]; stats: ContainerStatView[] }[],
  liveContainers: ContainerRuntimeView[],
  liveStats: Record<string, ContainerStatView[]>,
  bucketSeconds: number,
): ContainerSeries[] => {
  const fields = uniqueFields(STACK_STAT_METRICS.flatMap((metric) => metric.summaryFields)).map((field) => ({
    ...field,
    aggregate: 'sum' as const,
  }));
  const byId = new Map(
    historicalServices.map((service) => [
      service.serviceName,
      {
        id: `service:${service.serviceName}`,
        name: service.serviceName,
        stats: service.stats,
      },
    ]),
  );
  const memberships = new Map(
    historicalServices.flatMap((service) =>
      service.containerIds.map((id) => [normalizeDockerId(id), service.serviceName] as const),
    ),
  );
  const members = new Map<string, ContainerSeries[]>();
  for (const container of liveContainers) {
    if (container.state !== ContainerStateStatus.Running) continue;
    const id = normalizeDockerId(container.id);
    if (!id) continue;
    const service = memberships.get(id);
    // A membership refresh will supply the authoritative service identity.
    if (!service) continue;
    const group = members.get(service) ?? [];
    group.push({ id, name: container.name, stats: liveStats[id] ?? [] });
    members.set(service, group);
  }
  for (const [id, group] of members) {
    const history = byId.get(id)!;
    const latestHistoryBucket = history.stats.reduce(
      (latest, stat) => Math.max(latest, Math.floor(Number(stat.created) / bucketSeconds) * bucketSeconds),
      0,
    );
    const reportingReplicas = new Map<number, number>();
    for (const member of group) {
      const buckets = new Set(
        member.stats.map((stat) => Math.floor(Number(stat.created) / bucketSeconds) * bucketSeconds),
      );
      for (const bucket of buckets) reportingReplicas.set(bucket, (reportingReplicas.get(bucket) ?? 0) + 1);
    }
    const live = buildAggregatedMetricChartData(
      group,
      fields,
      { start: 0, end: Number.MAX_SAFE_INTEGER },
      bucketSeconds,
    )
      .filter(
        (stat) =>
          // Keep saved historical totals authoritative. A partial live observation
          // must not replace the complete total while another replica is reporting.
          stat.created > latestHistoryBucket && reportingReplicas.get(stat.created) === group.length,
      )
      .map((stat) => ({
        ...stat,
        containerId: '',
        cpuUsage: stat.cpuUsage ?? 0,
        memoryActive: stat.memoryActive ?? 0,
        memoryCache: stat.memoryCache ?? 0,
        memoryLimit: stat.memoryLimit ?? 0,
        rxBytes: stat.rxBytes ?? 0,
        txBytes: stat.txBytes ?? 0,
      }));
    byId.set(id, {
      ...history,
      stats: mergeStatsByCreated(
        history.stats.map((stat) => ({
          ...stat,
          created: Math.floor(Number(stat.created) / bucketSeconds) * bucketSeconds,
        })),
        live,
      ),
    });
  }
  return Array.from(byId.values()).sort((a, b) => a.name.localeCompare(b.name));
};

const buildMetricChartData = (
  containers: ContainerSeries[],
  metric: MetricConfig,
  selectedContainer: ContainerSelection,
  range: { start: number; end: number },
) => {
  const selectedContainers =
    selectedContainer === ALL_CONTAINERS
      ? containers
      : containers.filter((container) => container.id === selectedContainer);
  const fields = uniqueFields([...metric.fields, ...metric.summaryFields]);
  const bucketSeconds = statsBucketSeconds({ from: range.start, to: range.end });

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
    averageStatBuckets(container.stats, bucketSeconds).forEach((stat) => {
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
        ...(field.theme ? { theme: field.theme } : { color: field.color }),
      },
    ]),
  ) satisfies ChartConfig;

const appendLiveStats = (
  setLiveStats: Dispatch<SetStateAction<Record<string, ContainerStatView[]>>>,
  activeIds: Set<string>,
  updates: { id: string; stat: ContainerStatView }[],
  bucketSeconds: number,
  maxAgeSeconds: number,
) => {
  setLiveStats((current) => {
    const next = Object.fromEntries(Object.entries(current).filter(([id]) => activeIds.has(id)));

    updates.forEach(({ id, stat }) => {
      next[id] = appendLiveStatBucket(next[id] ?? [], stat, bucketSeconds, maxAgeSeconds);
    });

    const removedInactiveContainer = Object.keys(next).length !== Object.keys(current).length;
    return updates.length > 0 || removedInactiveContainer ? next : current;
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

const getGradientId = (metricKind: MetricConfig['kind'], key: StatField) => `${metricKind}-${key}`;
