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
import { byteTransform } from '@/lib/bytes.helper';

const NetworkUsage = () => {
  const [isPending, startTransition] = useTransition();
  const { stats, container, isLoading } = useContainerStatsContext();
  const [transitionedStats, setTransitionedStats] = useState(stats);

  const downsampledStats = useMemo(() => {
    const step = Math.ceil(stats.length / (60 * 24)); // Keep only 1440 points
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
      <div className="font-medium text-foreground">Network I/O:</div>
      <div className="text-xs">
        {container?.state === 'Running' && container?.lastStats
          ? byteTransform(container?.lastStats.rxBytes ?? 0, 2) +
            ' / ' +
            byteTransform(container?.lastStats.txBytes ?? 0, 2)
          : '-/-'}
      </div>
    </div>
  );
};

export default NetworkUsage;
