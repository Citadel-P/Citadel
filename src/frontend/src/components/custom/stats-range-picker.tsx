import { ChartNoAxesCombined } from 'lucide-react';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { STATS_PRESETS } from '@/lib/stats-range';
import type { StatsRangeState } from '@/hooks/useStatsRange';

export function StatsRangePicker({ range }: { range: StatsRangeState }) {
  const value = range.selection.kind === 'relative' ? String(range.selection.minutes) : range.selection.kind;
  return (
    <div className="flex items-center gap-2">
      <div className="hidden items-center gap-1.5 text-xs font-medium sm:flex">
        <ChartNoAxesCombined className="size-3.5 text-muted-foreground" />
        History
      </div>
      <Select
        value={value}
        onValueChange={(next) =>
          range.setSelection(
            next === 'today' || next === 'yesterday' ? { kind: next } : { kind: 'relative', minutes: Number(next) },
          )
        }>
        <SelectTrigger aria-label="Time range" className="h-8 w-36 bg-background text-xs shadow-none">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {STATS_PRESETS.map(({ minutes, label }) => (
            <SelectItem key={minutes} value={String(minutes)}>
              {label}
            </SelectItem>
          ))}
          <SelectItem value="today">Today</SelectItem>
          <SelectItem value="yesterday">Yesterday</SelectItem>
        </SelectContent>
      </Select>
    </div>
  );
}
