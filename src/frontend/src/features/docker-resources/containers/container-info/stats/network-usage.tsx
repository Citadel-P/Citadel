import { statsChartData, statsValueAxis } from '@/lib/stats-chart';
import { useStatsChartBounds } from '@/components/custom/stats-chart-range';
import { statsAxis } from '@/lib/stats-range';
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from 'recharts';
import {
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
          cursor={false}
          content={
            <ChartTooltipContent
              labelFormatter={(_, n) => {
                const created = (n.at(0)?.payload as ContainerStatView).created as number;
                return <span className="text-foreground">{dayjs(created * 1000).format('MMM D, HH:mm:ss')}</span>;
              }}
              formatter={(value, name) => (
                <>
                  <div
                    className="h-2.5 w-2.5 shrink-0 rounded-xs bg-(--color-bg)"
                    style={
                      {
                        '--color-bg': `var(--color-${name})`,
                      } as React.CSSProperties
                    }
                  />
                  {chartConfig[name as keyof typeof chartConfig]?.label || name}
                  <div className="ml-auto flex items-baseline gap-0.5 font-mono font-medium tabular-nums text-foreground">
                    {((value as number) / 1024 / 1024).toFixed(2)}
                    <span className="font-normal text-muted-foreground">MB</span>
                  </div>
                </>
              )}
            />
          }
        />
        <Area
          dataKey="rxBytes"
          type="linear"
          isAnimationActive={false}
          connectNulls={false}
          fill="url(#fillrxBytes)"
          stroke="var(--color-rxBytes)"
        />
        <Area
          dataKey="txBytes"
          type="linear"
          isAnimationActive={false}
          connectNulls={false}
          fill="url(#filltxBytes)"
          stroke="var(--color-txBytes)"
        />
        <ChartLegend content={<ChartLegendContent />} />
      </AreaChart>
    ),
    [stats, gradientDefs, chartConfig, bounds],
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
