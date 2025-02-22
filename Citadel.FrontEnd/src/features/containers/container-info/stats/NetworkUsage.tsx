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
const chartConfig = {
  rxBytes: {
    label: 'Data received',
    color: 'hsl(var(--chart-1))',
  },
  txBytes: {
    label: 'Data sent',
    color: 'hsl(var(--chart-2))',
  },
} satisfies ChartConfig;

const NetworkUsage = () => {
  const stats = useContextSelector(ContainerStatsContext, (v) => v?.stats);
  const isLoading = useContextSelector(ContainerStatsContext, (v) => v?.isLoading);

  return isLoading ? (
    <Skeleton className="h-[225px] w-full rounded-xl" />
  ) : (
    <Card className="bg-background rounded-sm shadow-sm">
      <CardContent className="px-2 pt-4 sm:px-6 sm:pt-6">
        <ChartContainer config={chartConfig} className="aspect-auto h-[250px] w-full">
          <AreaChart data={stats} accessibilityLayer>
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
            <CartesianGrid vertical={true} />
            <XAxis dataKey="created" tickLine={false} axisLine={false} tickMargin={8} minTickGap={32} />

            <ChartTooltip
              cursor={false}
              defaultIndex={1}
              content={
                <ChartTooltipContent
                  formatter={(value, name) => (
                    <>
                      <div
                        className="h-2.5 w-2.5 shrink-0 rounded-[2px] bg-[--color-bg]"
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
        </ChartContainer>
      </CardContent>
    </Card>
  );
};

export default NetworkUsage;
