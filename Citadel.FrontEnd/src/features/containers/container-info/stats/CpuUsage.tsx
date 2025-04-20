import { Area, AreaChart, CartesianGrid, XAxis } from 'recharts';
import {
  ChartConfig,
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
} from '@/components/ui/chart';
import { Card, CardContent } from '@/components/ui/card';
import { ContainerStatsContext } from './ContainerStatsProvider';
import { useContextSelector } from 'use-context-selector';
import { Skeleton } from '@/components/ui/skeleton';
import { useMemo, useState, useTransition, useEffect } from 'react';
import dayjs from 'dayjs';

const CpuUsage = () => {
  const stats = useContextSelector(ContainerStatsContext, (v) => v?.stats) || [];
  const isLoading = useContextSelector(ContainerStatsContext, (v) => v?.isLoading) || false;

  // Use transition for smoother updates
  const [isPending, startTransition] = useTransition();

  // State to hold the transitioned stats
  const [transitionedStats, setTransitionedStats] = useState(stats);

  // Downsample the stats to reduce the number of data points
  const downsampledStats = useMemo(() => {
    const step = Math.ceil(stats.length / (24 * 60)); // Keep only 1440 points
    return stats.filter((_, index) => index % step === 0);
  }, [stats]);

  // Start a transition when stats change
  useEffect(() => {
    startTransition(() => {
      setTransitionedStats(downsampledStats);
    });
  }, [downsampledStats, startTransition]);

  // Memoize chart configuration
  const chartConfig = useMemo(
    () =>
      ({
        stats: {
          label: 'Cpu',
        },
        cpuUsage: {
          label: 'Cpu Usage',
          color: 'var(--chart-1)',
        },
      }) satisfies ChartConfig,
    [],
  );

  // Memoize gradient definitions
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

  // Memoize the chart rendering logic
  const memoizedChart = useMemo(
    () => (
      <AreaChart data={transitionedStats} accessibilityLayer>
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
              formatter={(value, name) => (
                <>
                  <div
                    className="h-2.5 w-2.5 shrink-0 rounded-[2px] bg-(--color-bg)"
                    style={
                      {
                        '--color-bg': `var(--color-${name})`,
                      } as React.CSSProperties
                    }
                  />
                  {chartConfig['stats']?.label || name}
                  <div className="ml-auto flex items-baseline gap-0.5 font-mono font-medium tabular-nums text-foreground">
                    {value}
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
    [transitionedStats, gradientDefs, chartConfig],
  );

  return isLoading || isPending ? (
    <Skeleton className="h-[225px] w-full rounded-xl" />
  ) : (
    <Card className="bg-background rounded-sm shadow-xs">
      <CardContent className="px-2 pt-4 sm:px-6 sm:pt-6">
        <ChartContainer config={chartConfig} className="aspect-auto h-[250px] w-full">
          {memoizedChart}
        </ChartContainer>
      </CardContent>
    </Card>
  );
};

export default CpuUsage;
