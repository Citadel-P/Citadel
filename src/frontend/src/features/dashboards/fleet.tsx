import { useDashboardRead } from './use-dashboard-read';
import { useState, useContext, type ComponentProps, type ReactNode } from 'react';
import { Link } from 'react-router';
import { ArrowRight, ChevronRight, CircleCheck, ArrowUpRight } from 'lucide-react';
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
import { getStateIconAppearance, StateIndicator } from '@/components/custom/state-indicator';
import { Button } from '@/components/ui/button';
import Loader from '@/components/ui/loader';
import { TimelineItem } from './timeline';
import { useOpenAlertEventSheet } from '@/features/alerters/alert-events/alert-task-sheet';
import { cn, formatActivityEvent } from '@/lib/utils';
import { useRead } from '@/lib/hooks';
import { CitadelIcons } from '@/lib/icons';
import { Tile, ClearScopeContext, WidgetNotice, WidgetSkeleton } from './tile';
import { sourcePath } from './source-path';
import { FleetMembers } from './fleet-members';

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
  const openAlert = useOpenAlertEventSheet();
  const format = useProfileDateTimeFormatter();
  const Icon = CitadelIcons[row.kind as keyof typeof CitadelIcons] ?? CitadelIcons.Activity;
  const first = row.reasons[0];
  const second = row.reasons.find((r) => r.code !== first.code && ['Image update', 'New Git commit'].includes(r.code));
  return (
    <Sheet>
      <SheetTrigger asChild>
        <Button
          variant="ghost"
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
              {r.target &&
                (r.code.startsWith('Backup ') ? (
                  <Link className="table-link" to={'/backup-policies/edit/' + row.id + '?runId=' + r.target + '#runs'}>
                    Open backup run
                  </Link>
                ) : (
                  <button type="button" className="table-link" onClick={() => openAlert(r.target!)}>
                    Open alert
                  </button>
                ))}
            </div>
          ))}
        </div>
      </SheetContent>
    </Sheet>
  );
}
function Results({ query }: { query: Filter }) {
  const format = useProfileDateTimeFormatter();
  const openAlert = useOpenAlertEventSheet();
  const [page, setPage] = useState(1);
  const read = useDashboardRead(
    'getDashboardFleet',
    { query: { ...query, offset: (page - 1) * 25, limit: 25 } },
    { retry: false },
  );
  if (read.isError)
    return (
      <div className="px-4">
        <WidgetNotice>Resource list unavailable.</WidgetNotice>
      </div>
    );
  const result = read.data?.data;
  if (!result) return <Loader />;
  const total =
    query.read === 'Attention' && query.tab === 'Updates'
      ? result.updates
      : query.read === 'Attention' && query.tab === 'Problems'
        ? result.problems
        : result.total;
  return (
    <div className="@container min-h-0 flex-1 overflow-y-auto px-4">
      <p className="pb-2 text-xs text-muted-foreground" aria-live="polite">
        {total.toLocaleString()}{' '}
        {query.read === 'OpenAlerts'
          ? total === 1
            ? 'open alert'
            : 'open alerts'
          : total === 1
            ? 'matching resource'
            : 'matching resources'}
      </p>
      {query.read === 'OpenAlerts' ? (
        <ul>
          {result.rows.map((row) => {
            const event = row.reasons[0];
            return (
              <TimelineItem
                key={row.id}
                format={format}
                type={row.kind}
                title={event ? formatActivityEvent(event.code) : row.name}
                time={event?.observedAt ?? undefined}
                onOpen={() => openAlert(row.id)}
                tone={row.state === 'Critical' ? 'failure' : row.state === 'Warning' ? 'warning' : 'info'}
                status={<StateBadge kind="alertSeverity" value={row.state} />}
                context={event && row.name}
              />
            );
          })}
        </ul>
      ) : (
        result.rows.map((row) => <AttentionRow key={row.kind + row.id} row={row} />)
      )}
      {total > 25 && (
        <PaginationControls currentPage={page} onPageChange={setPage} totalPages={Math.ceil(total / 25)} />
      )}
    </div>
  );
}
function Drill({
  title,
  query,
  children,
  className,
  size,
}: {
  className?: string;
  size?: ComponentProps<typeof Button>['size'];
  title: string;
  query: Filter;
  children: ReactNode;
}) {
  return (
    <Sheet>
      <SheetTrigger asChild>
        <Button
          variant="ghost"
          size={size}
          className={className ?? 'h-auto justify-start whitespace-normal px-1 py-1.5 text-left'}>
          {children}
        </Button>
      </SheetTrigger>
      <SheetContent className="w-full sm:max-w-md">
        {query.read === 'Members' ? (
          <FleetMembers title={title} kinds={query.kinds} state={query.state} tags={query.tags} />
        ) : (
          <>
            <SheetHeader className="border-b pe-10">
              <SheetTitle>{title}</SheetTitle>
            </SheetHeader>
            <Results query={query} />
          </>
        )}
      </SheetContent>
    </Sheet>
  );
}
function FleetSummary({ data, measures, tags }: { data: DashboardFleetPage; measures: string[]; tags: string[] }) {
  const clearScope = useContext(ClearScopeContext);
  const empty =
    measures.includes('Platforms') && measures.includes('Alerts') && data.states.length === 0 && data.alerts === 0;
  const profile = useRead('getCurrentProfile', undefined, { enabled: empty && !tags.length });
  const auth = !profile.isError ? profile.data?.data.authorization : undefined;
  const states = data.states,
    platforms = states.filter((g) => g.kind === 'Platform'),
    workloads = states.filter((g) => g.kind !== 'Platform');
  const sum = (groups: typeof states) => groups.reduce((n, g) => n + g.count, 0);
  const totalPlatforms = sum(platforms);
  const online = platforms.find((g) => g.state === 'Online')?.count ?? 0;

  if (empty) {
    const Icon = CitadelIcons.Platform;
    return (
      <div className="flex min-w-0 flex-wrap items-center gap-3 border-s ps-4">
        <span aria-hidden="true" className="dashboard-card-symbol">
          <Icon className="size-4" />
        </span>
        <span className="font-medium">
          {tags.length ? 'No platforms or workloads match these tags.' : 'No platforms or workloads in view.'}
        </span>
        {tags.length > 0 && clearScope ? (
          <Button variant="outline" size="sm" onClick={clearScope}>
            Clear filters
          </Button>
        ) : !tags.length && (auth?.isAdministrator || auth?.writableResourceTypes?.includes(ResourceType.Platform)) ? (
          <Button variant="outline" size="sm" asChild>
            <Link to="/platforms/add">
              Add platform <ArrowRight aria-hidden="true" />
            </Link>
          </Button>
        ) : null}
      </div>
    );
  }

  const measureControls = measures.map((m) => {
    const Icon = m === 'Platforms' ? CitadelIcons.Platform : m === 'Alerts' ? CitadelIcons.Alert : CitadelIcons.Stack;
    const label = m === 'Platforms' ? 'Platforms online' : m === 'Alerts' ? 'Open alerts' : 'Managed workloads';
    const content = (
      <>
        <span className="dashboard-measure-icon" data-measure={m}>
          <Icon className="size-5 shrink-0" />
        </span>
        <span className="dashboard-measure-reading">
          <span className="dashboard-measure-label">{label}</span>
          <span className="dashboard-measure-figures">
            <strong className="dashboard-measure-value tabular-nums">
              {m === 'Platforms'
                ? online.toLocaleString() + ' / ' + totalPlatforms.toLocaleString()
                : m === 'Alerts'
                  ? data.alerts.toLocaleString()
                  : sum(workloads).toLocaleString()}
            </strong>
            {m === 'Platforms' && totalPlatforms > 0 && (
              <span className={online === totalPlatforms ? 'text-success' : 'text-muted-foreground'}>
                {Math.round((100 * online) / totalPlatforms)}%
              </span>
            )}
          </span>
        </span>
        <ArrowUpRight aria-hidden="true" className="dashboard-measure-link size-3" />
      </>
    );
    if (m === 'Workloads')
      return (
        <Sheet key={m}>
          <SheetTrigger asChild>
            <Button variant="ghost" className="dashboard-measure">
              {content}
            </Button>
          </SheetTrigger>
          <SheetContent className="w-full sm:max-w-md">
            <SheetHeader className="border-b pe-10">
              <SheetTitle>{label}</SheetTitle>
            </SheetHeader>
            <div className="flex flex-col divide-y px-4">
              {[DashboardStateKind.Deployment, DashboardStateKind.Stack, DashboardStateKind.SwarmService].map((k) => {
                const Icon = CitadelIcons[k];
                return (
                  <Drill key={k} title={kindLabel(k)} query={{ read: 'Members', kinds: k, tags }}>
                    <Icon aria-hidden="true" className="text-muted-foreground" />
                    {kindLabel(k)}
                    <strong className="ms-auto tabular-nums">
                      {sum(workloads.filter((g) => g.kind === k)).toLocaleString()}
                    </strong>
                    <ChevronRight />
                  </Drill>
                );
              })}
            </div>
          </SheetContent>
        </Sheet>
      );
    return (
      <Drill
        key={m}
        className="dashboard-measure"
        title={label}
        query={m === 'Alerts' ? { read: 'OpenAlerts', tags } : { read: 'Members', kinds: 'Platform', tags }}>
        {content}
      </Drill>
    );
  });
  return <div className="dashboard-measures">{measureControls}</div>;
}
function ReviewOpenAlerts({ tags, hasSummary }: { tags: string[]; hasSummary?: boolean }) {
  const summary = useDashboardRead('getDashboardFleet', { query: { read: 'Summary', tags } }, { enabled: false });
  const alertCount = hasSummary && !summary.isError ? summary.data?.data.alerts : undefined;
  if (alertCount === 0) return null;
  return (
    <>
      <p>Open alerts can include events outside these criteria.</p>
      <Drill title="Open alerts" query={{ read: 'OpenAlerts', tags }}>
        Review {alertCount == null ? '' : alertCount + ' '}open alerts <ArrowRight />
      </Drill>
    </>
  );
}

function ResourceStateDonut({
  states,
  total,
  label,
  statusKind,
}: {
  states: DashboardFleetPage['states'];
  total: number;
  label: string;
  statusKind: Parameters<typeof getStateIconAppearance>[1];
}) {
  const segments = [];
  let counted = 0;
  for (const { state, count } of states) {
    if (count <= 0) continue;
    const start = counted / total;
    counted += count;
    segments.push({ state, start, end: counted / total });
  }
  return (
    <div className="dashboard-state-donut relative grid aspect-square place-items-center">
      <div className="dashboard-state-ring pointer-events-none absolute inset-0" aria-hidden="true">
        {segments.map(({ state, start, end }) => (
          <span
            key={state}
            className={'absolute inset-0 bg-current ' + getStateIconAppearance(state, statusKind).className}
            style={{
              maskImage: `conic-gradient(transparent ${start}turn, black ${start}turn ${end}turn, transparent ${end}turn)`,
            }}
          />
        ))}
      </div>
      <div className="flex max-w-[63%] flex-col items-center text-center">
        <strong className="text-lg font-semibold leading-none tabular-nums">{total.toLocaleString()}</strong>
        <span className="mt-1 text-[10px] leading-tight text-muted-foreground">{label}</span>
      </div>
    </div>
  );
}

export function FleetTile({
  tile,
  tags,
  enabled,
  previewLimit,
  hasSummary,
}: {
  previewLimit?: number;
  hasSummary?: boolean;
  tile: TileType;
  tags: string[];
  enabled: boolean;
}) {
  const s = tile.settings;
  const [chosen, setKind] = useState<{ base: DashboardStateKind; value: DashboardStateKind }>();
  const kind =
    s.kind === 'ResourceState'
      ? chosen?.base === s.resourceKind
        ? chosen.value
        : s.resourceKind
      : DashboardStateKind.Deployment;
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
  const read = useDashboardRead('getDashboardFleet', { query }, { enabled: enabled && !unavailable, retry: false });
  const data = read.isError ? undefined : read.data?.data;
  const statusKind = kind === 'Platform' ? 'platform' : kind === 'Container' ? 'container' : undefined;
  const total = data?.states.reduce((n, g) => n + g.count, 0) ?? 0;
  let body: ReactNode;
  let attentionFilter: ReactNode;
  if (unavailable) body = <WidgetNotice>Container tag filtering is unavailable.</WidgetNotice>;
  else if (read.isError) body = <WidgetNotice>{tile.title} unavailable.</WidgetNotice>;
  else if (!data) body = <WidgetSkeleton kind={s.kind} />;
  else if (s.kind === 'FleetSummary') body = <FleetSummary data={data} measures={s.measures} tags={tags} />;
  else if (s.kind === 'ResourceState' && total === 0) {
    const Icon = CitadelIcons[kind];
    body = (
      <div className="flex flex-1 items-center justify-center gap-3 text-muted-foreground">
        <Icon aria-hidden="true" className="size-5 shrink-0" />
        <p>
          No {kindLabel(kind).toLowerCase()} {tags.length ? 'match these tags.' : 'in view.'}
        </p>
      </div>
    );
  } else if (s.kind === 'ResourceState')
    body = (
      <div className="dashboard-distribution">
        <ResourceStateDonut
          states={data.states}
          total={total}
          label={(total === 1 ? kindLabel(kind).slice(0, -1) : kindLabel(kind)).toLowerCase()}
          statusKind={statusKind}
        />
        <div className="dashboard-state-legend">
          {data.states.map((g) => (
            <Drill
              key={g.state}
              title={g.state + ' · ' + kindLabel(kind)}
              query={{ ...query, read: 'Members', state: g.state }}>
              <StateIndicator kind={statusKind} value={g.state} />
              {g.state} <strong className="ms-auto tabular-nums">{g.count.toLocaleString()}</strong>{' '}
              <span className="dashboard-state-share">
                {new Intl.NumberFormat(undefined, { style: 'percent', maximumFractionDigits: 1 }).format(
                  g.count / total,
                )}
              </span>
            </Drill>
          ))}
        </div>
      </div>
    );
  else {
    const counts = { All: data.total, Problems: data.problems, Updates: data.updates };
    attentionFilter = (
      <SegmentedControl
        className="dashboard-attention-filter"
        aria-label="Attention filter"
        value={tab}
        onValueChange={(v) => setTab({ base: defaultTab, value: v as DashboardAttentionTab })}
        options={Object.values(DashboardAttentionTab).map((v) => ({
          value: v,
          label: (
            <>
              {v} <span className="dashboard-tab-count">{counts[v]}</span>
            </>
          ),
        }))}
      />
    );
    body = (
      <>
        <div
          key={tab}
          className={cn(
            'dashboard-queue @container flex min-h-0 flex-1 flex-col overflow-auto',
            counts[tab] > 0 && 'px-4 py-2',
          )}>
          {data.rows.slice(0, s.limit).map((row) => (
            <AttentionRow key={row.kind + row.id} row={row} />
          ))}
          {!data.rows.length && (
            <div className="dashboard-attention-empty">
              <span aria-hidden="true" className="dashboard-radar">
                <CircleCheck />
              </span>
              <div>
                <p className="font-medium text-foreground">
                  {tab === 'Updates'
                    ? 'No known updates.'
                    : tab === 'All'
                      ? 'No matching resource issues or updates.'
                      : 'No matching resource issues.'}
                </p>
                {tab !== 'Updates' && <ReviewOpenAlerts tags={tags} hasSummary={hasSummary} />}
              </div>
            </div>
          )}
        </div>
        {counts[tab] > 0 && (
          <Drill
            title={tile.title}
            query={query}
            size="sm"
            className="dashboard-widget-footer shrink-0 text-muted-foreground hover:text-foreground">
            View all {counts[tab]} resources <ArrowRight aria-hidden="true" className="size-3.5" />
          </Drill>
        )}
      </>
    );
  }
  return (
    <Tile
      tile={tile}
      pending={!unavailable && (!enabled || read.isLoading)}
      controls={
        s.kind === 'ResourceState' ? (
          <Select
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
        ) : (
          attentionFilter
        )
      }>
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
