import { useCallback, useEffect, useMemo, useState } from 'react';
import { resolveStatsRange, statsRangeIsLive, statsRangeQuery, type StatsRange } from '@/lib/stats-range';

export function useStatsRange() {
  const [selection, saveSelection] = useState<StatsRange>({ kind: 'relative', minutes: 60 });
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  const setSelection = useCallback((next: StatsRange) => {
    setNow(Math.floor(Date.now() / 1000));
    saveSelection(next);
  }, []);
  const live = statsRangeIsLive(selection);
  useEffect(() => {
    if (!live) return;
    const tick = () => setNow(Math.floor(Date.now() / 1000));
    tick();
    const timer = window.setInterval(tick, 10_000);
    return () => window.clearInterval(timer);
  }, [live]);
  const bounds = resolveStatsRange(selection, now);
  const { hours, minutes, from, to } = statsRangeQuery(selection, bounds);
  // Stable query keys while the live window moves; realtime supplies new samples.
  const query = useMemo(() => ({ hours, minutes, from, to }), [hours, minutes, from, to]);
  return { selection, setSelection, bounds, query, live, hours: (bounds.to - bounds.from) / 3600 };
}
export type StatsRangeState = ReturnType<typeof useStatsRange>;
