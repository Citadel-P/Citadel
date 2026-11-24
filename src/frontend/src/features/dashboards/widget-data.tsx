import { monitoringLink } from '@/lib/monitoring-link';
import { Link } from 'react-router';
import { lazy, Suspense, useMemo } from 'react';
import type { DashboardWidget, DashboardSourceView, DashboardMetricSource } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { StateBadge } from '@/components/custom/state-badge';
import { Timestamp } from '@/components/custom/timestamp';
import { useAppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { resourceLink } from '@/features/instance/schedules/presentation';
import { validLink } from './catalogue';
import type { MetricTile } from './metrics';
import { sourcePath } from './source-path';
const MetricHistory = lazy(() => import('./metrics').then((m) => ({ default: m.MetricHistory })));
import { DashboardRangeClock } from './range-clock';
import { CopyResourceLink } from './row-actions';
import { Tile } from './tile';

type Options = { tags: string[]; range: number; enabled: boolean; interval: number | false };
type WorkloadTile = DashboardWidget & { settings: Extract<DashboardWidget['settings'], { kind: 'Workloads' }> };
function SourceRows({
  kind,
  tiles,
  tags,
  enabled,
  interval,
}: { kind: DashboardMetricSource['kind']; tiles: WorkloadTile[] } & Options) {
  const limit = Math.max(...tiles.map((t) => t.settings.limit));
  const rows = useRead(
    'listDashboardSources',
    { query: { kind, tags, limit } },
    { enabled, refetchInterval: interval, retry: false },
  );
  return (
    <>
      {tiles.map((tile) => (
        <Tile key={tile.id} tile={tile}>
          {rows.isError ? (
            <p>Unavailable</p>
          ) : !rows.data ? (
            <p>Loading…</p>
          ) : rows.data.data.length ? (
            <ul className="divide-y">
              {rows.data.data.slice(0, tile.settings.limit).map((r) => (
                <SourceRow key={r.id} kind={kind} row={r} />
              ))}
            </ul>
          ) : (
            <p>No matching workloads.</p>
          )}
        </Tile>
      ))}
    </>
  );
}
function SourceRow({ kind, row }: { kind: DashboardMetricSource['kind']; row: DashboardSourceView }) {
  const link = sourcePath(kind, row);
  return (
    <li className="flex flex-wrap items-center justify-between gap-2 py-1.5">
      <Link className="table-link" to={link}>
        {row.name}
      </Link>
      <StateBadge value={row.status} />
      <Link className="table-link text-xs" to={monitoringLink(link, kind, 'stats')}>
        Statistics
      </Link>
      <Link className="table-link text-xs" to={monitoringLink(link, kind, 'logs')}>
        Logs
      </Link>
      <CopyResourceLink path={link} />
    </li>
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
    { enabled, refetchInterval: interval, retry: false },
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
                <p>Resource unavailable.</p>
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
                <p>Unavailable</p>
              ) : (
                <>
                  <p>Installed version · {status.version}</p>
                  <p>
                    {status.availableUpdate
                      ? 'Available · ' + status.availableUpdate.version
                      : 'No available release reported.'}
                  </p>
                  {agentVersions.length > 0 && <p>Reported Agent versions · {agentVersions.join(', ')}</p>}
                  <Link className="table-link" to="/updates">
                    View updates
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
              <p>Unavailable</p>
            ) : !items ? (
              <p>Loading…</p>
            ) : items.length ? (
              <ul className="divide-y">
                {items.slice(0, tile.settings.kind === 'UpcomingSchedules' ? tile.settings.limit : 0).map((item) => {
                  const link = resourceLink(item);
                  return (
                    <li
                      key={item.purpose + item.ownerId}
                      className="flex flex-wrap items-center justify-between gap-2 py-1">
                      {link ? (
                        <Link to={link} className="table-link">
                          {item.name}
                        </Link>
                      ) : (
                        item.name
                      )}
                      <span>
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
export function SupplementalTiles({ tiles, ...options }: Options & { tiles: DashboardWidget[] }) {
  const workloads = tiles.filter((w): w is WorkloadTile => w.settings.kind === 'Workloads');
  const metrics = tiles.filter((w): w is MetricTile => w.settings.kind === 'Metrics');
  const maintenance = tiles.filter((w) => w.settings.kind === 'UpcomingSchedules' || w.settings.kind === 'Updates');
  return (
    <>
      {tiles.map(
        (w) =>
          w.settings.kind === 'Notes' && (
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
          ),
      )}
      {[...new Set(workloads.map((t) => t.settings.resourceKind))].map((kind) => (
        <SourceRows
          {...options}
          key={kind}
          kind={kind}
          tiles={workloads.filter((t) => t.settings.resourceKind === kind)}
        />
      ))}
      {metrics.length > 0 && (
        <DashboardRangeClock minutes={options.range} live={!!options.interval}>
          {[...new Set(metrics.map((t) => t.settings.source.kind))].map((kind) => (
            <MetricSources
              {...options}
              key={kind}
              kind={kind}
              tiles={metrics.filter((t) => t.settings.source.kind === kind)}
            />
          ))}
        </DashboardRangeClock>
      )}
      {maintenance.length > 0 && <Maintenance {...options} tiles={maintenance} />}
    </>
  );
}
