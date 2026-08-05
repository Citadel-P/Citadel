import { Area, AreaChart, CartesianGrid, XAxis } from 'recharts';
import {
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
import { ContainerStatView } from '@/api/generated/api.types';
import type { ContainerStatsResource } from './types';

const CpuUsageHeader = ({
  container,
  windowHours,
  controls,
}: {
  container: ContainerStatsResource | undefined;
  windowHours: number;
  controls?: ReactNode;
}) => (
  <StatsPanelHeader
    title="CPU Usage"
    description={`Showing total CPU usage for the past ${windowHours} hours`}
    controls={controls}>
    <StatsSummaryItem
      label="Usage"
      value={
        container?.state === 'Running' && container?.containerStat && container.containerStat.cpuUsage
          ? `${(container.containerStat.cpuUsage as number).toFixed(2)}%`
          : '-'
      }
    />
  </StatsPanelHeader>
);

const CpuUsage = ({
  stats,
  container,
  isLoading,
  windowHours,
  controls,
}: {
  stats: ContainerStatView[];
  container: ContainerStatsResource | undefined;
  isLoading: boolean;
  windowHours: number;
  controls?: ReactNode;
}) => {
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

  const gradientDefs = useMemo(
    () => (
      <defs>
        <linearGradient id="fillcpuUsage" x1="0" y1="0" x2="0" y2="1">
          <stop offset="5%" stopColor="var(--color-cpuUsage)" stopOpacity={0.8} />
          <stop offset="95%" stopColor="var(--color-cpuUsage)" stopOpacity={0.1} />
        </linearGradient>
      </defs>
    ),
    [],
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
          tickFormatter={(timestamp) => dayjs(timestamp * 1000).format('HH:mm:ss')}
        />
        <ChartTooltip
          cursor={false}
          defaultIndex={1}
          content={
            <ChartTooltipContent
              nameKey="stats"
              indicator="dot"
              labelFormatter={(_, n) => {
                const created = (n.at(0)?.payload as ContainerStatView).created as number;
                return <span className="text-foreground">{dayjs(created * 1000).format('HH:mm:ss')}</span>;
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
                  {chartConfig['cpuUsage']?.label || name}
                  <div className="ml-auto flex items-baseline gap-0.5 font-mono font-medium tabular-nums text-foreground">
                    {(value as number).toFixed(2)}
                    <span className="font-normal text-muted-foreground">%</span>
                  </div>
                </>
              )}
            />
          }
        />
        <Area dataKey="cpuUsage" type="natural" fill="url(#fillcpuUsage)" stroke="var(--color-cpuUsage)" stackId="a" />
        <ChartLegend content={<ChartLegendContent />} />
      </AreaChart>
    ),
    [stats, gradientDefs, chartConfig],
  );

  return isLoading ? (
    <Skeleton className="h-56.25 w-full rounded-xl" />
  ) : (
    <Card className="bg-background rounded-sm shadow-xs py-0">
      <CpuUsageHeader container={container} windowHours={windowHours} controls={controls} />
      <CardContent className="px-2 sm:px-6">
        <ChartContainer config={chartConfig} className="aspect-auto h-62.5 w-full">
          {memoizedChart}
        </ChartContainer>
      </CardContent>
    </Card>
  );
};

export default CpuUsage;
