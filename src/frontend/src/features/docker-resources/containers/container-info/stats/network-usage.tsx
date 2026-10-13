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
import { NETWORK_CHART_COLORS } from '@/components/custom/chart-series-colors';
import { StatsPanelHeader, StatsSummaryItem } from '@/components/custom/common';
import { Card, CardContent } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { ReactNode, useMemo } from 'react';
import dayjs from 'dayjs';
import { ContainerStatView } from '@/api/generated/api.types';
import type { ContainerStatsResource } from './types';
import { byteTransform } from '@/lib/bytes.helper';

const NetworkUsageHeader = ({
  container,
  controls,
}: {
  container: ContainerStatsResource | undefined;
  controls?: ReactNode;
}) => (
  <StatsPanelHeader
    title="Network Usage"
    description="Showing total network usage over the selected period"
    controls={controls}>
    <StatsSummaryItem
      label="Received"
      value={
        container?.state === 'Running' && container?.containerStat
          ? byteTransform(container.containerStat.rxBytes, 2)
          : '-'
      }
    />
    <StatsSummaryItem
      label="Sent"
      value={
        container?.state === 'Running' && container?.containerStat
          ? byteTransform(container.containerStat.txBytes, 2)
          : '-'
      }
    />
  </StatsPanelHeader>
);

const NetworkUsage = ({
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
        rxBytes: {
          label: 'Data received',
          theme: NETWORK_CHART_COLORS.rxBytes,
        },
        txBytes: {
          label: 'Data sent',
          theme: NETWORK_CHART_COLORS.txBytes,
        },
      }) satisfies ChartConfig,
    [],
  );

  const gradientDefs = useMemo(
    () => (
      <defs>
        <linearGradient id="fillrxBytes" x1="0" y1="0" x2="0" y2="1">
          <stop offset="5%" stopColor="var(--color-rxBytes)" stopOpacity={0.18} />
          <stop offset="95%" stopColor="var(--color-rxBytes)" stopOpacity={0.02} />
        </linearGradient>
        <linearGradient id="filltxBytes" x1="0" y1="0" x2="0" y2="1">
          <stop offset="5%" stopColor="var(--color-txBytes)" stopOpacity={0.18} />
          <stop offset="95%" stopColor="var(--color-txBytes)" stopOpacity={0.02} />
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
              labelFormatter={(_, n) => {
                const created = (n.at(0)?.payload as ContainerStatView).created as number;
                return dayjs(created * 1000).format('MMM D, h:mm:ss A');
              }}
              valueFormatter={(value) => `${(Number(value) / 1024 / 1024).toFixed(2)} MB`}
            />
          }
        />
        <Area
          dataKey="rxBytes"
          type="linear"
          isAnimationActive={false}
          connectNulls={false}
          activeDot={CHART_ACTIVE_DOT}
          fill="url(#fillrxBytes)"
          stroke="var(--color-rxBytes)"
        />
        <Area
          dataKey="txBytes"
          type="linear"
          isAnimationActive={false}
          connectNulls={false}
          activeDot={CHART_ACTIVE_DOT}
          fill="url(#filltxBytes)"
          stroke="var(--color-txBytes)"
        />
        <ChartLegend content={<ChartLegendContent />} />
      </AreaChart>
    ),
    [stats, gradientDefs, bounds],
  );

  return isLoading ? (
    <Skeleton className="h-56.25 w-full rounded-xl" />
  ) : (
    <Card className="bg-background rounded-sm shadow-xs py-0">
      <NetworkUsageHeader container={container} controls={controls} />
      <CardContent className="px-2 sm:px-6">
        <ChartContainer config={chartConfig} className="aspect-auto h-62.5 w-full">
          {memoizedChart}
        </ChartContainer>
      </CardContent>
    </Card>
  );
};

export default NetworkUsage;
