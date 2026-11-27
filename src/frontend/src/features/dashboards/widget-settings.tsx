import type { DashboardRequirements } from './requirements';
import { metricColors } from './catalogue';
import { WidgetThumbnail } from './widget-thumbnail';
import { sizePresets } from './size-presets';
import { useState, type ReactNode } from 'react';
import {
  DashboardFleetKind,
  DashboardStateKind,
  DashboardSourceKind,
  DashboardAttentionTab,
  DashboardFleetMeasure,
  DashboardChartColor,
  DashboardChartStyle,
  DashboardMetric,
  type DashboardWidget,
  type DashboardMetricSource,
  type DashboardWidgetSettings,
} from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { MarkdownDescriptionEditor } from '@/components/custom/markdown-description';
import { ResourceSelectorField } from '@/components/custom/common';
import { useDebounce } from '@/hooks/useDebounce';
import { CitadelIcons } from '@/lib/icons';
import { useRead } from '@/lib/hooks';
import { ResourceMultiFilter } from '@/components/custom/resource-filter';
import { useResolvedTagFilter } from '@/features/tags/components';
import { Choice, chartPalette } from './display-controls';

import { ChartArea, ChartLine, Blend } from 'lucide-react';
import { SegmentedControl } from '@/components/ui/segmented-control';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { Switch } from '@/components/ui/switch';

export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="grid gap-1 text-sm">
      {label}
      {children}
    </label>
  );
}
type SourceLookup = { batches: DashboardRequirements['batches']; tags: string[] };
export function MetricSourcePicker({
  source,
  change,
  fixedKind = false,
  label = 'Target resource',
  excludes,
  lookup,
}: {
  fixedKind?: boolean;
  label?: string;
  excludes?: DashboardMetricSource[];
  lookup?: SourceLookup;
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
  const batch = lookup?.batches.find((b) => b.kind === source.kind && b.ids.includes(source.id));
  const ids = batch?.ids ?? [source.id];
  // The editor observes the canvas batch. Only standalone pickers own a separate pin lookup.
  const pinned = useRead(
    'listDashboardSources',
    { query: { kind: source.kind, tags: lookup?.tags ?? [], ids: ids.join(','), limit: ids.length } },
    { enabled: !lookup && !!source.id, staleTime: 15000 },
  );
  const items = !open || candidates.isError ? [] : (candidates.data?.data ?? []);
  const selected =
    items.find((i) => i.id === source.id) ??
    (!pinned.isError && (!lookup || batch) ? pinned.data?.data.find((i) => i.id === source.id) : undefined);
  return (
    <>
      {fixedKind ? (
        <span className="text-xs text-muted-foreground">{source.kind}</span>
      ) : (
        <Choice
          label="Resource type"
          value={source.kind}
          options={Object.values(DashboardSourceKind)}
          icons={CitadelIcons}
          labels={{ SwarmService: 'Swarm service' }}
          change={(kind) => {
            setSearch('');
            change({ kind, id: '' });
          }}
        />
      )}

      <div className="grid gap-1 text-sm">
        <span>{label}</span>
        <ResourceSelectorField
          key={source.kind}
          className="max-w-none"
          aria-label={label === 'Target resource' ? 'Metric resource' : label}
          isItemDisabled={(item) => !!excludes?.some((p) => p.kind === source.kind && p.id === item.id)}
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
            if (item) change({ kind: source.kind, id: item.id });
          }}
        />
      </div>
    </>
  );
}
function Pins({
  pins,
  change,
  lookup,
}: {
  lookup: SourceLookup;
  pins: DashboardMetricSource[];
  change: (pins: DashboardMetricSource[]) => void;
}) {
  const [choice, setChoice] = useState<DashboardMetricSource>({ kind: DashboardSourceKind.Platform, id: '' });
  const [adding, setAdding] = useState(false);
  return (
    <div className="space-y-3">
      {pins.map((p, i) => (
        <div key={p.kind + p.id} className="space-y-1 border-b py-2">
          <MetricSourcePicker
            lookup={lookup}
            source={p}
            fixedKind
            label={'Pinned resource ' + (i + 1)}
            excludes={pins.filter((_, j) => j !== i)}
            change={(v) => {
              if (v.id && !pins.some((p, j) => j !== i && p.kind === v.kind && p.id === v.id))
                change(pins.map((old, j) => (j === i ? v : old)));
            }}
          />
          <div className="flex gap-1">
            <Button
              size="sm"
              variant="ghost"
              disabled={i === 0}
              onClick={() => {
                const next = [...pins];
                [next[i - 1], next[i]] = [next[i], next[i - 1]];
                change(next);
              }}>
              Move up
            </Button>
            <Button size="sm" variant="ghost" onClick={() => change(pins.filter((_, j) => j !== i))}>
              Remove
            </Button>
          </div>
        </div>
      ))}
      {pins.length < 8 &&
        (adding || pins.length === 0 ? (
          <MetricSourcePicker
            label="Add resource to watchlist"
            excludes={pins}
            lookup={lookup}
            source={choice}
            change={(p) => {
              if (!p.id) {
                setChoice(p);
                return;
              }
              if (!pins.some((v) => v.kind === p.kind && v.id === p.id)) change([...pins, p]);
              setChoice({ ...p, id: '' });
              setAdding(false);
            }}
          />
        ) : (
          <Button type="button" variant="outline" onClick={() => setAdding(true)}>
            Add resource
          </Button>
        ))}
    </div>
  );
}
export function WidgetSettings({
  widget,
  batches,
  tags,
  change,
  place,
}: {
  widget: DashboardWidget;
  batches: SourceLookup['batches'];
  tags: string[];
  change: (w: DashboardWidget) => void;
  place: (layout: DashboardWidget['layout'], limit?: number) => void;
}) {
  const { selectedTagNames } = useResolvedTagFilter(tags);
  const lookup = { batches, tags: selectedTagNames };
  const s = widget.settings;
  const settings = (value: DashboardWidgetSettings) => change({ ...widget, settings: value });
  return (
    <div className="space-y-4">
      <div className="dashboard-inspector-identity">
        <WidgetThumbnail kind={s.kind} color={s.kind === 'Metrics' ? chartPalette[s.color] : undefined} />
        <Field label="Widget title">
          <Input value={widget.title} maxLength={128} onChange={(e) => change({ ...widget, title: e.target.value })} />
        </Field>
      </div>
      {s.kind === 'ResourceState' && (
        <Choice
          label="Resource type"
          value={s.resourceKind}
          options={Object.values(DashboardStateKind)}
          icons={CitadelIcons}
          change={(resourceKind) => settings({ ...s, resourceKind })}
        />
      )}
      {s.kind === 'NeedsAttention' && (
        <>
          <div className="grid gap-1.5 text-sm">
            <span>Default tab</span>
            <SegmentedControl
              aria-label="Default tab"
              value={s.tab}
              options={Object.values(DashboardAttentionTab).map((value) => ({ value, label: value }))}
              onValueChange={(tab) => settings({ ...s, tab: tab as DashboardAttentionTab })}
            />
          </div>
          <ResourceMultiFilter
            label="Resource type"
            icon={CitadelIcons.Dashboard}
            options={Object.values(DashboardFleetKind).map((value) => {
              const Icon = CitadelIcons[value];
              return { value, label: value, icon: <Icon /> };
            })}
            selected={s.kinds}
            onChange={(kinds) => settings({ ...s, kinds: kinds as DashboardFleetKind[] })}
          />
          <div className="grid gap-1.5 text-sm">
            <span>Visible rows</span>
            <SegmentedControl
              aria-label="Visible rows"
              value={String(s.limit)}
              options={['3', '5', '10'].map((value) => ({ value, label: value }))}
              onValueChange={(v) => settings({ ...s, limit: Number(v) })}
            />
          </div>
        </>
      )}
      {s.kind === 'FleetSummary' && (
        <ResourceMultiFilter
          label="Measure"
          icon={CitadelIcons.Dashboard}
          selected={s.measures}
          options={Object.values(DashboardFleetMeasure).map((value) => ({ value, label: value }))}
          onChange={(v) => settings({ ...s, measures: v as DashboardFleetMeasure[] })}
        />
      )}
      {s.kind === 'Watchlist' && <Pins lookup={lookup} pins={s.pins} change={(pins) => settings({ ...s, pins })} />}
      {(s.kind === 'Backups' || s.kind === 'RecentActivity') && (
        <Choice
          label="Activity preset"
          value={s.kind}
          labels={{ RecentActivity: 'General', Backups: 'Backup activity' }}
          options={['RecentActivity', 'Backups']}
          change={(kind) => settings({ kind, limit: s.limit })}
        />
      )}
      {'limit' in s && s.kind !== 'NeedsAttention' && (
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
      {s.kind === 'Metrics' && (
        <>
          <h3 className="font-medium">Data</h3>
          <p className="text-xs text-muted-foreground">Uses the dashboard chart range.</p>
          <MetricSourcePicker lookup={lookup} source={s.source} change={(source) => settings({ ...s, source })} />
          <Choice
            label="Metric"
            value={s.metric}
            options={Object.values(DashboardMetric)}
            labels={{ Cpu: 'CPU', DiskIo: 'Disk I/O' }}
            change={(metric) => settings({ ...s, metric })}
          />
          <h3 className="border-t pt-3 font-medium">Appearance</h3>
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
            <div className="flex min-h-6 items-center justify-between">
              Color
              {s.color !== metricColors[s.metric] && (
                <Button variant="link" size="xs" onClick={() => settings({ ...s, color: metricColors[s.metric] })}>
                  Use suggested {metricColors[s.metric].toLowerCase()}
                </Button>
              )}
            </div>
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
          <div className="grid gap-1 text-sm">
            <label htmlFor="dashboard-note">Note</label>
            <MarkdownDescriptionEditor
              id="dashboard-note"
              value={s.text}
              maxLength={2048}
              onChange={(e) => settings({ ...s, text: e.target.value })}
            />
          </div>
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
        </>
      )}
      <div className="grid gap-2 border-t pt-3">
        <span className="text-sm font-medium">Layout</span>
        <div className="flex flex-wrap gap-1">
          {sizePresets[s.kind].map(([w, h, limit], i) => (
            <Button
              size="sm"
              variant="outline"
              key={i}
              aria-pressed={
                widget.layout.w === w &&
                widget.layout.h === h &&
                (limit === undefined || ('limit' in s && s.limit === limit))
              }
              onClick={() => place({ ...widget.layout, x: Math.min(widget.layout.x, 12 - w), w, h }, limit)}>
              {['Compact', 'Standard', 'Expanded'][i]}
              {limit !== undefined && <span className="text-muted-foreground"> · {limit} rows</span>}
            </Button>
          ))}
        </div>
      </div>
      <details className="border-t pt-4">
        <summary className="mb-3 cursor-pointer font-medium">
          Advanced position &amp; size · {widget.layout.w} × {widget.layout.h}
        </summary>
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
      </details>
    </div>
  );
}
