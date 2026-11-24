import { useState, useCallback } from 'react';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { RangeField, ScopeField } from './widget-settings';
import { DashboardGrid } from './grid';
import type { DashboardConfiguration } from '@/api/generated/api.types';

export function DashboardView({
  config,
  paused,
  setPaused,
  editing = false,
}: {
  editing?: boolean;
  config: DashboardConfiguration;
  paused: boolean;
  setPaused: (value: boolean) => void;
}) {
  const [tags, setTags] = useState(config.tags);
  const [range, setRange] = useState(config.rangeMinutes);
  const [generation, refresh] = useState(0);
  const [refreshing, setRefreshing] = useState(false);
  const settled = useCallback(() => setRefreshing(false), []);
  return (
    <>
      <div className="flex flex-wrap items-center gap-2">
        <Popover>
          <PopoverTrigger asChild>
            <Button variant="outline">Tag filter{tags.length ? ' · ' + tags.length : ''}</Button>
          </PopoverTrigger>
          <PopoverContent className="w-80">
            <ScopeField value={tags} change={setTags} />
          </PopoverContent>
        </Popover>
        {config.widgets.some((w) => w.settings.kind === 'Metrics') && <RangeField value={range} change={setRange} />}
        <Button variant="outline" onClick={() => setPaused(!paused)}>
          {paused ? 'Resume' : 'Pause'}
        </Button>
        <Button
          variant="outline"
          disabled={refreshing || editing || !config.widgets.length}
          onClick={() => {
            setRefreshing(true);
            refresh((g) => g + 1);
          }}>
          Refresh
        </Button>
      </div>
      <DashboardGrid
        refresh={generation}
        onSettled={settled}
        configuration={{ ...config, tags, rangeMinutes: range }}
        paused={paused || editing}
      />
    </>
  );
}
