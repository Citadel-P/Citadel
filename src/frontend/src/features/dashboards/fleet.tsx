import { useState, useContext, type ReactNode } from 'react';
import { Link } from 'react-router';
import { ArrowRight, ChevronRight } from 'lucide-react';
import {
  DashboardFleetMeasure,
  DashboardAttentionTab,
  ResourceType,
  DashboardStateKind,
  type DashboardFleetRow,
  type DashboardFleetPage,
  type ActivityResourceType,
  type DashboardWidget,
} from '@/api/generated/api.types';
import { Timestamp } from '@/components/custom/timestamp';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { Select, SelectValue, SelectContent, SelectItem, SelectTrigger } from '@/components/ui/select';
import { SegmentedControl } from '@/components/ui/segmented-control';
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from '@/components/ui/sheet';
import { PaginationControls, TargetCell } from '@/components/custom/common';
import { StateBadge } from '@/components/custom/state-badge';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Button } from '@/components/ui/button';
import Loader from '@/components/ui/loader';
import { useRead } from '@/lib/hooks';
import { CitadelIcons } from '@/lib/icons';
import { Tile, ClearScopeContext, useFrozenDashboard, WidgetNotice } from './tile';
import { sourcePath } from './source-path';

type Filter = {
  read: 'Attention' | 'States' | 'Summary' | 'Members' | 'OpenAlerts';
  kinds?: string;
  tab?: DashboardAttentionTab;
  state?: string;
  tags: string[];
  limit?: number;
  offset?: number;
};
type TileType = DashboardWidget & {
  settings: Extract<DashboardWidget['settings'], { kind: 'NeedsAttention' | 'ResourceState' | 'FleetSummary' }>;
};
function owner(row: DashboardFleetRow) {
  return row.kind === 'Container' ? (
    <Link className="table-link" to={sourcePath('Container', row)}>
      {row.name}
    </Link>
  ) : (
    <TargetCell resourceId={row.id} resourceType={row.kind as ActivityResourceType} resourceName={row.name} />
  );
}
function Reason({ code }: { code: string }) {
  if (code.startsWith('Backup '))
    return (
      <StateBadge
        kind="run"
        value={code.slice(7)}
        label={code === 'Backup Failed' ? 'Latest run failed' : code.slice(7).replace(/([a-z])([A-Z])/g, '$1 $2')}
      />
    );
  if (code.endsWith(' alert')) return <StateBadge kind="alertSeverity" value={code.split(' ')[0]} label={code} />;
  if (code === 'Image update' || code === 'New Git commit')
    return <StateBadge kind="activity" value="Information" label={code} />;
  return <StateBadge value={code} />;
}
function AttentionRow({ row }: { row: DashboardFleetRow }) {
  const paused = useFrozenDashboard();
  const format = useProfileDateTimeFormatter();
  const Icon = CitadelIcons[row.kind as keyof typeof CitadelIcons] ?? CitadelIcons.Activity;
  const first = row.reasons[0];
  const second = row.reasons.find((r) => r.code !== first.code && ['Image update', 'New Git commit'].includes(r.code));
  return (
    <Sheet>
      <SheetTrigger asChild>
        <Button
          variant="ghost"
          disabled={paused}
          className="grid h-auto w-full grid-cols-[1rem_minmax(0,1fr)_1rem] items-center gap-x-3 gap-y-0.5 border-b px-1.5 py-1 text-left @min-[24rem]:flex @min-[24rem]:h-11 @min-[24rem]:py-0">
          <Icon className="shrink-0" />
          <span className="min-w-0 flex-1">
            <span title={row.name} className="block truncate font-medium">
              {row.name}
            </span>
            <span className="block text-xs font-normal text-muted-foreground">
              {row.kind.replace(/([a-z])([A-Z])/g, '$1 $2')}
            </span>
          </span>
          <span className="col-start-2 row-start-2 flex flex-wrap gap-1 @min-[24rem]:justify-end">
            {first && <Reason code={first.code} />} {second && <Reason code={second.code} />}
          </span>
          <ChevronRight className="col-start-3 row-start-1 size-3.5" />
        </Button>
      </SheetTrigger>
      <SheetContent className="overflow-auto">
        <SheetHeader>
          <SheetTitle>{row.name}</SheetTitle>
        </SheetHeader>
        <div className="space-y-4 px-4">
          {owner(row)}
          {row.reasons.map((r, i) => (
            <div key={i} className="space-y-1 border-t py-2">
              <Reason code={r.code} />{' '}
              {r.occurrences > 1 && (
                <span className="text-xs text-muted-foreground">{r.occurrences} observations · earliest shown</span>
              )}
              {r.evidence && <p className="break-all font-mono text-xs">{r.evidence}</p>}
              {r.observedAt && <Timestamp value={r.observedAt} formatDateTime={format} />}
              {['Image update', 'New Git commit'].includes(r.code) &&
                (row.kind === 'Stack' || row.kind === 'Deployment' || row.kind === 'SwarmService') && (
                  <Link className="table-link block" to={sourcePath(row.kind, row)}>
                    View update details
                  </Link>
                )}
              {r.target && (
                <Link
                  className="table-link"
                  to={
                    r.code.startsWith('Backup ')
                      ? '/backup-policies/edit/' + row.id + '?runId=' + r.target + '#runs'
                      : '/alerts/' + r.target
                  }>
                  {r.code.startsWith('Backup ') ? 'Open backup run' : 'Open alert'}
                </Link>
              )}
            </div>
          ))}
        </div>
      </SheetContent>
    </Sheet>
  );
}
function Results({ query }: { query: Filter }) {
  const [page, setPage] = useState(1);
  const read = useRead(
    'getDashboardFleet',
    { query: { ...query, offset: (page - 1) * 25, limit: 25 } },
    { retry: false },
  );
  if (read.isError) return <WidgetNotice>Resource list unavailable.</WidgetNotice>;
  const result = read.data?.data;
  if (!result) return <Loader />;
  const total =
    query.read === 'Attention' && query.tab === 'Updates'
      ? result.updates
      : query.read === 'Attention' && query.tab === 'Problems'
        ? result.problems
        : result.total;
  return (
    <div className="@container min-h-0 overflow-y-auto px-4">
      {result.rows.map((row) =>
        query.read === 'Attention' ? (
          <AttentionRow key={row.kind + row.id} row={row} />
        ) : (
          <div key={row.kind + row.id} className="flex justify-between border-b py-2">
            {query.read === 'OpenAlerts' ? (
              <Link className="table-link" to={'/alerts/' + row.id}>
                {row.name}
              </Link>
            ) : (
              owner(row)
            )}
            <StateBadge kind={query.read === 'OpenAlerts' ? 'alertSeverity' : undefined} value={row.state} />
          </div>
        ),
      )}
      {total === 0 && <p>No matching resources.</p>}
      {total > 25 && (
        <PaginationControls currentPage={page} onPageChange={setPage} totalPages={Math.ceil(total / 25)} />
      )}
    </div>
  );
}
function Drill({ title, query, children }: { title: string; query: Filter; children: ReactNode }) {
  const paused = useFrozenDashboard();
  return (
    <Sheet>
      <SheetTrigger asChild>
        <Button
          disabled={paused}
          variant="ghost"
          className="h-auto justify-start whitespace-normal px-1 py-1.5 text-left">
          {children}
        </Button>
      </SheetTrigger>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>{title}</SheetTitle>
        </SheetHeader>
        <Results query={query} />
      </SheetContent>
    </Sheet>
  );
}
function FleetSummary({ data, measures, tags }: { data: DashboardFleetPage; measures: string[]; tags: string[] }) {
  const clearScope = useContext(ClearScopeContext);
  const empty = measures.includes('Alerts') && data.total === 0 && data.states.length === 0;
  const paused = useFrozenDashboard();
  const profile = useRead('getCurrentProfile', undefined, { enabled: !paused && empty && !tags.length });
  const auth = !profile.isError ? profile.data?.data.authorization : undefined;
  const states = data.states,
    platforms = states.filter((g) => g.kind === 'Platform'),
    workloads = states.filter((g) => g.kind !== 'Platform');
  const sum = (groups: typeof states) => groups.reduce((n, g) => n + g.count, 0);

  const measureControls = measures.map((m) => {
    const Icon = m === 'Platforms' ? CitadelIcons.Platform : m === 'Alerts' ? CitadelIcons.Alert : CitadelIcons.Stack;
    const label = m === 'Platforms' ? 'Platforms online' : m === 'Alerts' ? 'Open alerts' : 'Managed workloads';
    const content = (
      <>
        <Icon className="size-5 shrink-0" />
        <span>
          <span className="block text-xs text-muted-foreground">{label}</span>
          <strong className="text-xl tabular-nums">
            {m === 'Platforms'
              ? (platforms.find((g) => g.state === 'Online')?.count ?? 0) + ' / ' + sum(platforms)
              : m === 'Alerts'
                ? data.alerts
                : sum(workloads).toLocaleString()}
          </strong>
        </span>
      </>
    );
    if (m === 'Workloads')
      return (
        <Sheet key={m}>
          <SheetTrigger asChild>
            <Button disabled={paused} variant="ghost" className="h-auto justify-start px-1 py-1.5 text-left">
              {content}
            </Button>
          </SheetTrigger>
          <SheetContent>
            <SheetHeader>
              <SheetTitle>{label}</SheetTitle>
            </SheetHeader>
            <div className="flex flex-col px-4">
              {[DashboardStateKind.Deployment, DashboardStateKind.Stack, DashboardStateKind.SwarmService].map((k) => (
                <Drill key={k} title={kindLabel(k)} query={{ read: 'Members', kinds: k, tags }}>
                  {kindLabel(k)}{' '}
                  <strong className="ms-auto tabular-nums">
                    {sum(workloads.filter((g) => g.kind === k)).toLocaleString()}
                  </strong>
                  <ChevronRight />
                </Drill>
              ))}
            </div>
          </SheetContent>
        </Sheet>
      );
    return (
      <Drill
        key={m}
        title={label}
        query={m === 'Alerts' ? { read: 'OpenAlerts', tags } : { read: 'Members', kinds: 'Platform', tags }}>
        {content}
      </Drill>
    );
  });
  return (
    <div className="@container flex flex-wrap items-center justify-around gap-x-4 gap-y-1">
      {empty && (
        <div className="flex items-center gap-2 text-sm">
          {tags.length ? 'No resources match these tags.' : 'No resources yet.'}
          {tags.length > 0 && clearScope ? (
            <Button variant="link" onClick={clearScope}>
              Clear filters
            </Button>
          ) : !tags.length &&
            (auth?.isAdministrator || auth?.writableResourceTypes?.includes(ResourceType.Platform)) ? (
            <Link className="table-link" to="/platforms/add">
              Add platform
            </Link>
          ) : null}
        </div>
      )}
      {measureControls[0]}
      {measureControls.length > 1 && (
        <>
          <div className="hidden flex-1 items-center justify-around gap-2 @min-[24rem]:flex">
            {measureControls.slice(1)}
          </div>
          <Sheet>
            <SheetTrigger asChild>
              <Button variant="outline" size="sm" className="@min-[24rem]:hidden">
                All measures
              </Button>
            </SheetTrigger>
            <SheetContent>
              <SheetHeader>
                <SheetTitle>Fleet summary</SheetTitle>
              </SheetHeader>
              <div className="flex flex-col gap-2 px-4">{measureControls}</div>
            </SheetContent>
          </Sheet>
        </>
      )}
    </div>
  );
}
export function FleetTile({
  tile,
  tags,
  enabled,
  interval,
  previewLimit,
}: {
  previewLimit?: number;
  tile: TileType;
  tags: string[];
  enabled: boolean;
  interval: number | false;
}) {
  const s = tile.settings;
  const [chosen, setKind] = useState<{ base: DashboardStateKind; value: DashboardStateKind }>();
  const kind =
    s.kind === 'ResourceState'
      ? chosen?.base === s.resourceKind
        ? chosen.value
        : s.resourceKind
      : DashboardStateKind.Deployment;
  const paused = useFrozenDashboard();
  const defaultTab = s.kind === 'NeedsAttention' ? s.tab : DashboardAttentionTab.All;
  const [override, setTab] = useState<{ base: DashboardAttentionTab; value: DashboardAttentionTab }>();
  const tab = override?.base === defaultTab ? override.value : defaultTab;
  const query: Filter =
    s.kind === 'FleetSummary'
      ? s.measures.includes(DashboardFleetMeasure.Alerts)
        ? { read: 'Summary', tags }
        : {
            read: 'States',
            tags,
            kinds: [
              ...(s.measures.includes(DashboardFleetMeasure.Platforms) ? ['Platform'] : []),
              ...(s.measures.includes(DashboardFleetMeasure.Workloads) ? ['Deployment', 'Stack', 'SwarmService'] : []),
            ].join(','),
          }
      : s.kind === 'ResourceState'
        ? { read: 'States', kinds: kind, tags }
        : {
            read: 'Attention',
            tags,
            kinds: [...s.kinds].sort().join(',') || undefined,
            tab,
            limit: previewLimit ?? s.limit,
          };
  const unavailable = s.kind === 'ResourceState' && kind === 'Container' && tags.length > 0;
  const read = useRead(
    'getDashboardFleet',
    { query },
    { enabled: enabled && !unavailable, refetchInterval: interval, staleTime: interval ? 15000 : 0, retry: false },
  );
  const data = read.isError ? undefined : read.data?.data;
  let body: ReactNode;
  if (unavailable) body = <WidgetNotice>Container tag filtering is unavailable.</WidgetNotice>;
  else if (read.isError) body = <WidgetNotice>{tile.title} unavailable.</WidgetNotice>;
  else if (!data) body = <Loader />;
  else if (s.kind === 'FleetSummary') body = <FleetSummary data={data} measures={s.measures} tags={tags} />;
  else if (s.kind === 'ResourceState')
    body = (
      <>
        <strong className="text-xl tabular-nums">
          {data.states.reduce((n, g) => n + g.count, 0).toLocaleString()}{' '}
          <span className="text-sm">{kindLabel(kind)}</span>
        </strong>
        {data.states.map((g) => (
          <Drill
            key={g.state}
            title={g.state + ' · ' + kindLabel(kind)}
            query={{ ...query, read: 'Members', state: g.state }}>
            <StateIndicator value={g.state} />
            {g.state}
            <strong className="ms-auto tabular-nums">{g.count.toLocaleString()}</strong>
          </Drill>
        ))}
      </>
    );
  else {
    const counts = { All: data.total, Problems: data.problems, Updates: data.updates };
    body = (
      <>
        <SegmentedControl
          disabled={paused}
          aria-label="Attention filter"
          value={tab}
          onValueChange={(v) => setTab({ base: defaultTab, value: v as DashboardAttentionTab })}
          options={Object.values(DashboardAttentionTab).map((v) => ({ value: v, label: v + ' ' + counts[v] }))}
        />
        <div className="@container min-h-0 flex-1 overflow-auto">
          {data.rows.slice(0, s.limit).map((row) => (
            <AttentionRow key={row.kind + row.id} row={row} />
          ))}
          {!data.rows.length && (
            <p className="py-3 text-muted-foreground">
              {tab === 'Updates'
                ? 'No known updates.'
                : tab === 'All'
                  ? 'No known issues or updates.'
                  : 'No known issues.'}
            </p>
          )}
        </div>
        {counts[tab] > 0 && (
          <Drill title={tile.title} query={query}>
            View all {counts[tab]} resources <ArrowRight />
          </Drill>
        )}
      </>
    );
  }
  return (
    <Tile tile={tile} pending={!unavailable && (!enabled || read.isLoading)}>
      {s.kind === 'ResourceState' && (
        <Select
          disabled={paused}
          value={kind}
          onValueChange={(value) => setKind({ base: s.resourceKind, value: value as DashboardStateKind })}>
          <SelectTrigger aria-label="Resource type" className="w-fit self-end">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {Object.values(DashboardStateKind).map((kind) => {
              const Icon = CitadelIcons[kind];
              return (
                <SelectItem key={kind} value={kind}>
                  <Icon aria-hidden="true" className="size-4 text-muted-foreground" />
                  {kindLabel(kind)}
                </SelectItem>
              );
            })}
          </SelectContent>
        </Select>
      )}
      {body}
    </Tile>
  );
}
export function kindLabel(kind: DashboardStateKind) {
  return {
    Platform: 'Platforms',
    Deployment: 'Deployments',
    Stack: 'Stacks',
    SwarmService: 'Swarm services',
    Container: 'Containers',
  }[kind];
}
