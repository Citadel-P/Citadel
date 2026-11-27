import { TagNameFilter, useResolvedTagFilter } from '@/features/tags/components';
import type { ComponentType } from 'react';
import { StatsRangePicker } from '@/components/custom/stats-range-picker';
import { Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from '@/components/ui/select';
export const chartPalette = {
  Blue: 'var(--color-blue-500)',
  Green: 'var(--color-emerald-500)',
  Amber: 'var(--color-amber-500)',
  Violet: 'var(--color-violet-500)',
  Cyan: 'var(--color-cyan-500)',
};
export function Choice<T extends string>({
  label,
  value,
  options,
  icons,
  labels,
  change,
}: {
  label: string;
  value: T;
  options: readonly T[];
  icons?: Partial<Record<T, ComponentType<{ className?: string }>>>;
  labels?: Partial<Record<T, string>>;
  change: (v: T) => void;
}) {
  return (
    <div className="grid gap-1 text-sm">
      {label}
      <Select value={value} onValueChange={(v) => change(v as T)}>
        <SelectTrigger aria-label={label} className="w-full">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {options.map((v) => {
            const Icon: ComponentType<{ className?: string }> | undefined = icons?.[v];
            return (
              <SelectItem key={v} value={v}>
                {Icon && <Icon className="size-4 text-muted-foreground" />}
                {labels?.[v] ?? v}
              </SelectItem>
            );
          })}
        </SelectContent>
      </Select>
    </div>
  );
}
export function RangeField({ value, change }: { value: number; change: (v: number) => void }) {
  return (
    <div className="flex items-center gap-2 text-sm">
      Chart range
      <StatsRangePicker
        showLabel={false}
        includeCalendarDays={false}
        range={{
          selection: { kind: 'relative', minutes: value },
          setSelection: (next) => {
            if (next.kind === 'relative') change(next.minutes);
          },
        }}
      />
    </div>
  );
}

export function ScopeField({ value, change }: { value: string[]; change: (tags: string[]) => void }) {
  const { selection } = useResolvedTagFilter(value);
  return <TagNameFilter selection={selection} setSelectedTagNames={change} />;
}
