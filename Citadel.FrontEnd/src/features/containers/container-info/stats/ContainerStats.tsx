import { Area, AreaChart, CartesianGrid, XAxis } from 'recharts';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
  ChartConfig,
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
} from '@/components/ui/chart';
import { useContainerStatsContext } from './ContainerStatsProvider';

const chartConfig = {
  stats: {
    label: 'Memory',
  },
  memoryUsage: {
    label: 'Memory Usage',
    color: 'hsl(var(--chart-1))',
  },
} satisfies ChartConfig;

const ContainerStats = () => {
  const { stats } = useContainerStatsContext();

  return (
    <Card className="bg-background rounded-sm shadow-none">
      <CardHeader className="flex items-center gap-2 space-y-0 border-b py-5 sm:flex-row">
        <div className="grid flex-1 gap-1 text-center sm:text-left">
          <CardTitle className="text-base">Memory usage</CardTitle>
          <CardDescription className="text-sm">Showing snapshots for the last 24 hours</CardDescription>
        </div>
      </CardHeader>
      <CardContent className="px-2 pt-4 sm:px-6 sm:pt-6">
        <ChartContainer config={chartConfig} className="aspect-auto h-[250px] w-full">
          <AreaChart data={stats} accessibilityLayer>
            <defs>
              <linearGradient id="fillmemoryUsage" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="var(--color-memoryUsage)" stopOpacity={0.8} />
                <stop offset="95%" stopColor="var(--color-memoryUsage)" stopOpacity={0.1} />
              </linearGradient>
              <linearGradient id="fillMobile" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="var(--color-mobile)" stopOpacity={0.8} />
                <stop offset="95%" stopColor="var(--color-mobile)" stopOpacity={0.1} />
              </linearGradient>
            </defs>
            <CartesianGrid vertical={false} />
            <XAxis
              dataKey="created"
              tickLine={false}
              axisLine={false}
              tickMargin={8}
              minTickGap={32}
              tickFormatter={(value) => new Date(value).toLocaleTimeString('en-US', { timeStyle: 'short' })}
            />

            <ChartTooltip
              cursor={false}
              defaultIndex={1}
              content={
                <ChartTooltipContent
                  nameKey="stats"
                  indicator="dot"
                  labelFormatter={(value) => new Date(value).toLocaleTimeString('en-US', { timeStyle: 'medium' })}
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
                      {chartConfig['stats']?.label || name}
                      <div className="ml-auto flex items-baseline gap-0.5 font-mono font-medium tabular-nums text-foreground">
                        {value}
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

export default ContainerStats;
