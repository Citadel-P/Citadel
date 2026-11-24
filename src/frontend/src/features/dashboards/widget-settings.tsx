import { useState, type ReactNode } from 'react';
import {
  DashboardChartColor,
  DashboardChartStyle,
  DashboardMetric,
  DashboardWorkloadKind,
  type DashboardWidget,
  type DashboardMetricSource,
  type DashboardWidgetSettings,
} from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { ResourceSelectorField } from '@/components/custom/common';
import { SearchField } from '@/components/custom/search-field';
import { useRead } from '@/lib/hooks';
import { STATS_PRESETS } from '@/lib/stats-range';
import { TagNameFilter, useResolvedTagFilter } from '@/features/tags/components';

export function Choice<T extends string>({
  label,
  value,
  options,
  change,
}: {
  label: string;
  value: T;
  options: readonly T[];
  change: (v: T) => void;
}) {
  return (
    <label className="grid gap-1 text-sm">
      {label}
      <select
        className="h-9 rounded-md border bg-background px-2"
        value={value}
        onChange={(e) => change(e.target.value as T)}>
        {options.map((v) => (
          <option key={v}>{v}</option>
        ))}
      </select>
    </label>
  );
}
export function RangeField({ value, change }: { value: number; change: (v: number) => void }) {
  return (
    <label className="flex items-center gap-2 text-sm">
      History range
      <select
        className="h-9 rounded-md border bg-background px-2"
        value={value}
        onChange={(e) => change(Number(e.target.value))}>
        {STATS_PRESETS.map((p) => (
          <option key={p.minutes} value={p.minutes}>
            {p.label}
          </option>
        ))}
      </select>
    </label>
  );
}
export function ScopeField({ value, change }: { value: string[]; change: (v: string[]) => void }) {
  const { selection } = useResolvedTagFilter(value);
  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="text-sm">Match all tags</span>
      <TagNameFilter selection={selection} setSelectedTagNames={change} />
    </div>
  );
}
export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="grid gap-1 text-sm">
      {label}
      {children}
    </label>
  );
}
export function MetricSourcePicker({
  source,
  change,
}: {
  source: DashboardMetricSource;
  change: (s: DashboardMetricSource) => void;
}) {
  const [search, setSearch] = useState('');
  const candidates = useRead('listDashboardSources', { query: { kind: source.kind, search, limit: 25 } });
  const items = candidates.isFetchedAfterMount && !candidates.isError ? (candidates.data?.data ?? []) : [];
  const selected = items.find((i) => i.id === source.id);
  return (
    <>
      <Choice
        label="Resource type"
        value={source.kind}
        options={['Platform', 'Deployment', 'Stack', 'SwarmService', 'Container']}
        change={(kind) => {
          setSearch('');
          change(kind === 'Container' ? { kind, id: '', platformId: '' } : { kind, id: '' });
        }}
      />
      <SearchField onSearch={setSearch} placeholder="Find a resource by name…" />
      <ResourceSelectorField
        aria-label="Metric resource"
        items={items}
        selected={selected}
        placeholder={source.id ? 'Keep selected resource' : 'Choose resource'}
        allowClear={false}
        onSelect={(item) => {
          if (item)
            change(
              source.kind === 'Container'
                ? { kind: 'Container', id: item.id, platformId: item.platformId ?? '' }
                : { kind: source.kind, id: item.id },
            );
        }}
      />
      {candidates.isError && <p role="alert">Resource search unavailable.</p>}
    </>
  );
}
export function WidgetSettings({
  widget,
  change,
  place,
}: {
  widget: DashboardWidget;
  change: (w: DashboardWidget) => void;
  place: (layout: DashboardWidget['layout']) => void;
}) {
  const s = widget.settings;
  const settings = (value: DashboardWidgetSettings) => change({ ...widget, settings: value });
  return (
    <div className="space-y-3">
      <Field label="Widget title">
        <Input value={widget.title} maxLength={128} onChange={(e) => change({ ...widget, title: e.target.value })} />
      </Field>
      {'limit' in s && (
        <Field label="Rows">
          <Input
            type="number"
            min={1}
            max={10}
            value={s.limit}
            onChange={(e) => settings({ ...s, limit: Number(e.target.value) })}
          />
        </Field>
      )}
      {s.kind === 'Workloads' && (
        <Choice
          label="Workload type"
          value={s.resourceKind}
          options={Object.values(DashboardWorkloadKind)}
          change={(resourceKind) => settings({ ...s, resourceKind })}
        />
      )}
      {s.kind === 'Metrics' && (
        <>
          <MetricSourcePicker source={s.source} change={(source) => settings({ ...s, source })} />
          <Choice
            label="Metric"
            value={s.metric}
            options={Object.values(DashboardMetric)}
            change={(metric) => settings({ ...s, metric })}
          />
          <Choice
            label="Chart"
            value={s.presentation}
            options={Object.values(DashboardChartStyle)}
            change={(presentation) => settings({ ...s, presentation })}
          />
          <Choice
            label="Color"
            value={s.color}
            options={Object.values(DashboardChartColor)}
            change={(color) => settings({ ...s, color })}
          />
          <label className="flex gap-2">
            <input type="checkbox" checked={s.legend} onChange={(e) => settings({ ...s, legend: e.target.checked })} />
            Show legend
          </label>
        </>
      )}
      {s.kind === 'Notes' && (
        <>
          <Field label="Note">
            <Textarea value={s.text} maxLength={2048} onChange={(e) => settings({ ...s, text: e.target.value })} />
          </Field>
          {s.links.map((link, i) => (
            <div key={i} className="space-y-2 border-t pt-2">
              {(['label', 'href'] as const).map((k) => (
                <Field key={k} label={k === 'label' ? 'Link label' : 'Link address'}>
                  <Input
                    value={link[k]}
                    onChange={(e) =>
                      settings({ ...s, links: s.links.map((l, j) => (j === i ? { ...l, [k]: e.target.value } : l)) })
                    }
                  />
                </Field>
              ))}
              <Button
                size="sm"
                variant="outline"
                onClick={() => settings({ ...s, links: s.links.filter((_, j) => i !== j) })}>
                Remove link
              </Button>
            </div>
          ))}
          <Button
            size="sm"
            variant="outline"
            disabled={s.links.length >= 8}
            onClick={() => settings({ ...s, links: [...s.links, { label: '', href: '' }] })}>
            Add link
          </Button>
        </>
      )}
      <div className="grid grid-cols-2 gap-2">
        {(['x', 'y', 'w', 'h'] as const).map((k, i) => (
          <Field key={k} label={['Column', 'Row', 'Width', 'Height'][i]}>
            <Input
              type="number"
              value={widget.layout[k]}
              min={i < 2 ? 0 : 2}
              max={[10, 512, 12, 24][i]}
              onChange={(e) => {
                if (e.target.value) place({ ...widget.layout, [k]: Number(e.target.value) });
              }}
            />
          </Field>
        ))}
      </div>
    </div>
  );
}
