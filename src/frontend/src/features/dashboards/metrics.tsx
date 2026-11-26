import MemoryUsage from '@/features/docker-resources/containers/container-info/stats/memory-usage';
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  ChartLegend,
  ChartLegendContent,
  CHART_ACTIVE_DOT,
  CHART_HOVER_CURSOR,
} from '@/components/ui/chart';
import Loader from '@/components/ui/loader';
import { monitoringLink } from '@/lib/monitoring-link';
import { useId, useContext, useEffect, useRef, useState, useMemo, type ReactNode } from 'react';
import { Link } from 'react-router';
import { Area, CartesianGrid, ComposedChart, Line, XAxis, YAxis } from 'recharts';
import type { ContainerStatView, DashboardMetricSource, DashboardSourceView } from '@/api/generated/api.types';
import { statsChartData, statsValueAxis } from '@/lib/stats-chart';
import { filterStatsRange, statsBucketSeconds, resolveStatsRange, statsRangeQuery } from '@/lib/stats-range';
import { requestErrorStatus } from '@/lib/request-error';
import { useRead } from '@/lib/hooks';
import { useTaskStatsStream } from '@/features/swarm-resources/tasks/hooks/useTaskStats';
import { useServiceStatsStream } from '@/features/swarm-resources/services/hooks/useServiceStats';
import { usePlatformLiveStats } from '@/features/platforms/platform-series';
import { useRealtimeContext } from '@/lib/context/realtime-context';
import { Choice, chartPalette } from './display-controls';
import { RangeClock } from './range-clock';
import { useStackSeries } from './stack-series';
import { useAppContext } from '@/lib/context/app-context';
import { latestLiveStat, mergeStatsBuckets } from '@/lib/stats-buckets';
import { sourcePath } from './source-path';
import { Timestamp } from '@/components/custom/timestamp';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { Tile, WidgetNotice } from './tile';

import type { MetricTile } from './requirements';
type Sample = { created: number } & Partial<
  Record<
    | 'cpuUsage'
    | 'memoryCache'
    | 'memoryActiveCache'
    | 'memoryLimit'
    | 'memoryActive'
    | 'memoryUsage'
    | 'rxBytesPerSecond'
    | 'txBytesPerSecond'
    | 'blockReadBytesPerSecond'
    | 'blockWriteBytesPerSecond',
    number | null
  >
>;
const emptyIds: string[] = [];
const emptyStats: ContainerStatView[] = [];
type Series = { name: string; stats: Sample[] };
function Chart({
  tile,
  series,
  platform,
  minutes,
  now,
  formatTime,
}: {
  formatTime: DateTimeFormatter;
  now: number;
  tile: MetricTile;
  series: Series;
  platform: boolean;
  minutes: number;
}) {
  const gradientId = useId().replaceAll(':', '');
  const frame = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    if (!frame.current) return;
    const observer = new IntersectionObserver(([entry]) => setVisible(entry.isIntersecting));
    observer.observe(frame.current);
    return () => observer.disconnect();
  }, []);
  const { metric, legend, color, presentation } = tile.settings;
  const bounds = resolveStatsRange({ kind: 'relative', minutes }, now);
  const data = series.stats;
  const keys =
    metric === 'Cpu'
      ? ['cpuUsage']
      : metric === 'Memory'
        ? [platform ? 'memoryUsage' : 'memoryActive']
        : metric === 'Network'
          ? ['rxBytesPerSecond', 'txBytesPerSecond']
          : ['blockReadBytesPerSecond', 'blockWriteBytesPerSecond'];
  const labels: Record<string, string> = {
    cpuUsage: 'CPU',
    memoryUsage: 'Memory',
    memoryActive: 'Working set',
    rxBytesPerSecond: 'Receive',
    txBytesPerSecond: 'Send',
    blockReadBytesPerSecond: 'Read',
    blockWriteBytesPerSecond: 'Write',
  };
  const unit =
    metric === 'Cpu' || (platform && metric === 'Memory')
      ? 'percent'
      : metric === 'Memory'
        ? 'bytes'
        : 'bytesPerSecond';
  const measured = data.some((s) => keys.some((k) => s[k as keyof Sample] != null));
  const latest = data.at(-1);
  const format = statsValueAxis(unit).tickFormatter;
  const config = Object.fromEntries(
    keys.map((key, i) => [key, { label: labels[key], color: i ? 'var(--chart-2)' : chartPalette[color] }]),
  );
  if (!platform && metric === 'Memory' && measured)
    return (
      <div ref={frame} className="flex min-h-40 flex-1 flex-col">
        {visible && (
          <MemoryUsage
            stats={data}
            controls={
              latest && (
                <span className="text-muted-foreground">
                  <Timestamp value={latest.created * 1000} formatDateTime={formatTime} />
                </span>
              )
            }
            current={latest}
            isLoading={false}
            embedded={{ bounds, color: chartPalette[color], presentation, legend }}
          />
        )}
      </div>
    );
  return (
    <div ref={frame} className="flex min-h-40 min-w-0 flex-1 flex-col md:min-h-0">
      {measured && latest && (
        <div className="mb-1 flex flex-wrap items-baseline justify-between gap-x-2">
          <span className="font-mono text-base font-semibold tabular-nums">
            {keys
              .map((k) => {
                const v = latest[k as keyof Sample];
                return labels[k] + ' ' + (v == null ? '—' : format(v));
              })
              .join(' · ')}
          </span>
          <span className="text-xs text-muted-foreground">
            <Timestamp value={latest.created * 1000} formatDateTime={formatTime} />
          </span>
        </div>
      )}
      {!measured ? (
        <p className="text-muted-foreground">No measurements in this range.</p>
      ) : (
        visible && (
          <div className="relative min-h-20 flex-1">
            <div className="absolute inset-0">
              <ChartContainer config={config} className="aspect-auto h-full w-full">
                <ComposedChart data={data} accessibilityLayer>
                  {presentation === 'Gradient' && (
                    <defs>
                      {keys.map((k) => (
                        <linearGradient key={k} id={gradientId + k} x1="0" x2="0" y1="0" y2="1">
                          <stop offset="0%" stopColor={`var(--color-${k})`} stopOpacity={0.32} />
                          <stop offset="100%" stopColor={`var(--color-${k})`} stopOpacity={0.03} />
                        </linearGradient>
                      ))}
                    </defs>
                  )}
                  <CartesianGrid vertical={false} stroke="var(--border)" />
                  <XAxis
                    dataKey="created"
                    type="number"
                    domain={[bounds.from, bounds.to]}
                    tick={{ fontSize: 10 }}
                    tickFormatter={(v) =>
                      new Date(v * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
                    }
                  />
                  <YAxis {...statsValueAxis(unit)} />
                  <ChartTooltip
                    cursor={CHART_HOVER_CURSOR}
                    content={
                      <ChartTooltipContent
                        indicator="dot"
                        labelFormatter={(_, points) => formatTime(Number(points[0]?.payload?.created) * 1000)}
                        valueFormatter={(v) => format(Number(v))}
                      />
                    }
                  />
                  {legend && <ChartLegend content={<ChartLegendContent />} />}
                  {keys.map((k, i) =>
                    presentation !== 'Line' ? (
                      <Area
                        key={k}
                        dataKey={k}
                        name={labels[k]}
                        stroke={i ? 'var(--chart-2)' : chartPalette[color]}
                        strokeDasharray={i ? '4 3' : undefined}
                        activeDot={CHART_ACTIVE_DOT}
                        fill={presentation === 'Gradient' ? `url(#${gradientId + k})` : `var(--color-${k})`}
                        fillOpacity={presentation === 'Gradient' ? 1 : 0.18}
                        isAnimationActive={false}
                      />
                    ) : (
                      <Line
                        key={k}
                        dataKey={k}
                        name={labels[k]}
                        stroke={i ? 'var(--chart-2)' : chartPalette[color]}
                        strokeDasharray={i ? '4 3' : undefined}
                        dot={false}
                        isAnimationActive={false}
                      />
                    ),
                  )}
                </ComposedChart>
              </ChartContainer>
            </div>
          </div>
        )
      )}
    </div>
  );
}

function SeriesChart({
  tile,
  series,
  platform,
  minutes,
  now,
  formatTime,
}: {
  formatTime: DateTimeFormatter;
  tile: MetricTile;
  series: Series[];
  platform: boolean;
  minutes: number;
  now: number;
}) {
  const [name, setName] = useState('');
  const selected = series.find((s) => s.name === name) ?? series[0];
  return (
    <>
      {series.length > 1 && (
        <Choice label="Service" value={selected.name} options={series.map((s) => s.name)} change={setName} />
      )}
      {series.length === 1 && selected.name && <span className="text-xs">{selected.name}</span>}
      <Chart formatTime={formatTime} tile={tile} series={selected} platform={platform} minutes={minutes} now={now} />
    </>
  );
}

function useChartSeries(
  baseStats: readonly Sample[] | undefined,
  live: readonly Sample[],
  seconds: number,
  stackSeries: Series[] | undefined,
  minutes: number,
  now: number,
) {
  const series = useMemo(
    () =>
      stackSeries
        ? stackSeries
        : baseStats
          ? [{ name: '', stats: mergeStatsBuckets<Sample>(baseStats, live, seconds) }]
          : undefined,
    [baseStats, live, seconds, stackSeries],
  );
  const visibleSeries = useMemo(() => {
    const bounds = resolveStatsRange({ kind: 'relative', minutes }, now);
    return series?.map((s) => ({ name: s.name, stats: statsChartData(filterStatsRange(s.stats, bounds), bounds) }));
  }, [series, now, minutes]);
  return { series, visibleSeries };
}

export function MetricHistory({
  tiles,
  source,
  row,
  minutes,
  interval,
  manual,
}: {
  tiles: MetricTile[];
  source: DashboardMetricSource;
  row: DashboardSourceView;
  minutes: number;
  interval: number | false;
  manual: boolean;
}) {
  const app = useAppContext();
  const canReadPlatform = app.platforms?.some((p) => p.id === row.platformId) === true;
  const query = statsRangeQuery({ kind: 'relative', minutes }, resolveStatsRange({ kind: 'relative', minutes }));
  const seconds = statsBucketSeconds(resolveStatsRange({ kind: 'relative', minutes }));
  const platformSource =
    !manual && source.kind === 'Platform' ? app.platforms?.find((p) => p.id === source.id) : undefined;
  const platformLive = usePlatformLiveStats(platformSource, seconds, minutes * 60);
  const allowedPlatform = !manual && canReadPlatform ? (row.platformId ?? '') : '';
  const taskLive = useTaskStatsStream(
    source.kind === 'Deployment' ? allowedPlatform : '',
    row.containerProjectionId ?? undefined,
    seconds,
    minutes * 60,
  );
  const now = useContext(RangeClock);
  const { groups } = useRealtimeContext();
  const streamGroup = source.kind === 'Platform' ? 'platforms' : allowedPlatform && 'containers:' + allowedPlatform;
  const latest = source.kind === 'Platform' ? platformSource?.stats?.[0]?.created : latestLiveStat(taskLive)?.created;
  // Stack and Swarm history also supplies membership for aggregating replicas.
  const receivingSamples = !!latest && now - latest <= 65 && !!streamGroup && groups?.isGroupLive(streamGroup);
  const opts = {
    staleTime: manual ? 0 : 15000,
    refetchInterval: receivingSamples ? (false as const) : interval,
    retry: (attempt: number, error: Error) => attempt < 2 && requestErrorStatus(error) === 429,
    retryDelay: (attempt: number) => 500 * 2 ** attempt + Math.random() * 350,
  };
  const platform = useRead(
    'getPlatformStats',
    { id: source.id, query },
    { ...opts, enabled: source.kind === 'Platform' },
  );
  const deployment = useRead(
    'getDeploymentStats',
    { id: source.id, query },
    { ...opts, enabled: source.kind === 'Deployment' },
  );
  const stack = useRead('getStackStats', { stackId: source.id, query }, { ...opts, enabled: source.kind === 'Stack' });
  const service = useRead(
    'getSwarmServiceStats',
    { platformId: row.platformId ?? undefined, resourceId: row.dockerServiceId ?? undefined, query },
    { ...opts, enabled: source.kind === 'SwarmService' && canReadPlatform && !!row.dockerServiceId },
  );
  const read = {
    Platform: platform,
    Stack: stack,
    Deployment: deployment,
    SwarmService: service,
  }[source.kind];
  const data = read.data?.data;
  const serviceLive = useServiceStatsStream(
    source.kind === 'SwarmService' && service.data?.data.complete ? allowedPlatform : '',
    service.data?.data.observedContainerProjectionIds ?? emptyIds,
    seconds,
    minutes * 60,
  );
  const live =
    source.kind === 'Platform'
      ? platformLive
      : source.kind === 'Deployment'
        ? taskLive
        : source.kind === 'SwarmService' && serviceLive.observedContainers === service.data?.data.expectedTasks
          ? serviceLive.stats
          : emptyStats;
  const stackSeries = useStackSeries(
    !manual && source.kind === 'Stack' && stack.data && stack.data.data.services.length <= 8 ? source.id : undefined,
    stack.data?.data.services,
    seconds,
    minutes * 60,
  );
  const baseStats = data && 'stats' in data ? data.stats : undefined;
  const hasServices = !!data && 'services' in data;
  const { series, visibleSeries } = useChartSeries(
    baseStats,
    live,
    seconds,
    hasServices ? stackSeries : undefined,
    minutes,
    now,
  );
  const formatDateTime = useProfileDateTimeFormatter();
  const latestMeasurement = Math.max(Number(latest ?? 0), ...(series ?? []).map((s) => s.stats.at(-1)?.created ?? 0));
  const delayed = latestMeasurement > 0 && now - latestMeasurement > Math.max(130, 2 * seconds);
  const link = sourcePath(source.kind, row);
  return (
    <>
      {tiles.map((tile) => {
        let body: ReactNode;
        if (read.isError || !series)
          body = read.isLoading ? (
            <Loader />
          ) : (
            <WidgetNotice>
              {requestErrorStatus(read.error) === 429
                ? 'Statistics reads are busy. Try Refresh shortly.'
                : 'Unavailable'}
            </WidgetNotice>
          );
        else if (!series.length) body = <p className="text-muted-foreground">No measurements in this range.</p>;
        else if (series.length > 8)
          body = <p>This stack has more than eight service series. Open full statistics to view them.</p>;
        else
          body = visibleSeries && (
            <SeriesChart
              formatTime={formatDateTime}
              series={visibleSeries}
              tile={tile}
              platform={source.kind === 'Platform'}
              minutes={minutes}
              now={now}
            />
          );
        return (
          <Tile tile={tile} key={tile.id}>
            <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
              <Link
                className="table-link max-w-full truncate rounded border bg-muted/35 px-2 py-0.5 font-mono text-xs font-medium text-foreground"
                to={link}>
                {row.name}
              </Link>
              <Link className="table-link ml-auto" to={monitoringLink(link, source.kind, 'stats', minutes)}>
                Full statistics
              </Link>
            </div>
            {delayed && !read.isError && (
              <p className="text-xs text-muted-foreground">
                Latest measurement <Timestamp value={latestMeasurement * 1000} formatDateTime={formatDateTime} />
              </p>
            )}
            {service.data?.data.complete === false && (
              <p className="text-xs">
                Partial coverage · {service.data.data.observedTasks} of {service.data.data.expectedTasks} tasks
              </p>
            )}
            {body}
          </Tile>
        );
      })}
    </>
  );
}
