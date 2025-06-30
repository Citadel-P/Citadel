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
import { useContainerStatsContext } from './ContainerStatsProvider';
import { Skeleton } from '@/components/ui/skeleton';
import { useMemo, useState, useTransition, useEffect } from 'react';
import dayjs from 'dayjs';

const NetworkUsage = () => {
  const { stats, isLoading } = useContainerStatsContext();

  // Use transition for smoother updates
  const [isPending, startTransition] = useTransition();

  // State to hold the transitioned stats
  const [transitionedStats, setTransitionedStats] = useState(stats);

  // Downsample the stats to reduce the number of data points
  const downsampledStats = useMemo(() => {
    const step = Math.ceil(stats.length / (60 * 24)); // Keep only 1440 points
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
        rxBytes: {
          label: 'Data received',
          color: 'var(--chart-1)',
        },
        txBytes: {
          label: 'Data sent',
          color: 'var(--chart-2)',
        },
      }) satisfies ChartConfig,
    [],
  );

  // Memoize gradient definitions
  const gradientDefs = useMemo(
    () => (
      <defs>
        <linearGradient id="fillrxBytes" x1="0" y1="0" x2="0" y2="1">
          <stop offset="5%" stopColor="var(--color-rxBytes)" stopOpacity={0.8} />
          <stop offset="95%" stopColor="var(--color-rxBytes)" stopOpacity={0.1} />
        </linearGradient>
        <linearGradient id="filltxBytes" x1="0" y1="0" x2="0" y2="1">
          <stop offset="5%" stopColor="var(--color-txBytes)" stopOpacity={0.8} />
          <stop offset="95%" stopColor="var(--color-txBytes)" stopOpacity={0.1} />
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
                  {chartConfig[name as keyof typeof chartConfig]?.label || name}
                  <div className="ml-auto flex items-baseline gap-0.5 font-mono font-medium tabular-nums text-foreground">
                    {((value as number) / 1024 / 1004).toFixed(2)}
                    <span className="font-normal text-muted-foreground">MB</span>
                  </div>
                </>
              )}
            />
          }
        />
        <Area dataKey="rxBytes" type="natural" fill="url(#fillrxBytes)" stroke="var(--color-rxBytes)" stackId="a" />
        <Area dataKey="txBytes" type="natural" fill="url(#filltxBytes)" stroke="var(--color-txBytes)" stackId="a" />
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

export default NetworkUsage;
