import { createContext, useEffect, useState, type ReactNode } from 'react';
import { resolveStatsRange, statsBucketSeconds } from '@/lib/stats-range';
export const RangeClock = createContext(0);
export function DashboardRangeClock({
  minutes,
  live,
  children,
}: {
  minutes: number;
  live: boolean;
  children: ReactNode;
}) {
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  useEffect(() => {
    if (!live) return;
    const seconds = statsBucketSeconds(resolveStatsRange({ kind: 'relative', minutes }));
    const timer = window.setInterval(() => setNow(Math.floor(Date.now() / 1000)), seconds * 1000);
    return () => window.clearInterval(timer);
  }, [live, minutes]);
  return <RangeClock.Provider value={now}>{children}</RangeClock.Provider>;
}
