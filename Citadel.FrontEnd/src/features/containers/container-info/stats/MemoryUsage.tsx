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
  stats: {
    label: 'Memory',
  },
  memoryUsage: {
    label: 'Memory Usage',
    color: 'var(--chart-1)',
  },
} satisfies ChartConfig;

const MemoryUsage = () => {
  const stats = useContextSelector(ContainerStatsContext, (v) => v?.stats);
  const isLoading = useContextSelector(ContainerStatsContext, (v) => v?.isLoading);

  return isLoading ? (
    <Skeleton className="h-[225px] w-full rounded-xl" />
  ) : (
    <Card className="bg-background rounded-sm shadow-xs">
      <CardContent className="px-2 pt-4 sm:px-6 sm:pt-6">
        <ChartContainer config={chartConfig} className="aspect-auto h-[250px] w-full">
          <AreaChart data={stats} accessibilityLayer>
            <defs>
              <linearGradient id="fillmemoryUsage" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="var(--color-memoryUsage)" stopOpacity={0.8} />
                <stop offset="95%" stopColor="var(--color-memoryUsage)" stopOpacity={0.1} />
              </linearGradient>
            </defs>
            <CartesianGrid vertical={true} />
            <XAxis dataKey="created" tickLine={false} axisLine={false} tickMargin={8} minTickGap={32} />

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
                        {((value as number) / 1024 / 1004).toFixed(2)}
                        <span className="font-normal text-muted-foreground">MB</span>
                      </div>
                    </>
                  )}
                />
              }
            />
            <Area
              dataKey="memoryUsage"
              type="natural"
              fill="url(#fillmemoryUsage)"
              stroke="var(--color-memoryUsage)"
              stackId="a"
            />
            <ChartLegend content={<ChartLegendContent />} />
          </AreaChart>
        </ChartContainer>
      </CardContent>
    </Card>
  );
};

export default MemoryUsage;
