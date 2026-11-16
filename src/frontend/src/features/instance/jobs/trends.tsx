import { Line, LineChart, XAxis, YAxis, CartesianGrid } from 'recharts';
import type { JobHistoryBucketView, JobHistorySessionView } from '@/api/generated/api.types';
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  ChartLegend,
  ChartLegendContent,
} from '@/components/ui/chart';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import type { DateTimeFormatter } from '@/lib/date-time';
import { cn } from '@/lib/utils';

export function trendPoints(buckets: JobHistoryBucketView[]) {
  const ordered = [...buckets].sort((a, b) => a.at - b.at);
  const result: { at: number; productive: number | null; empty: number | null }[] = [];
  let previousEnd: number | undefined;
  for (const b of ordered) {
    if (previousEnd != null && b.at > previousEnd) result.push({ at: previousEnd, productive: null, empty: null });
    previousEnd = b.at + b.resolutionMs;
    result.push({
      at: b.at,
      productive: b.gap || !b.productive ? null : b.productiveMs / b.productive,
      empty: b.gap || !b.noWork ? null : b.noWorkMs / b.noWork,
    });
  }
  return result;
}
export function JobTrends({
  buckets,
  sessions,
  formatDateTime,
  collection = false,
}: {
  buckets: JobHistoryBucketView[];
  sessions: JobHistorySessionView[];
  formatDateTime: DateTimeFormatter;
  collection?: boolean;
}) {
  if (!buckets.length) return <WorkTrendChart values={[]} collection={collection} formatDateTime={formatDateTime} />;
  return (
    <div className="space-y-4">
      <p className="text-xs text-muted-foreground">
        {collection
          ? 'Average time to collect one round of container measurements.'
          : 'Average elapsed time per check, separated by checks with work and checks with nothing to process.'}{' '}
        Includes I/O waits, not CPU usage. Longer checks can reflect more work; they do not by themselves indicate a
        problem. Recent windows cover five minutes; older windows cover one hour. Boundary windows may extend beyond the
        selected range.
      </p>
      {sessions
        .filter((s) => buckets.some((b) => b.bootId === s.snapshot.bootId))
        .map((session) => {
          const values = buckets.filter((b) => b.bootId === session.snapshot.bootId);
          return (
            <div key={session.snapshot.bootId}>
              <p className="mb-2 text-xs text-muted-foreground">
                Core session started {formatDateTime(session.snapshot.startedAt, 'exact')}
              </p>
              <WorkTrendChart values={values} collection={collection} formatDateTime={formatDateTime} />
            </div>
          );
        })}
      <details className="[&>[data-slot=table-container]]:max-h-80 [&>[data-slot=table-container]]:overflow-y-auto">
        <summary className="cursor-pointer text-sm">Recent measurement windows</summary>
        <Table>
          <TableHeader className="sticky top-0 z-10 bg-card">
            <TableRow>
              <TableHead>Window / startup</TableHead>
              <TableHead>Work completed</TableHead>
              <TableHead>Productive / empty checks</TableHead>
              <TableHead>Reported failures</TableHead>
              <TableHead>Reported backlog</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {buckets.slice(0, 12).map((b) => (
              <TableRow key={`${b.bootId}-${b.at}`}>
                <TableCell>
                  {formatDateTime(b.at)} · {b.resolutionMs === 3_600_000 ? '1 hour' : '5 minutes'} ·{' '}
                  {b.bootId.slice(0, 8)}
                  {b.gap && ' · Observation gap'}
                </TableCell>
                <TableCell>
                  {b.units == null ? 'Unavailable' : `${b.units.toLocaleString()} ${b.unit ?? 'units'}`}
                </TableCell>
                <TableCell>
                  {b.productive} / {b.noWork}
                </TableCell>
                <TableCell>{b.failures}</TableCell>
                <TableCell>
                  {b.backlog == null ? 'Unavailable' : `${b.backlogIsLowerBound ? 'At least ' : ''}${b.backlog}`}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </details>
    </div>
  );
}

function WorkTrendChart({
  values,
  collection,
  formatDateTime,
}: {
  values: JobHistoryBucketView[];
  collection: boolean;
  formatDateTime: DateTimeFormatter;
}) {
  const points = trendPoints(values);
  const hasData = points.some((point) => point.productive !== null || point.empty !== null);
  return (
    <div className="relative">
      <ChartContainer
        className={cn('h-40 w-full', !hasData && 'pointer-events-none opacity-30')}
        inert={!hasData}
        aria-hidden={!hasData}
        config={{
          productive: {
            label: collection ? 'Collected measurements (ms)' : 'Checks with work (ms)',
            color: 'var(--chart-1)',
          },
          empty: {
            label: collection ? 'Nothing to collect (ms)' : 'Nothing to process (ms)',
            color: 'var(--chart-2)',
          },
        }}>
        <LineChart accessibilityLayer={hasData} data={points}>
          <ChartLegend content={<ChartLegendContent />} />
          <CartesianGrid vertical={false} />
          <XAxis
            dataKey="at"
            type="number"
            domain={hasData ? ['dataMin', 'dataMax'] : [0, 1]}
            tick={hasData}
            tickFormatter={(v: number) => new Date(v).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
          />
          <YAxis width={55} unit=" ms" tick={hasData} domain={hasData ? undefined : [0, 1]} />
          {hasData && (
            <ChartTooltip
              content={
                <ChartTooltipContent
                  labelFormatter={(_label, items) => (items.length ? formatDateTime(Number(items[0].payload.at)) : '')}
                />
              }
            />
          )}
          <Line
            dataKey="productive"
            stroke="var(--color-productive)"
            dot={values.length <= 24}
            connectNulls={false}
            isAnimationActive={false}
          />
          <Line
            dataKey="empty"
            stroke="var(--color-empty)"
            dot={values.length <= 24}
            connectNulls={false}
            isAnimationActive={false}
          />
        </LineChart>
      </ChartContainer>
      {!hasData && (
        <div className="absolute inset-0 flex items-center justify-center text-sm text-muted-foreground" role="status">
          No data
        </div>
      )}
    </div>
  );
}
