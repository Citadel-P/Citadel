import { useState } from 'react';
import { ClearScopeContext } from './tile';
import { RangeField, ScopeField } from './display-controls';
import { DashboardGrid } from './grid';
import type { DashboardConfiguration } from '@/api/generated/api.types';

export function DashboardView({
  config,
  editing = false,
  onAddWidget,
}: {
  editing?: boolean;
  config: DashboardConfiguration;
  onAddWidget?: () => void;
}) {
  const [scopeOverride, setScopeOverride] = useState<{ base: string; tags: string[] }>();
  const [rangeOverride, setRangeOverride] = useState<{ base: number; range: number }>();
  const base = JSON.stringify(config.tags);
  const tags = scopeOverride?.base === base ? scopeOverride.tags : config.tags;
  const range = rangeOverride?.base === config.rangeMinutes ? rangeOverride.range : config.rangeMinutes;
  const setTags = (tags: string[]) => setScopeOverride({ base, tags });
  const setRange = (range: number) => setRangeOverride({ base: config.rangeMinutes, range });
  return (
    <ClearScopeContext.Provider value={() => setTags([])}>
      {config.widgets.length > 0 && (
        <div className="dashboard-view-controls mb-3 flex flex-wrap items-center justify-end gap-2 rounded-lg border bg-card p-2">
          {config.widgets.some((w) => w.settings.kind === 'Metrics') && <RangeField value={range} change={setRange} />}
          <ScopeField value={tags} change={setTags} />
        </div>
      )}
      <DashboardGrid
        configuration={{ ...config, tags, rangeMinutes: range }}
        inactive={editing}
        onAddWidget={onAddWidget}
      />
    </ClearScopeContext.Provider>
  );
}
