import { Area, AreaChart, CartesianGrid, XAxis } from 'recharts';
import {
  ChartConfig,
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
} from '@/components/ui/chart';
import { Card, CardContent, CardHeader, CardDescription, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { useMemo } from 'react';
import dayjs from 'dayjs';
import { byteTransform } from '@/lib/bytes.helper';
import { ContainerStatView } from '@/api/generated/api.types';
import { DockerContainerView } from '@/api/types';

const MemoryUsage = ({
  stats,
  container,
  isLoading,
}: {
  stats: ContainerStatView[];
  container: DockerContainerView | undefined;
  isLoading: boolean;
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
      <MemoryUsageHeader container={container} />
      <CardContent className="px-2 sm:px-6">
        <ChartContainer config={chartConfig} className="aspect-auto h-62.5 w-full">
          {memoizedChart}
        </ChartContainer>
      </CardContent>
    </Card>
  );
};
interface MemoryUsageHeaderProps {
  container: DockerContainerView | undefined;
}

const MemoryUsageHeader = ({ container }: MemoryUsageHeaderProps) => {
  const renderStat = (label: string, value: string | number | undefined) => (
    <div className="flex flex-1 flex-col justify-center gap-1 border-t px-6 py-4 text-left even:border-l sm:border-t-0 sm:border-l sm:px-8 sm:py-6">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className="text-sm text-foreground font-medium leading-none">
        {container?.state === 'Running' && value !== undefined ? byteTransform(value, 2) : '-'}
      </span>
    </div>
  );

  return (
    <CardHeader className="flex flex-col items-stretch border-b p-0! sm:flex-row">
      <div className="flex flex-1 flex-col justify-center gap-1 px-6 pb-3 sm:pb-0">
        <CardTitle>Memory Usage</CardTitle>
        <CardDescription>Showing total memory usage for the past 24 hours</CardDescription>
      </div>
      <div className="flex">
        {renderStat('Active', container?.containerStat?.memoryActive)}
        {renderStat('Cache', container?.containerStat?.memoryCache)}
        {renderStat('Limit', container?.containerStat?.memoryLimit)}
      </div>
    </CardHeader>
  );
};

export default MemoryUsage;
