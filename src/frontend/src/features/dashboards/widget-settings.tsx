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
import { useDebounce } from '@/hooks/useDebounce';
import { CitadelIcons } from '@/lib/icons';
import { useRead } from '@/lib/hooks';
import { Choice, chartPalette } from './display-controls';
import { TagNameFilter, useResolvedTagFilter } from '@/features/tags/components';

import { ChartArea, ChartLine, Blend } from 'lucide-react';
import { SegmentedControl } from '@/components/ui/segmented-control';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { Switch } from '@/components/ui/switch';

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
  const [open, setOpen] = useState(false);
  const term = useDebounce(search, 250);
  const candidates = useRead(
    'listDashboardSources',
    { query: { kind: source.kind, search: term, limit: 25 } },
    { enabled: open, staleTime: 15000 },
  );
  const pinned = useRead(
    'listDashboardSources',
    { query: { kind: source.kind, tags: [], ids: source.id, limit: 1 } },
    { enabled: !!source.id, staleTime: 15000 },
  );
  const items = candidates.isError ? [] : (candidates.data?.data ?? []);
  const selected = items.find((i) => i.id === source.id) ?? pinned.data?.data.find((i) => i.id === source.id);
  return (
    <>
      <Choice
        label="Resource type"
        value={source.kind}
        options={['Platform', 'Deployment', 'Stack', 'SwarmService', 'Container']}
        icons={CitadelIcons}
        labels={{ SwarmService: 'Swarm service' }}
        change={(kind) => {
          setSearch('');
          change(kind === 'Container' ? { kind, id: '', platformId: '' } : { kind, id: '' });
        }}
      />

      <div className="grid gap-1 text-sm">
        <span>Target resource</span>
        <ResourceSelectorField
          key={source.kind}
          aria-label="Metric resource"
          searchPlaceholder="Find a resource by name…"
          onOpenChange={setOpen}
          onSearchChange={setSearch}
          emptyMessage={
            candidates.isError
              ? 'Resource search unavailable.'
              : candidates.isFetching
                ? 'Searching…'
                : 'No matching resources.'
          }
          items={items}
          selected={selected}
          placeholder={
            source.id ? (pinned.isLoading ? 'Loading resource…' : 'Resource unavailable') : 'Choose resource'
          }
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
      </div>
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
    <div className="space-y-4">
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
          icons={CitadelIcons}
          labels={{ SwarmService: 'Swarm service' }}
          change={(resourceKind) => settings({ ...s, resourceKind })}
        />
      )}
      {s.kind === 'Metrics' && (
        <>
          <h3 className="border-t pt-5 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
            Data configuration
          </h3>
          <MetricSourcePicker source={s.source} change={(source) => settings({ ...s, source })} />
          <Choice
            label="Metric"
            value={s.metric}
            options={Object.values(DashboardMetric)}
            labels={{ Cpu: 'CPU', DiskIo: 'Disk I/O' }}
            change={(metric) => settings({ ...s, metric })}
          />
          <h3 className="border-t pt-5 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
            Appearance
          </h3>
          <div className="grid gap-1.5 text-sm">
            Chart style
            <SegmentedControl
              className="grid h-12 w-full grid-cols-3 p-1"
              aria-label="Chart style"
              value={s.presentation}
              onValueChange={(v) => settings({ ...s, presentation: v as DashboardChartStyle })}
              options={Object.values(DashboardChartStyle).map((v) => {
                const Icon = { Line: ChartLine, Area: ChartArea, Gradient: Blend }[v];
                return {
                  value: v,
                  label: (
                    <span className="flex flex-col items-center gap-1">
                      <Icon aria-hidden="true" className="size-3.5" />
                      {v}
                    </span>
                  ),
                };
              })}
            />
          </div>
          <div className="grid gap-1.5 text-sm">
            Color
            <RadioGroup
              aria-label="Chart color"
              value={s.color}
              onValueChange={(v) => settings({ ...s, color: v as DashboardChartColor })}
              className="flex gap-2 p-1">
              {Object.values(DashboardChartColor).map((color) => (
                <RadioGroupItem
                  key={color}
                  aria-label={color}
                  title={color}
                  value={color}
                  style={{ backgroundColor: chartPalette[color] }}
                  className="size-6 border-transparent text-white data-[state=checked]:ring-2 data-[state=checked]:ring-ring data-[state=checked]:ring-offset-2 ring-offset-background"
                />
              ))}
            </RadioGroup>
          </div>
          <label htmlFor="dashboard-chart-legend" className="flex items-center justify-between gap-2 text-sm">
            Show legend
            <Switch
              id="dashboard-chart-legend"
              checked={s.legend}
              onCheckedChange={(legend) => settings({ ...s, legend })}
            />
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
      <h3 className="border-t pt-5 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">Layout</h3>
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
