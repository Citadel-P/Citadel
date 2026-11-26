import { useState, useCallback } from 'react';
import { Timestamp } from '@/components/custom/timestamp';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { Pause, Play, RefreshCw, Tags } from 'lucide-react';
import { ClearScopeContext } from './tile';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { RangeField } from './display-controls';
import { ScopeField } from './widget-settings';
import { DashboardGrid } from './grid';
import type { DashboardConfiguration } from '@/api/generated/api.types';

export function DashboardView({
  boardId,
  config,
  paused,
  setPaused,
  editing = false,
}: {
  boardId?: string;
  editing?: boolean;
  config: DashboardConfiguration;
  paused: boolean;
  setPaused: (value: boolean) => void;
}) {
  const [scopeOverride, setScopeOverride] = useState<{ base: string; tags: string[] }>();
  const [rangeOverride, setRangeOverride] = useState<{ base: number; range: number }>();
  const base = JSON.stringify(config.tags);
  const tags = scopeOverride?.base === base ? scopeOverride.tags : config.tags;
  const range = rangeOverride?.base === config.rangeMinutes ? rangeOverride.range : config.rangeMinutes;
  const setTags = (tags: string[]) => setScopeOverride({ base, tags });
  const setRange = (range: number) => setRangeOverride({ base: config.rangeMinutes, range });
  const [generation, refresh] = useState(0);
  const [refreshing, setRefreshing] = useState(false);
  const [pausedAt, setPausedAt] = useState<number>();
  const format = useProfileDateTimeFormatter();
  const settled = useCallback(() => {
    setRefreshing(false);
    setPausedAt(Date.now());
  }, []);
  return (
    <ClearScopeContext.Provider value={() => setTags([])}>
      <div className="mb-3 flex flex-wrap items-center gap-2 rounded-lg border bg-card p-2">
        <Popover>
          <PopoverTrigger asChild>
            <Button size="sm" variant="outline">
              <Tags />
              Tag filter{tags.length ? ' · ' + tags.length : ''}
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-80">
            <ScopeField value={tags} change={setTags} />
          </PopoverContent>
        </Popover>
        {config.widgets.some((w) => w.settings.kind === 'Metrics') && <RangeField value={range} change={setRange} />}
        <div className="ml-auto flex items-center gap-1">
          <span className="mr-2 text-xs text-muted-foreground" role="status">
            {refreshing ? (
              'Refreshing…'
            ) : paused ? (
              <>Paused {pausedAt && <Timestamp value={pausedAt} formatDateTime={format} />}</>
            ) : (
              'Auto'
            )}
          </span>
          <Button
            size="sm"
            variant="outline"
            onClick={() => {
              if (!paused) setPausedAt(Date.now());
              setPaused(!paused);
            }}>
            {paused ? <Play /> : <Pause />}
            {paused ? 'Resume' : 'Pause'}
          </Button>
          <Button
            variant="outline"
            disabled={refreshing || editing || !config.widgets.length}
            onClick={() => {
              setRefreshing(true);
              refresh((g) => g + 1);
            }}>
            <RefreshCw />
            Refresh
          </Button>
        </div>
      </div>
      <DashboardGrid
        boardId={boardId}
        refresh={generation}
        onSettled={settled}
        configuration={{ ...config, tags, rangeMinutes: range }}
        paused={paused || editing}
        inactive={editing}
      />
    </ClearScopeContext.Provider>
  );
}
