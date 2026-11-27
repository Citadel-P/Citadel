import { useDashboardRead } from './use-dashboard-read';
import { MarkdownDescription } from '@/components/custom/markdown-description';
import { DashboardFleetMeasure } from '@/api/generated/api.types';
import { monitoringLink } from '@/lib/monitoring-link';
import { Link } from 'react-router';
import { lazy, Suspense, useContext, type ReactNode } from 'react';
import type { DashboardMetricSource, DashboardSourceView } from '@/api/generated/api.types';
import { Skeleton } from '@/components/ui/skeleton';
import { Button } from '@/components/ui/button';
import { FleetTile } from './fleet';
import { MaintenanceTiles } from './maintenance';
import { StateIndicator } from '@/components/custom/state-indicator';
import { CitadelIcons } from '@/lib/icons';
import { HealthCard } from '@/components/custom/health-card';
import { Activity } from 'lucide-react';
import { ResourceMenu, WorkloadDetail } from './resource-rows';
import { validLink } from './catalogue';
import type { MetricTile, DashboardRequirements } from './requirements';
import { sourcePath } from './source-path';
const MetricHistory = lazy(() => import('./metrics').then((m) => ({ default: m.MetricHistory })));
import { DashboardRangeClock } from './range-clock';
import { TileSlots } from './tile-slots';
import { Tile, WidgetSkeleton } from './tile';

type Options = { tags: string[]; range: number; enabled: boolean; interval: number | false };
type Resolved = {
  kind: DashboardMetricSource['kind'];
  ids: string[];
  rows: DashboardSourceView[];
  pending: boolean;
  failed: boolean;
};
function SourceReads({
  batches,
  enabled,
  tags,
  children,
  collected = [],
}: {
  batches: { kind: Resolved['kind']; ids: string[] }[];
  collected?: Resolved[];
  children: (data: Resolved[]) => ReactNode;
} & Omit<Options, 'range' | 'interval'>) {
  const [batch, ...rest] = batches;
  const read = useDashboardRead(
    'listDashboardSources',
    { query: { kind: batch.kind, tags, ids: batch.ids.join(','), limit: batch.ids.length } },
    { enabled, retry: false },
  );
  const data = [
    ...collected,
    {
      ...batch,
      rows: read.isError ? [] : (read.data?.data ?? []),
      pending: !enabled || read.isLoading,
      failed: read.isError,
    },
  ];
  return rest.length ? (
    <SourceReads batches={rest} enabled={enabled} tags={tags} collected={data}>
      {children}
    </SourceReads>
  ) : (
    children(data)
  );
}
function MetricSources({
  kind,
  tiles,
  data,
  range,
  interval,
  enabled,
}: { kind: Resolved['kind']; tiles: MetricTile[]; data: Resolved[] } & Options) {
  const ids = [...new Set(tiles.map((t) => t.settings.source.id))];
  return (
    <>
      {ids.map((id) => {
        const subset = tiles.filter((t) => t.settings.source.id === id);
        const batch = data.find((b) => b.kind === kind && b.ids.includes(id));
        const row = batch?.rows.find((r) => r.id === id);
        if (!row || !row.matchesTags) {
          const placeholder = row ? 'Outside current filter.' : 'Resource unavailable.';
          return subset.map((w) => (
            <Tile key={w.id} tile={w} pending={!enabled || batch?.pending}>
              {batch?.pending ? <WidgetSkeleton kind="Metrics" /> : <p>{placeholder}</p>}
            </Tile>
          ));
        }
        if (row.exceedsChartSeriesLimit)
          return subset.map((tile) => (
            <Tile key={tile.id} tile={tile}>
              <p>This stack has more than eight recorded service identities.</p>
              <Link className="table-link" to={monitoringLink(sourcePath(kind, row), kind, 'stats', range)}>
                Full statistics
              </Link>
            </Tile>
          ));
        return (
          <Suspense
            key={id}
            fallback={subset.map((w) => (
              <Tile key={w.id} tile={w} pending>
                <WidgetSkeleton kind="Metrics" />
              </Tile>
            ))}>
            <MetricHistory
              tiles={subset}
              source={subset[0].settings.source}
              row={row}
              minutes={range}
              interval={interval}
            />
          </Suspense>
        );
      })}
    </>
  );
}
export function SupplementalTiles({ plan, ...options }: Options & { plan: DashboardRequirements }) {
  const editing = useContext(TileSlots)?.editing;
  const { metrics, notes, maintenance, fleet, watches, batches } = plan;
  const previewLimits = new Map<string, number>();
  for (const { settings } of fleet) {
    if (settings.kind === 'NeedsAttention') {
      const key = [...settings.kinds].sort().join(',');
      previewLimits.set(key, Math.max(settings.limit, previewLimits.get(key) ?? 0));
    }
  }

  return (
    <>
      {fleet.map((tile) => (
        <FleetTile
          key={tile.id}
          tile={tile}
          {...options}
          hasSummary={fleet.some(
            (w) => w.settings.kind === 'FleetSummary' && w.settings.measures.includes(DashboardFleetMeasure.Alerts),
          )}
          previewLimit={
            tile.settings.kind === 'NeedsAttention'
              ? previewLimits.get([...tile.settings.kinds].sort().join(','))
              : undefined
          }
        />
      ))}
      {notes.map((w) => (
        <Tile key={w.id} tile={w}>
          {editing && !w.settings.text && !w.settings.links.length ? (
            <p className="text-muted-foreground">Add a note or link in widget settings.</p>
          ) : (
            <MarkdownDescription allowLink={validLink}>{w.settings.text}</MarkdownDescription>
          )}
          <ul>
            {w.settings.links
              .filter((l) => validLink(l.href))
              .map((l, i) => (
                <li key={i}>
                  <Button variant="link" size="sm" asChild>
                    {l.href.startsWith('/') ? (
                      <Link to={l.href}>{l.label}</Link>
                    ) : (
                      <a href={l.href} rel="noreferrer">
                        {l.label}
                      </a>
                    )}
                  </Button>
                </li>
              ))}
          </ul>
        </Tile>
      ))}
      {batches.length > 0 && (
        <SourceReads {...options} batches={batches}>
          {(data) => (
            <>
              {watches.map((tile) => {
                const groups = tile.settings.pins.map((p) =>
                  data.find((b) => b.kind === p.kind && b.ids.includes(p.id)),
                );
                return (
                  <Tile tile={tile} key={tile.id} pending={groups.some((b) => b?.pending)}>
                    <ul className="divide-y">
                      {tile.settings.pins.map((pin, i) => {
                        const batch = groups[i];
                        const row = batch?.rows.find((r) => r.id === pin.id);
                        const Icon = CitadelIcons[pin.kind];
                        return (
                          <li className="group flex min-h-10 items-center gap-2 py-1" key={pin.kind + pin.id}>
                            <Icon className="size-4" />
                            {!row ? (
                              batch?.pending ? (
                                <Skeleton role="status" aria-label="Loading resource" className="h-4 w-32" />
                              ) : batch?.failed ? (
                                'Source read unavailable.'
                              ) : (
                                'Resource unavailable.'
                              )
                            ) : !row.matchesTags ? (
                              'Outside current filter.'
                            ) : (
                              <>
                                <Link className="table-link flex-1 truncate" to={sourcePath(pin.kind, row)}>
                                  {row.name}
                                </Link>
                                <StateIndicator
                                  kind={pin.kind === 'Platform' ? 'platform' : undefined}
                                  value={row.status}
                                />
                                {row.status}
                                {pin.kind !== 'Platform' && row.platformId && (
                                  <HealthCard
                                    title="Workload health"
                                    resourceName={row.name}
                                    triggerLabel={'View health for ' + row.name}
                                    trigger={<Activity className="size-4" />}
                                    status={null}
                                    summary={<WorkloadDetail kind={pin.kind} id={row.id} />}
                                  />
                                )}
                                <ResourceMenu
                                  path={sourcePath(pin.kind, row)}
                                  kind={pin.kind}
                                  range={options.range}
                                  logs={row.canViewLogs}
                                />
                              </>
                            )}
                          </li>
                        );
                      })}
                    </ul>
                  </Tile>
                );
              })}
              <DashboardRangeClock minutes={options.range} live={!!options.interval}>
                {[...metrics].map(([kind, tiles]) => (
                  <MetricSources {...options} data={data} key={kind} kind={kind} tiles={tiles} />
                ))}
              </DashboardRangeClock>
            </>
          )}
        </SourceReads>
      )}
      {maintenance.length > 0 && <MaintenanceTiles {...options} tiles={maintenance} />}
    </>
  );
}
