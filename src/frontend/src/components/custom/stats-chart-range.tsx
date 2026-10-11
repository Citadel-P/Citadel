import { createContext, useContext, type ReactNode } from 'react';
import type { StatsBounds } from '@/lib/stats-range';
import type { StatsRangeState } from '@/hooks/useStatsRange';
import { AlertMessage } from './alert-message';
import { StatsRangePicker } from './stats-range-picker';
import { TabActions } from './tab-actions';

const StatsBoundsContext = createContext<StatsBounds | undefined>(undefined);
export const useStatsChartBounds = () => useContext(StatsBoundsContext);
export function StatsChartRange({
  range,
  children,
  error,
}: {
  range: StatsRangeState;
  children: ReactNode;
  error?: unknown;
}) {
  return (
    <StatsBoundsContext.Provider value={range.bounds}>
      <TabActions>
        <StatsRangePicker range={range} />
      </TabActions>
      {error ? (
        <AlertMessage type="error" title="Metrics unavailable">
          Could not load metrics for this range. Try another range or refresh the page.
        </AlertMessage>
      ) : (
        children
      )}
    </StatsBoundsContext.Provider>
  );
}
