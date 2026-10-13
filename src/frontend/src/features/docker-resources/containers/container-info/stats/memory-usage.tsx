import { statsChartData, statsValueAxis } from '@/lib/stats-chart';
import { useStatsChartBounds } from '@/components/custom/stats-chart-range';
import { statsAxis } from '@/lib/stats-range';
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from 'recharts';
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
import { StatsPanelHeader, StatsSummaryItem } from '@/components/custom/common';
import { Card, CardContent } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { ReactNode, useMemo } from 'react';
import dayjs from 'dayjs';
import { byteTransform } from '@/lib/bytes.helper';
import { ContainerStatView } from '@/api/generated/api.types';
import type { ContainerStatsResource } from './types';

const MemoryUsage = ({
  stats,
  container,
  isLoading,
  controls,
}: {
  stats: Omit<ContainerStatView, 'containerId'>[];
  container: ContainerStatsResource | undefined;
  isLoading: boolean;
  controls?: ReactNode;
}) => {
  const bounds = useStatsChartBounds();
  const chartConfig = useMemo(
    () =>
      ({
        memoryActive: {
          label: 'Usage',
          color: 'var(--chart-1)',
        },
        memoryCache: {
          label: 'Cache',
          color: 'var(--chart-2)',
        },
      }) satisfies ChartConfig,
    [],
  );

  const gradientDefs = useMemo(
    () => (
      <defs>
        <linearGradient id="fillmemoryActive" x1="0" y1="0" x2="0" y2="1">
          <stop offset="5%" stopColor="var(--color-memoryActive)" stopOpacity={0.18} />
          <stop offset="95%" stopColor="var(--color-memoryActive)" stopOpacity={0.02} />
        </linearGradient>
        <linearGradient id="fillmemoryCache" x1="0" y1="0" x2="0" y2="1">
          <stop offset="5%" stopColor="var(--color-memoryCache)" stopOpacity={0.18} />
          <stop offset="95%" stopColor="var(--color-memoryCache)" stopOpacity={0.02} />
        </linearGradient>
      </defs>
    ),
    [],
  );

  const memoizedChart = useMemo(
    () => (
      <AreaChart data={statsChartData(stats, bounds)} accessibilityLayer>
        {gradientDefs}
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
              labelFormatter={(_, n) => {
                const created = (n.at(0)?.payload as ContainerStatView).created as number;
                return dayjs(created * 1000).format('MMM D, h:mm:ss A');
              }}
              valueFormatter={(value) => byteTransform(Number(value), 2)}
            />
          }
        />
        <Area
          dataKey="memoryActive"
          type="linear"
          isAnimationActive={false}
          connectNulls={false}
          activeDot={CHART_ACTIVE_DOT}
          fill="url(#fillmemoryActive)"
          stroke="var(--color-memoryActive)"
        />
        <Area
          dataKey="memoryCache"
          type="linear"
          isAnimationActive={false}
          connectNulls={false}
          activeDot={CHART_ACTIVE_DOT}
          fill="url(#fillmemoryCache)"
          stroke="var(--color-memoryCache)"
        />
        <ChartLegend content={<ChartLegendContent />} />
      </AreaChart>
    ),
    [stats, gradientDefs, bounds],
  );

  return isLoading ? (
    <Skeleton className="h-56.26 w-full rounded-xl" />
  ) : (
    <Card className="bg-background rounded-sm shadow-xs py-0">
      <MemoryUsageHeader container={container} controls={controls} />
      <CardContent className="px-2 sm:px-6">
        <ChartContainer config={chartConfig} className="aspect-auto h-62.5 w-full">
          {memoizedChart}
        </ChartContainer>
      </CardContent>
    </Card>
  );
};
interface MemoryUsageHeaderProps {
  container: ContainerStatsResource | undefined;
  controls?: ReactNode;
}

const MemoryUsageHeader = ({ container, controls }: MemoryUsageHeaderProps) => {
  const renderStat = (label: string, value: string | number | null | undefined) => (
    <StatsSummaryItem
      label={label}
      value={container?.state === 'Running' && value !== undefined ? byteTransform(value, 2) : '-'}
    />
  );

  return (
    <StatsPanelHeader
      title="Memory Usage"
      description="Memory usage excluding file cache over the selected period. Cache is shown separately."
      controls={controls}>
      {renderStat('Usage', container?.containerStat?.memoryActive)}
      {renderStat('Cache', container?.containerStat?.memoryCache)}
      {renderStat('Limit', container?.containerStat?.memoryLimit)}
    </StatsPanelHeader>
  );
};

export default MemoryUsage;
