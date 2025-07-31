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
import { NullableOfContainerStatView } from '@/api/_generated';
import { byteTransform } from '@/lib/bytes.helper';
import { useContainerStatsContext } from './ContainerStatsContext';
import { DockerContainerView } from '@/api/models';

const NetworkUsageHeader = ({ container }: { container: DockerContainerView | undefined }) => (
  <CardHeader className="flex flex-col items-stretch border-b !p-0 sm:flex-row">
    <div className="flex flex-1 flex-col justify-center gap-1 px-6 pb-3 sm:pb-0">
      <CardTitle>Network Usage</CardTitle>
      <CardDescription>Showing total network usage for the past 24 hours</CardDescription>
    </div>
    <div className="flex">
      <div className="flex flex-1 flex-col justify-center gap-1 border-t px-6 py-4 text-left even:border-l sm:border-t-0 sm:border-l sm:px-8 sm:py-6">
        <span className="text-xs text-muted-foreground">Received</span>
        <span className="text-sm text-foreground font-medium leading-none">
          {container?.state === 'Running' && container?.containerStat
            ? byteTransform(container.containerStat.rxBytes, 2)
            : '-'}
        </span>
      </div>
      <div className="flex flex-1 flex-col justify-center gap-1 border-t px-6 py-4 text-left even:border-l sm:border-t-0 sm:border-l sm:px-8 sm:py-6">
        <span className="text-xs text-muted-foreground">Sent</span>
        <span className="text-sm text-foreground font-medium leading-none">
          {container?.state === 'Running' && container?.containerStat
            ? byteTransform(container.containerStat.txBytes, 2)
            : '-'}
        </span>
      </div>
    </div>
  </CardHeader>
);

const NetworkUsage = () => {
  const { stats, container, isLoading } = useContainerStatsContext();

  const chartConfig = useMemo(
    () =>
      ({
        rxBytes: {
          label: <span className="text-foreground">Data received</span>,
          color: 'var(--chart-1)',
        },
        txBytes: {
          label: <span className="text-foreground">Data sent</span>,
          color: 'var(--chart-2)',
        },
      }) satisfies ChartConfig,
    [],
  );

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
              labelFormatter={(_, n) => {
                const created = (n.at(0)?.payload as NullableOfContainerStatView).created as number;
                return <span className="text-foreground">{dayjs(created * 1000).format('HH:mm:ss')}</span>;
              }}
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
                    {((value as number) / 1024 / 1024).toFixed(2)}
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
    [stats, gradientDefs, chartConfig],
  );

  return isLoading ? (
    <Skeleton className="h-[225px] w-full rounded-xl" />
  ) : (
    <Card className="bg-background rounded-sm shadow-xs py-0">
      <NetworkUsageHeader container={container} />
      <CardContent className="px-2 sm:px-6">
        <ChartContainer config={chartConfig} className="aspect-auto h-[250px] w-full">
          {memoizedChart}
        </ChartContainer>
      </CardContent>
    </Card>
  );
};

export default NetworkUsage;
