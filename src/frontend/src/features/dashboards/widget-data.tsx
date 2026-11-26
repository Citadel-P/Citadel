import Loader from '@/components/ui/loader';
import { ArrowRight, CalendarClock } from 'lucide-react';
import { monitoringLink } from '@/lib/monitoring-link';
import { Link } from 'react-router';
import { lazy, Suspense, useMemo, type ReactNode } from 'react';
import type { DashboardWidget, DashboardMetricSource, DashboardSourceView } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { FleetTile } from './fleet';
import { StateIndicator } from '@/components/custom/state-indicator';
import { CitadelIcons } from '@/lib/icons';
import { HealthCard } from '@/components/custom/health-card';
import { Activity } from 'lucide-react';
import { ResourceMenu, WorkloadDetail } from './resource-rows';
import { Timestamp } from '@/components/custom/timestamp';
import { useAppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { resourceLink } from '@/features/instance/schedules/presentation';
import { validLink } from './catalogue';
import type { MetricTile, DashboardRequirements } from './requirements';
import { sourcePath } from './source-path';
const MetricHistory = lazy(() => import('./metrics').then((m) => ({ default: m.MetricHistory })));
import { DashboardRangeClock } from './range-clock';
import { Tile, WidgetNotice } from './tile';

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
  interval,
  children,
  collected = [],
}: {
  batches: { kind: Resolved['kind']; ids: string[] }[];
  collected?: Resolved[];
  children: (data: Resolved[]) => ReactNode;
} & Omit<Options, 'range'>) {
  const [batch, ...rest] = batches;
  const read = useRead(
    'listDashboardSources',
    { query: { kind: batch.kind, tags, ids: batch.ids.join(','), limit: batch.ids.length } },
    { enabled, staleTime: interval ? 15000 : 0, refetchInterval: interval, retry: false },
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
    <SourceReads batches={rest} enabled={enabled} tags={tags} interval={interval} collected={data}>
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
          const placeholder = batch?.pending ? 'Loading…' : row ? 'Outside current filter.' : 'Resource unavailable.';
          return subset.map((w) => (
            <Tile key={w.id} tile={w} pending={!enabled || batch?.pending}>
              <p>{placeholder}</p>
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
                Loading…
              </Tile>
            ))}>
            <MetricHistory
              tiles={subset}
              source={subset[0].settings.source}
              row={row}
              minutes={range}
              interval={interval}
              manual={!interval}
            />
          </Suspense>
        );
      })}
    </>
  );
}
function Maintenance({ tiles, tags, enabled, interval }: Options & { tiles: DashboardWidget[] }) {
  const profile = useRead('getCurrentProfile');
  const admin = !profile.isError && profile.data?.data.authorization.isAdministrator === true;
  const limit = Math.max(0, ...tiles.map((w) => (w.settings.kind === 'UpcomingSchedules' ? w.settings.limit : 0)));
  const schedules = useRead(
    'getUpcomingSchedules',
    { query: { tags, limit } },
    { enabled: admin && enabled && limit > 0, refetchInterval: interval },
  );
  const { applicationInfo, platforms } = useAppContext();
  const info = useRead('getApplicationInfo', undefined, {
    enabled: admin && tiles.some((w) => w.settings.kind === 'Updates'),
    staleTime: interval || 0,
    refetchInterval: interval,
  });
  const status = info.isError ? undefined : (info.data?.data ?? applicationInfo);
  const agentVersions = [...new Set(platforms?.map((p) => p.agentVersion).filter(Boolean))];
  const format = useProfileDateTimeFormatter();
  return (
    <>
      {tiles.map((tile) => {
        if (!admin)
          return (
            <Tile key={tile.id} tile={tile}>
              <p>{profile.isLoading ? 'Loading…' : 'Administrator access required.'}</p>
            </Tile>
          );
        if (tile.settings.kind === 'Updates') {
          return (
            <Tile key={tile.id} tile={tile}>
              <div>
                <Badge variant="outline">Instance-wide</Badge>
              </div>
              {!status ? (
                <WidgetNotice>Unavailable</WidgetNotice>
              ) : (
                <>
                  <p className="flex flex-wrap items-baseline gap-x-2 rounded-md border bg-muted/30 px-3 py-2 text-xs">
                    <span className="text-muted-foreground">Installed version</span>
                    <strong className="break-all font-mono">{status.version}</strong>
                  </p>
                  <p>
                    {status.availableUpdate
                      ? 'Available · ' + status.availableUpdate.version
                      : 'No available release reported.'}
                  </p>
                  {agentVersions.length > 0 && <p>Reported Agent versions · {agentVersions.join(', ')}</p>}
                  <Link
                    className="table-link mt-auto inline-flex w-fit items-center gap-1.5 text-xs font-medium"
                    to="/updates">
                    View updates <ArrowRight className="size-3" />
                  </Link>
                </>
              )}
            </Tile>
          );
        }
        const items = schedules.data?.data.items;
        return (
          <Tile key={tile.id} tile={tile}>
            {schedules.isError ? (
              <WidgetNotice>Unavailable</WidgetNotice>
            ) : !items ? (
              <Loader />
            ) : items.length ? (
              <ul className="divide-y">
                {items.slice(0, tile.settings.kind === 'UpcomingSchedules' ? tile.settings.limit : 0).map((item) => {
                  const link = resourceLink(item);
                  return (
                    <li key={item.purpose + item.ownerId} className="flex min-w-0 items-center gap-2.5 py-2 text-xs">
                      <CalendarClock aria-hidden="true" className="size-3.5 shrink-0 text-muted-foreground" />
                      {link ? (
                        <Link to={link} className="table-link min-w-0 truncate font-mono" title={item.name}>
                          {item.name}
                        </Link>
                      ) : (
                        <span className="min-w-0 truncate font-mono" title={item.name}>
                          {item.name}
                        </span>
                      )}
                      <span className="ms-auto shrink-0 text-muted-foreground">
                        {item.nextAt && <Timestamp value={item.nextAt} formatDateTime={format} />} {item.timeZone}
                      </span>
                    </li>
                  );
                })}
              </ul>
            ) : (
              <p>No upcoming schedules.</p>
            )}
          </Tile>
        );
      })}
    </>
  );
}
export function SupplementalTiles({ plan, ...options }: Options & { plan: DashboardRequirements }) {
  const { metrics, notes, maintenance, fleet, watches } = plan;
  const previewLimits = new Map<string, number>();
  for (const { settings } of fleet) {
    if (settings.kind === 'NeedsAttention') {
      const key = [...settings.kinds].sort().join(',');
      previewLimits.set(key, Math.max(settings.limit, previewLimits.get(key) ?? 0));
    }
  }
  const batches = useMemo(() => {
    const pins = [
      ...[...metrics.values()].flat().map((t) => t.settings.source),
      ...watches.flatMap((w) => w.settings.pins),
    ];
    const groups = new Map<Resolved['kind'], Set<string>>();
    for (const p of pins) {
      const set = groups.get(p.kind) ?? new Set<string>();
      set.add(p.id);
      groups.set(p.kind, set);
    }
    return [...groups]
      .sort(([a], [b]) => a.localeCompare(b))
      .flatMap(([kind, set]) => {
        const ids = [...set].sort();
        return Array.from({ length: Math.ceil(ids.length / 8) }, (_, i) => ({
          kind,
          ids: ids.slice(i * 8, i * 8 + 8),
        }));
      });
  }, [metrics, watches]);
  return (
    <>
      {fleet.map((tile) => (
        <FleetTile
          key={tile.id}
          tile={tile}
          {...options}
          previewLimit={
            tile.settings.kind === 'NeedsAttention'
              ? previewLimits.get([...tile.settings.kinds].sort().join(','))
              : undefined
          }
        />
      ))}
      {notes.map((w) => (
        <Tile key={w.id} tile={w}>
          <p className="whitespace-pre-wrap break-words">{w.settings.text}</p>
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
                                'Loading…'
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
                                <StateIndicator value={row.status} />
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
      {maintenance.length > 0 && <Maintenance {...options} tiles={maintenance} />}
    </>
  );
}
