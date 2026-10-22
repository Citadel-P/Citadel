import { createContext, useContext, useMemo, useRef, useState, type KeyboardEvent } from 'react';
import { CartesianGrid, Scatter, ScatterChart, XAxis, YAxis } from 'recharts';
import { X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ContainerHealthStatus as Status, type ContainerHealthTransitionView } from '@/api/generated/api.types';
import { ChartContainer, ChartTooltip, ChartTooltipContent } from '@/components/ui/chart';
import { HealthLabel, healthLabels, healthTime, healthTone } from '../health';
import { groupHealthChartPoints } from './health-chart-points';

const primaryStates = [Status.Healthy, Status.Starting, Status.Unhealthy];
const neutralStates = [Status.NotRunning, Status.NoCheck, Status.Disabled, Status.Unknown];
const axisWidth = 78;
const rightMargin = 16;
const markerSpacing = 14;

type Point = { timestamp: number; lane: number; index: number; entry: ContainerHealthTransitionView };

type PointSelection = {
  selectedIndex: number;
  hasSelection: boolean;
  onChoose: (index: number) => void;
  move: (event: KeyboardEvent<SVGGElement>, index: number) => void;
};
const PointSelectionContext = createContext<PointSelection | null>(null);
const pointShape = <HealthObservationPoint />;

export function ContainerHealthEvents({
  entries,
  now,
  start,
}: {
  entries: ContainerHealthTransitionView[];
  now: number;
  start: number;
}) {
  const [selectedKey, setSelectedKey] = useState<string>();
  const [chartWidth, setChartWidth] = useState(600);
  const chartRef = useRef<HTMLDivElement>(null);
  const groups = useMemo(
    () =>
      groupHealthChartPoints(
        entries,
        ((now - start) / 1000 / Math.max(1, chartWidth - axisWidth - rightMargin)) * markerSpacing,
      ),
    [entries, now, start, chartWidth],
  );
  const selectedIndex = Math.max(
    0,
    groups.findIndex((group) => selectedKey !== undefined && group.keys.includes(selectedKey)),
  );
  const selected = groups[selectedIndex]?.entry;
  const hasSelection = selectedKey !== undefined && groups.some((group) => group.keys.includes(selectedKey));
  const states = useMemo(
    () => [
      ...primaryStates,
      ...neutralStates.filter((status) => entries.some((entry) => entry.health.status === status)),
    ],
    [entries],
  );
  const points: Point[] = useMemo(
    () =>
      groups.map(({ entry }, index) => ({
        timestamp: entry.observedAt * 1000,
        lane: states.indexOf(entry.health.status),
        index,
        entry,
      })),
    [groups, states],
  );
  const move = (event: KeyboardEvent<SVGGElement>, index: number) => {
    const next =
      event.key === 'ArrowLeft'
        ? Math.min(groups.length - 1, index + 1)
        : event.key === 'ArrowRight'
          ? Math.max(0, index - 1)
          : event.key === 'Home'
            ? groups.length - 1
            : event.key === 'End'
              ? 0
              : undefined;
    if (next === undefined) return;
    event.preventDefault();
    event.currentTarget.ownerSVGElement?.querySelector<SVGGElement>(`[data-health-point="${next}"]`)?.focus();
  };

  // Recharts keys animation children on changed series props, even with animation disabled.
  // Keep the chart stable during selection so keyboard focus stays on the point.
  const chart = useMemo(() => {
    const tickTime = (value: number) =>
      now - start >= 86_400_000
        ? new Date(value).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
        : new Date(value).toLocaleTimeString(undefined, {
            hour: '2-digit',
            minute: '2-digit',
            ...(now - start < 3_600_000 ? { second: '2-digit' as const } : {}),
            hourCycle: 'h23',
          });
    return (
      <ChartContainer
        config={{}}
        onChartResize={setChartWidth}
        className="aspect-auto w-full px-2 pt-4 sm:px-3"
        style={{ height: Math.max(184, states.length * 28 + 64) }}
        aria-label="Health observations by time and state">
        <ScatterChart margin={{ top: 12, right: rightMargin, bottom: 0, left: 0 }}>
          <CartesianGrid vertical={false} strokeDasharray="3 3" />
          <XAxis
            type="number"
            dataKey="timestamp"
            domain={[start, now]}
            ticks={[start, (start + now) / 2, now]}
            tickFormatter={tickTime}
            tick={(props: unknown) => {
              const { x, y, payload } = props as { x: number; y: number; payload: { value: number } };
              return (
                <text
                  x={x}
                  y={y}
                  textAnchor={payload.value === start ? 'start' : payload.value === now ? 'end' : 'middle'}
                  className="fill-muted-foreground text-[10px]">
                  <tspan x={x} dy="0.9em">
                    {tickTime(payload.value)}
                  </tspan>
                  {now - start >= 86_400_000 && (
                    <tspan x={x} dy="1.2em">
                      {new Date(payload.value).toLocaleTimeString(undefined, {
                        hour: '2-digit',
                        minute: '2-digit',
                        hourCycle: 'h23',
                      })}
                    </tspan>
                  )}
                </text>
              );
            }}
            tickLine={false}
            axisLine={false}
            minTickGap={20}
            height={40}
          />
          <YAxis
            type="number"
            dataKey="lane"
            domain={[-0.5, states.length - 0.5]}
            ticks={states.map((_, index) => index)}
            tickFormatter={(index: number) => healthLabels[states[index]]}
            reversed
            tickLine={false}
            axisLine={false}
            width={axisWidth}
          />
          <ChartTooltip
            cursor={false}
            content={({ active, payload }) => {
              const point = payload?.[0]?.payload as Point | undefined;
              if (!point?.entry) return null;
              return (
                <ChartTooltipContent
                  active={active}
                  hideLabel
                  className="w-72 max-w-[calc(100vw-4rem)]"
                  payload={[
                    { graphicalItemId: 'health', name: 'health', dataKey: 'health', value: point.lane, payload: point },
                  ]}
                  formatter={() => <HealthObservationDetails entry={point.entry} />}
                />
              );
            }}
          />
          <Scatter data={points} isAnimationActive={false} shape={pointShape} activeShape={false} />
        </ScatterChart>
      </ChartContainer>
    );
  }, [now, start, states, points]);
  if (!selected) return null;

  return (
    <div ref={chartRef}>
      <PointSelectionContext.Provider
        value={{ selectedIndex, hasSelection, onChoose: (index) => setSelectedKey(groups[index].keys[0]), move }}>
        {chart}
      </PointSelectionContext.Provider>
      {hasSelection && (
        <div
          role="status"
          aria-label="Selected health observation"
          aria-live="polite"
          aria-atomic="true"
          className="flex items-start justify-between gap-3 border-t bg-muted/20 px-4 py-3 text-xs">
          <HealthObservationDetails entry={selected} />
          <Button
            type="button"
            variant="ghost"
            size="icon-xs"
            aria-label="Clear observation selection"
            onClick={() => {
              chartRef.current?.querySelector<SVGGElement>(`[data-health-point="${selectedIndex}"]`)?.focus();
              setSelectedKey(undefined);
            }}>
            <X className="size-3.5" aria-hidden="true" />
          </Button>
        </div>
      )}
    </div>
  );
}

function HealthObservationPoint({ cx, cy, payload: point }: { cx?: number; cy?: number; payload?: Point }) {
  const selection = useContext(PointSelectionContext);
  if (!point || !selection) return null;
  const { selectedIndex, hasSelection, onChoose, move } = selection;
  const choose = () => onChoose(point.index);
  return (
    <g
      role="button"
      tabIndex={point.index === selectedIndex ? 0 : -1}
      aria-pressed={hasSelection && point.index === selectedIndex}
      aria-label={`${healthLabels[point.entry.health.status]} at ${healthTime(point.timestamp)}`}
      data-health-point={point.index}
      className={`group/health-point cursor-pointer outline-none ${healthTone(point.entry.health.status)}`}
      onFocus={choose}
      onClick={choose}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          choose();
        } else move(event, point.index);
      }}>
      <circle cx={cx} cy={cy} r={10} fill="transparent" />
      <circle
        cx={cx}
        cy={cy}
        r={6.5}
        fill="none"
        stroke="currentColor"
        strokeWidth={1}
        className="opacity-0 group-focus-visible/health-point:opacity-100"
      />
      <circle cx={cx} cy={cy} r={4.5} fill="currentColor" />
    </g>
  );
}

function HealthObservationDetails({ entry: selected }: { entry: ContainerHealthTransitionView }) {
  return (
    <div className="min-w-0 space-y-1.5 text-xs">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
        <HealthLabel status={selected.health.status} />
        <time
          className="tabular-nums text-muted-foreground"
          dateTime={new Date(selected.observedAt * 1000).toISOString()}>
          {healthTime(selected.observedAt * 1000)}
        </time>
        <span className="text-muted-foreground">Container: {selected.state}</span>
      </div>
      {selected.sampleCount > 1 && (
        <p className="text-muted-foreground">
          {selected.sampleCount} observations with this state from {healthTime(selected.firstObservedAt * 1000)}. The
          dot marks the latest observation in this group, not continuous coverage.
        </p>
      )}
      {(selected.oomKilled || (selected.exitCode != null && selected.state.toLowerCase() !== 'running')) && (
        <div className="flex flex-wrap gap-3">
          {selected.oomKilled && <span className="text-destructive">Docker reported OOM killed</span>}
          {selected.exitCode != null && selected.state.toLowerCase() !== 'running' && (
            <span>Exit code {selected.exitCode}</span>
          )}
        </div>
      )}
    </div>
  );
}
