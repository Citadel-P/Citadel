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
import { byteTransform } from '@/lib/bytes.helper';
import { ContainerStatView, ContainerDataView } from '@/api/generated/api.types';

const MemoryUsage = ({
  stats,
  container,
  isLoading,
  windowHours,
  controls,
}: {
  stats: ContainerStatView[];
  container: ContainerDataView | undefined;
  isLoading: boolean;
  windowHours: number;
  controls?: ReactNode;
}) => {
  const chartConfig = useMemo(
    () =>
      ({
        memoryActive: {
          label: <span className="text-foreground">Active</span>,
          color: 'var(--chart-1)',
        },
        memoryCache: {
          label: <span className="text-foreground">Cache</span>,
          color: 'var(--chart-2)',
        },
      }) satisfies ChartConfig,
    [],
  );

  const gradientDefs = useMemo(
    () => (
      <defs>
        <linearGradient id="fillmemoryActive" x1="0" y1="0" x2="0" y2="1">
          <stop offset="5%" stopColor="var(--color-memoryActive)" stopOpacity={0.8} />
          <stop offset="95%" stopColor="var(--color-memoryActive)" stopOpacity={0.1} />
        </linearGradient>
        <linearGradient id="fillmemoryCache" x1="0" y1="0" x2="0" y2="1">
          <stop offset="5%" stopColor="var(--color-memoryCache)" stopOpacity={0.8} />
          <stop offset="95%" stopColor="var(--color-memoryCache)" stopOpacity={0.1} />
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
          dataKey="memoryActive"
          type="natural"
          fill="url(#fillmemoryActive)"
          stroke="var(--color-memoryActive)"
          stackId="a"
        />
        <Area
          dataKey="memoryCache"
          type="natural"
          fill="url(#fillmemoryCache)"
          stroke="var(--color-memoryCache)"
          stackId="a"
        />
        <ChartLegend content={<ChartLegendContent />} />
      </AreaChart>
    ),
    [stats, gradientDefs, chartConfig],
  );

  return isLoading ? (
    <Skeleton className="h-56.26 w-full rounded-xl" />
  ) : (
    <Card className="bg-background rounded-sm shadow-xs py-0">
      <MemoryUsageHeader container={container} windowHours={windowHours} controls={controls} />
      <CardContent className="px-2 sm:px-6">
        <ChartContainer config={chartConfig} className="aspect-auto h-62.5 w-full">
          {memoizedChart}
        </ChartContainer>
      </CardContent>
    </Card>
  );
};
interface MemoryUsageHeaderProps {
  container: ContainerDataView | undefined;
  windowHours: number;
  controls?: ReactNode;
}

const MemoryUsageHeader = ({ container, windowHours, controls }: MemoryUsageHeaderProps) => {
  const renderStat = (label: string, value: string | number | null | undefined) => (
    <StatsSummaryItem
      label={label}
      value={container?.state === 'Running' && value !== undefined ? byteTransform(value, 2) : '-'}
    />
  );

  return (
    <StatsPanelHeader
      title="Memory Usage"
      description={`Showing total memory usage for the past ${windowHours} hours`}
      controls={controls}>
      {renderStat('Active', container?.containerStat?.memoryActive)}
      {renderStat('Cache', container?.containerStat?.memoryCache)}
      {renderStat('Limit', container?.containerStat?.memoryLimit)}
    </StatsPanelHeader>
  );
};

export default MemoryUsage;
