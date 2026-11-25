import Loader from '@/components/ui/loader';
import { ArrowRight, CalendarClock } from 'lucide-react';
import { monitoringLink } from '@/lib/monitoring-link';
import { Link } from 'react-router';
import { lazy, Suspense, useMemo } from 'react';
import type { DashboardWidget, DashboardMetricSource, DashboardWorkloadKind } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { WorkloadTable } from './resource-rows';
import { Timestamp } from '@/components/custom/timestamp';
import { useAppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { resourceLink } from '@/features/instance/schedules/presentation';
import { validLink } from './catalogue';
import type { MetricTile, WorkloadTile, DashboardRequirements } from './requirements';
import { sourcePath } from './source-path';
const MetricHistory = lazy(() => import('./metrics').then((m) => ({ default: m.MetricHistory })));
import { DashboardRangeClock } from './range-clock';
import { Tile, WidgetNotice } from './tile';

type Options = { tags: string[]; range: number; enabled: boolean; interval: number | false };
function WorkloadRows({
  kind,
  tiles,
  tags,
  enabled,
  interval,
}: { kind: DashboardWorkloadKind; tiles: WorkloadTile[] } & Options) {
  const limit = Math.max(...tiles.map((t) => t.settings.limit));
  const rows = useRead(
    'listDashboardWorkloads',
    { query: { kind, tags, limit } },
    { enabled, staleTime: interval ? 15000 : 0, refetchInterval: interval, retry: false },
  );
  return (
    <>
      {tiles.map((tile) => (
        <Tile key={tile.id} tile={tile}>
          {rows.isError ? (
            <WidgetNotice>Unavailable</WidgetNotice>
          ) : !rows.data ? (
            <Loader />
          ) : rows.data.data.length ? (
            <WorkloadTable kind={kind} rows={rows.data.data.slice(0, tile.settings.limit)} />
          ) : (
            <p className="text-muted-foreground">No matching workloads.</p>
          )}
        </Tile>
      ))}
    </>
  );
}
function MetricSources({
  kind,
  tiles,
  tags,
  range,
  enabled,
  interval,
}: { kind: DashboardMetricSource['kind']; tiles: MetricTile[] } & Options) {
  const ids = useMemo(() => [...new Set(tiles.map((t) => t.settings.source.id))].sort(), [tiles]);
  const meta = useRead(
    'listDashboardSources',
    { query: { kind, tags, ids: ids.join(','), limit: ids.length } },
    { enabled, staleTime: interval ? 15000 : 0, refetchInterval: interval, retry: false },
  );
  return (
    <>
      {ids.map((id) => {
        const subset = tiles.filter((t) => t.settings.source.id === id);
        const row = meta.isError ? undefined : meta.data?.data.find((r) => r.id === id);
        const matching = subset.filter(
          (w) => w.settings.source.kind !== 'Container' || w.settings.source.platformId === row?.platformId,
        );
        const missing = subset.filter((w) => !matching.includes(w));
        const placeholder = meta.isLoading
          ? 'Loading…'
          : !row
            ? 'Resource unavailable.'
            : !row.matchesTags
              ? 'Outside current filter.'
              : 'Resource unavailable.';
        if (!row || !row.matchesTags)
          return subset.map((w) => (
            <Tile key={w.id} tile={w} pending={!enabled || meta.isLoading}>
              <p>{placeholder}</p>
            </Tile>
          ));
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
          <div className="contents" key={id}>
            {missing.map((w) => (
              <Tile key={w.id} tile={w}>
                <WidgetNotice>Resource unavailable.</WidgetNotice>
              </Tile>
            ))}
            {matching.length > 0 && (
              <Suspense
                fallback={matching.map((w) => (
                  <Tile key={w.id} tile={w} pending>
                    Loading…
                  </Tile>
                ))}>
                <MetricHistory
                  tiles={matching}
                  source={matching[0].settings.source}
                  row={row}
                  minutes={range}
                  interval={interval}
                  manual={!interval}
                />
              </Suspense>
            )}
          </div>
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
  const { workloads, metrics, notes, maintenance } = plan;
  return (
    <>
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
      {[...workloads].map(([kind, tiles]) => (
        <WorkloadRows {...options} key={kind} kind={kind} tiles={tiles} />
      ))}
      {metrics.size > 0 && (
        <DashboardRangeClock minutes={options.range} live={!!options.interval}>
          {[...metrics].map(([kind, tiles]) => (
            <MetricSources {...options} key={kind} kind={kind} tiles={tiles} />
          ))}
        </DashboardRangeClock>
      )}
      {maintenance.length > 0 && <Maintenance {...options} tiles={maintenance} />}
    </>
  );
}
