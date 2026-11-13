import { memoryBreakdown } from '@/lib/memory-stats';
import { useId, useMemo, type ReactNode } from 'react';
import dayjs from 'dayjs';
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from 'recharts';
import type { ContainerStatView } from '@/api/generated/api.types';
import { StatsPanelHeader, StatsSummaryItem } from '@/components/custom/common';
import { useStatsChartBounds } from '@/components/custom/stats-chart-range';
import { Card, CardContent } from '@/components/ui/card';
import { SegmentedControl } from '@/components/ui/segmented-control';
import { Skeleton } from '@/components/ui/skeleton';
import {
  CHART_ACTIVE_DOT,
  CHART_HOVER_CURSOR,
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from '@/components/ui/chart';
import { byteTransform } from '@/lib/bytes.helper';
import { useLocalStorage } from '@/lib/hooks';
import { statsChartData, statsValueAxis } from '@/lib/stats-chart';
import { statsAxis } from '@/lib/stats-range';
import type { ContainerStatsResource } from './types';

type MemoryStat = Pick<ContainerStatView, 'created'> &
  Partial<Record<'memoryActive' | 'memoryActiveCache' | 'memoryCache' | 'memoryLimit', number | null>>;

const breakdownConfig = {
  nonCache: { label: 'Non-cache memory', color: 'var(--chart-1)' },
  activeCache: { label: 'Active file cache', color: 'var(--chart-2)' },
  inactiveCache: { label: 'Inactive file cache', color: 'var(--chart-3)' },
} satisfies ChartConfig;
const workingSetConfig = { memoryActive: { label: 'Working set', color: 'var(--chart-1)' } } satisfies ChartConfig;

const MemoryUsage = ({
  stats,
  container,
  current,
  isLoading,
  controls,
}: {
  stats: MemoryStat[];
  container?: ContainerStatsResource;
  current?: MemoryStat;
  isLoading: boolean;
  controls?: ReactNode;
}) => {
  const bounds = useStatsChartBounds();
  const id = useId().replaceAll(':', '');
  const [storedMode, setMode] = useLocalStorage<'breakdown' | 'working-set'>('stats-memory-view', 'breakdown');
  const breakdown = storedMode !== 'working-set';
  const mode = breakdown ? 'breakdown' : 'working-set';
  const config = breakdown ? breakdownConfig : workingSetConfig;
  const data = useMemo(() => stats.map(memoryBreakdown), [stats]);
  const unavailable = breakdown && data.some((point) => point.nonCache == null);
  const latest = container ? (container.state === 'Running' ? container.containerStat : undefined) : current;
  const latestBreakdown = latest ? memoryBreakdown(latest) : undefined;
  const summary = breakdown
    ? ([
        ['Non-cache memory', latestBreakdown?.nonCache],
        ['Active file cache', latestBreakdown?.activeCache],
        ['Inactive file cache', latestBreakdown?.inactiveCache],
      ] as const)
    : ([['Working set', latest?.memoryActive]] as const);

  return isLoading ? (
    <Skeleton className="h-56.25 w-full rounded-xl" />
  ) : (
    <Card className="bg-background rounded-sm shadow-xs py-0">
      <StatsPanelHeader
        title="Memory Usage"
        description={
          breakdown
            ? 'Non-cache memory and active/inactive filesystem cache. Shared memory stays in non-cache memory.'
            : 'Docker working set: includes active file cache and excludes inactive file cache.'
        }
        controls={
          <div className="flex flex-wrap items-center gap-2">
            <SegmentedControl
              aria-label="Memory Usage view"
              value={mode}
              onValueChange={(value) => setMode(value as typeof mode)}
              options={[
                { value: 'breakdown', label: 'Breakdown' },
                { value: 'working-set', label: 'Working set' },
              ]}
            />
            {controls}
          </div>
        }>
        {summary.map(([label, value]) => (
          <StatsSummaryItem key={label} label={label} value={value == null ? '-' : byteTransform(value, 2)} />
        ))}
        <StatsSummaryItem
          label="Limit"
          value={latest?.memoryLimit == null ? '-' : byteTransform(latest.memoryLimit, 2)}
        />
      </StatsPanelHeader>
      <CardContent className="px-2 sm:px-6">
        {unavailable && (
          <p className="text-sm text-muted-foreground" role="status">
            Memory breakdown is unavailable for some samples. Select Working set to view those measurements.
          </p>
        )}
        <ChartContainer config={config} className="aspect-auto h-62.5 w-full">
          <AreaChart data={statsChartData(data, bounds)} accessibilityLayer>
            <defs>
              {Object.keys(config).map((key) => (
                <linearGradient key={key} id={`${id}-${key}`} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor={`var(--color-${key})`} stopOpacity={0.25} />
                  <stop offset="95%" stopColor={`var(--color-${key})`} stopOpacity={0.08} />
                </linearGradient>
              ))}
            </defs>
            <CartesianGrid vertical={false} strokeDasharray="3 3" />
            <YAxis {...statsValueAxis('bytes')} />
            <XAxis
              {...statsAxis(bounds)}
              dataKey="created"
              tickLine={false}
              axisLine={false}
              tickMargin={8}
              minTickGap={32}
              tickFormatter={(timestamp) =>
                dayjs(timestamp * 1000).format(bounds && bounds.to - bounds.from > 86400 ? 'MMM D HH:mm' : 'HH:mm')
              }
            />
            <ChartTooltip
              cursor={CHART_HOVER_CURSOR}
              content={
                <ChartTooltipContent
                  indicator="dot"
                  labelFormatter={(_, points) => {
                    const point = points.at(0)?.payload as MemoryStat | undefined;
                    return (
                      <div className="space-y-1">
                        <div>{dayjs(Number(point?.created) * 1000).format('MMM D, h:mm:ss A')}</div>
                        {breakdown && point?.memoryActive != null && point.memoryCache != null && (
                          <>
                            <div>Working set: {byteTransform(point.memoryActive, 2)}</div>
                            <div>Total charged: {byteTransform(point.memoryActive + point.memoryCache, 2)}</div>
                          </>
                        )}
                      </div>
                    );
                  }}
                  valueFormatter={(value) => byteTransform(Number(value), 2)}
                />
              }
            />
            {Object.keys(config).map((key) => (
              <Area
                key={key}
                dataKey={key}
                type="linear"
                isAnimationActive={false}
                connectNulls={false}
                activeDot={CHART_ACTIVE_DOT}
                fill={`url(#${id}-${key})`}
                stroke={`var(--color-${key})`}
                stackId={breakdown ? 'memory' : undefined}
              />
            ))}
            <ChartLegend content={<ChartLegendContent />} />
          </AreaChart>
        </ChartContainer>
      </CardContent>
    </Card>
  );
};
export default MemoryUsage;
