import { useId, useState } from 'react';
import { History, Radio } from 'lucide-react';
import type { ContainerHealthTransitionView } from '@/api/generated/api.types';
import { StatsRangePicker } from '@/components/custom/stats-range-picker';
import { resolveStatsRange, type StatsRange } from '@/lib/stats-range';
import { ContainerHealthEvents } from './container-health-events';

export function ContainerHealthHistory({
  entries,
  now,
  isConnected = true,
  isLoading = false,
  disabledReason,
}: {
  entries: ContainerHealthTransitionView[];
  now: number;
  isConnected?: boolean;
  isLoading?: boolean;
  disabledReason?: string;
}) {
  const captionId = useId();
  const titleId = useId();
  const [selection, setSelection] = useState<StatsRange>({ kind: 'relative', minutes: 60 });
  const bounds = resolveStatsRange(selection, Math.floor(now / 1000));
  const records = entries
    .filter((entry) => entry.observedAt >= bounds.from && entry.observedAt <= bounds.to)
    .sort((a, b) => b.observedAt - a.observedAt);
  const count = records.reduce((sum, entry) => sum + (entry.sampleCount ?? 1), 0);

  if (disabledReason) {
    return (
      <figure
        aria-labelledby={titleId}
        aria-describedby={captionId}
        aria-disabled="true"
        className="overflow-hidden rounded-lg border bg-muted/20 text-muted-foreground">
        <div className="border-b px-4 py-3">
          <h2 id={titleId} className="text-sm font-semibold">
            Health history
          </h2>
        </div>
        <figcaption id={captionId} className="relative flex min-h-44 items-center justify-center px-4 py-8">
          <div
            aria-hidden="true"
            className="pointer-events-none absolute inset-x-6 inset-y-8 flex flex-col justify-between opacity-40">
            {[0, 1, 2].map((line) => (
              <div key={line} className="border-t border-dashed border-muted-foreground/30" />
            ))}
          </div>
          <span className="relative rounded-md border bg-background px-4 py-3 text-center text-sm">
            {disabledReason}
          </span>
        </figcaption>
      </figure>
    );
  }

  return (
    <figure
      aria-labelledby={titleId}
      aria-describedby={captionId}
      className="overflow-hidden rounded-lg border bg-background">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b px-4 py-3">
        <div className="space-y-1">
          <div className="flex items-center gap-3">
            <h2 id={titleId} className="text-sm font-semibold">
              Health history
            </h2>
            <span className="inline-flex items-center gap-1 text-[11px] text-muted-foreground">
              <Radio className="size-3" aria-hidden="true" />
              {isConnected ? 'Live' : 'Reconnecting'}
            </span>
          </div>
          <p className="text-xs text-muted-foreground">
            <span>
              {isLoading
                ? 'Loading observations…'
                : `${count} recorded ${count === 1 ? 'observation' : 'observations'}`}
            </span>{' '}
            · Local time
          </p>
        </div>
        <StatsRangePicker range={{ selection, setSelection }} showLabel={false} />
      </div>
      {!isConnected && (
        <p role="status" className="border-b bg-muted/20 px-4 py-2 text-xs text-muted-foreground">
          {entries.length
            ? 'Connection interrupted. Saved observations remain available.'
            : 'Connection interrupted. Waiting to reconnect.'}
        </p>
      )}
      {isLoading ? (
        <p role="status" className="py-12 text-center text-sm text-muted-foreground">
          Loading history…
        </p>
      ) : records.length ? (
        <ContainerHealthEvents entries={records} now={bounds.to * 1000} start={bounds.from * 1000} />
      ) : (
        <div className="flex min-h-44 flex-col items-center justify-center gap-2 px-4 py-8 text-center text-muted-foreground">
          <History className="mb-1 size-5" aria-hidden="true" />
          <p className="text-sm">No observations in this time range.</p>
          <p className="text-xs">Try a longer range or wait for the next observation.</p>
        </div>
      )}
      <figcaption id={captionId} className="border-t bg-muted/10 px-4 py-2.5 text-xs text-muted-foreground">
        Hover for details · Select a point to inspect · Arrow keys to navigate
      </figcaption>
    </figure>
  );
}
