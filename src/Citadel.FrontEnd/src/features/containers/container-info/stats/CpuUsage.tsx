import { Area, AreaChart, CartesianGrid, XAxis } from 'recharts';
import {
  ChartConfig,
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
} from '@/components/ui/chart';
import { Card, CardContent, CardHeader, CardDescription } from '@/components/ui/card';
import { useContainerStatsContext } from './ContainerStatsProvider';
import { Skeleton } from '@/components/ui/skeleton';
import { useMemo, useState, useTransition, useEffect } from 'react';
import dayjs from 'dayjs';
import { ContainerView } from '@/api/_generated';

const CpuUsage = () => {
  const [isPending, startTransition] = useTransition();
  const { stats, container, isLoading } = useContainerStatsContext();
  const [transitionedStats, setTransitionedStats] = useState(stats);

  const downsampledStats = useMemo(() => {
    const step = Math.ceil(stats.length / (24 * 60)); // Keep only 1440 points
    return stats.filter((_, index) => index % step === 0);
  }, [stats]);

  useEffect(() => {
    startTransition(() => {
      setTransitionedStats(downsampledStats);
    });
  }, [downsampledStats, startTransition]);

  useEffect(() => {
    if (container?.lastStats) {
      setTransitionedStats((prev) => [...prev, container?.lastStats]);
    }
  }, [container]);

  const chartConfig = useMemo(
    () =>
      ({
        stats: {
          label: 'CPU',
        },
        cpuUsage: {
          label: 'CPU Usage',
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
    [transitionedStats, gradientDefs, chartConfig],
  );

  return isLoading || isPending ? (
    <Skeleton className="h-[225px] w-full rounded-xl" />
  ) : (
    <Card className="bg-background rounded-sm shadow-xs">
      <CardHeader>
        <CardDescription className="ml-1.5">
          <CardInfo container={container} />
        </CardDescription>
      </CardHeader>
      <CardContent className="px-2 sm:px-6">
        <ChartContainer config={chartConfig} className="aspect-auto h-[250px] w-full">
          {memoizedChart}
        </ChartContainer>
      </CardContent>
    </Card>
  );
};

const CardInfo = ({ container }: { container: ContainerView | undefined }) => {
  return (
    <div className="flex items-center gap-x-1">
      <div className="font-medium text-foreground">CPU usage:</div>
      <div className="text-xs">
        {container?.state === 'Running' && container?.lastStats ? container?.lastStats.cpuUsage + '%' : '-%'}
      </div>
    </div>
  );
};

export default CpuUsage;
