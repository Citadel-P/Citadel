import { useId, useMemo, useState, type ReactNode } from 'react';
import dayjs from 'dayjs';
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from 'recharts';
import type { ContainerStatView } from '@/api/generated/api.types';
import { StatsPanelHeader, StatsSummaryItem } from '@/components/custom/common';
import { NETWORK_CHART_COLORS } from '@/components/custom/chart-series-colors';
import { useStatsChartBounds } from '@/components/custom/stats-chart-range';
import { SegmentedControl } from '@/components/ui/segmented-control';
import { Card, CardContent } from '@/components/ui/card';
import {
  CHART_ACTIVE_DOT,
  CHART_HOVER_CURSOR,
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from '@/components/ui/chart';
import { Skeleton } from '@/components/ui/skeleton';
import { byteTransform } from '@/lib/bytes.helper';
import { mergeStatsByCreated } from '@/lib/live-stats';
import { statsChartData, statsValueAxis } from '@/lib/stats-chart';
import { filterStatsRange, statsAxis } from '@/lib/stats-range';

const series = {
  network: [
    {
      counter: 'rxBytes',
      delta: 'rxBytesDelta',
      rate: 'rxBytesPerSecond',
      label: 'Received',
      theme: NETWORK_CHART_COLORS.rxBytes,
    },
    {
      counter: 'txBytes',
      delta: 'txBytesDelta',
      rate: 'txBytesPerSecond',
      label: 'Sent',
      theme: NETWORK_CHART_COLORS.txBytes,
    },
  ],
  disk: [
    {
      counter: 'blockReadBytes',
      delta: 'blockReadBytesDelta',
      rate: 'blockReadBytesPerSecond',
      label: 'Read',
      theme: NETWORK_CHART_COLORS.rxBytes,
    },
    {
      counter: 'blockWriteBytes',
      delta: 'blockWriteBytesDelta',
      rate: 'blockWriteBytesPerSecond',
      label: 'Written',
      theme: NETWORK_CHART_COLORS.txBytes,
    },
  ],
} as const;

export type IoUsageProps = {
  stats: Omit<ContainerStatView, 'containerId'>[];
  isLoading: boolean;
  controls?: ReactNode;
};

export function IoUsage({ stats, isLoading, controls, kind }: IoUsageProps & { kind: keyof typeof series }) {
  const bounds = useStatsChartBounds();
  const id = useId().replaceAll(':', '');
  const [mode, setMode] = useState<'accumulated' | 'throughput'>(kind === 'network' ? 'accumulated' : 'throughput');
  const rate = mode === 'throughput';
  const fields = series[kind];
  const usage = useMemo(() => {
    const totals: (number | undefined)[] = [undefined, undefined];
    const data = mergeStatsByCreated([], bounds ? filterStatsRange(stats, bounds) : stats).map((stat) => {
      const point: { created: number; [key: string]: number | undefined } = { created: stat.created };
      fields.forEach((field, index) => {
        const delta = stat[field.delta];
        if (delta != null && Number.isFinite(delta) && delta >= 0) totals[index] = (totals[index] ?? 0) + delta;
        const speed = stat[field.rate];
        // Backend rates use raw sample intervals, never differences of bucket averages.
        point[field.counter] = rate
          ? speed != null && Number.isFinite(speed) && speed >= 0
            ? speed
            : undefined
          : totals[index];
      });
      return point;
    });
    return { data, totals };
  }, [stats, bounds, fields, rate]);
  const config = useMemo(
    () =>
      Object.fromEntries(
        fields.map((field) => [field.counter, { label: field.label, theme: field.theme }]),
      ) satisfies ChartConfig,
    [fields],
  );
  const unavailable = !usage.data.some((point) => fields.some((field) => point[field.counter] != null));
  const title = kind === 'network' ? 'Network Usage' : 'Disk I/O';

  return isLoading ? (
    <Skeleton className="h-56.25 w-full rounded-xl" />
  ) : (
    <Card className="bg-background rounded-sm shadow-xs py-0">
      <StatsPanelHeader
        title={title}
        description={
          rate ? 'Transfer rate · totals cover the selected period' : 'Traffic recorded over the selected period'
        }
        controls={
          <div className="flex flex-wrap items-center gap-2">
            <SegmentedControl
              aria-label={`${title} view`}
              value={mode}
              onValueChange={(value) => setMode(value as typeof mode)}
              options={[
                { value: 'accumulated', label: 'Accumulated' },
                { value: 'throughput', label: 'Throughput' },
              ]}
            />
            {controls}
          </div>
        }>
        {fields.map((field, index) => (
          <StatsSummaryItem
            key={field.counter}
            label={field.label}
            value={usage.totals[index] == null ? '-' : byteTransform(usage.totals[index], 2)}
          />
        ))}
      </StatsPanelHeader>
      <CardContent className="px-2 sm:px-6">
        {unavailable ? (
          <div className="flex h-62.5 items-center justify-center text-center text-sm text-muted-foreground">
            {kind === 'disk' && !stats.some((stat) => stat.blockReadBytes != null || stat.blockWriteBytes != null)
              ? 'Disk I/O is not reported in the selected period.'
              : 'Not enough measurements in this period. At least two observations are needed.'}
          </div>
        ) : (
          <ChartContainer config={config} className="aspect-auto h-62.5 w-full">
            <AreaChart data={statsChartData(usage.data, bounds)} accessibilityLayer>
              <defs>
                {fields.map((field) => (
                  <linearGradient key={field.counter} id={`${id}-${field.counter}`} x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor={`var(--color-${field.counter})`} stopOpacity={0.18} />
                    <stop offset="95%" stopColor={`var(--color-${field.counter})`} stopOpacity={0.02} />
                  </linearGradient>
                ))}
              </defs>
              <CartesianGrid vertical={false} strokeDasharray="3 3" />
              <YAxis {...statsValueAxis(rate ? 'bytesPerSecond' : 'bytes')} />
              <XAxis
                {...statsAxis(bounds)}
                dataKey="created"
                tickLine={false}
                axisLine={false}
                tickMargin={8}
                minTickGap={32}
                tickFormatter={(timestamp) =>
                  dayjs(timestamp * 1000).format(bounds && bounds.to - bounds.from > 86400 ? 'MMM D HH:mm' : 'HH:mm')
                }
              />
              <ChartTooltip
                cursor={CHART_HOVER_CURSOR}
                content={
                  <ChartTooltipContent
                    labelFormatter={(_, points) =>
                      dayjs(Number(points.at(0)?.payload.created) * 1000).format('MMM D, h:mm:ss A')
                    }
                    valueFormatter={(value) => `${byteTransform(Number(value), 2)}${rate ? '/s' : ''}`}
                  />
                }
              />
              {fields.map((field) => (
                <Area
                  key={field.counter}
                  dataKey={field.counter}
                  type="linear"
                  isAnimationActive={false}
                  connectNulls={false}
                  activeDot={CHART_ACTIVE_DOT}
                  fill={`url(#${id}-${field.counter})`}
                  stroke={`var(--color-${field.counter})`}
                />
              ))}
              <ChartLegend content={<ChartLegendContent />} />
            </AreaChart>
          </ChartContainer>
        )}
      </CardContent>
    </Card>
  );
}
