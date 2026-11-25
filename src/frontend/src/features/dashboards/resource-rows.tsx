import { useContext } from 'react';
import { EditorTileSlots, ViewOnlyContext } from './tile';
import { Link, useNavigate } from 'react-router';
import { Activity, ChartNoAxesCombined, ExternalLink, FileText, Link as LinkIcon } from 'lucide-react';
import { DropdownActions } from '@/components/custom/dropdown-with-dialog';
import { StateIndicator } from '@/components/custom/state-indicator';
import { healthLabels, healthTone, healthOrder } from '@/components/custom/health-status';
import { backupStatusOptions } from '@/features/backup-policies/coverage-status-options';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { HealthCard } from '@/components/custom/health-card';
import Loader from '@/components/ui/loader';
import { useRead } from '@/lib/hooks';
import { ContainerHealthSummary } from '@/features/docker-resources/containers/health';
import { WorkloadHealthSummary } from '@/features/workload-health/card';
import { workloadHealthStatus } from '@/features/workload-health/health';
import { BackupCoveragePopover } from '@/features/backup-policies/resource-backup-indicator';
import { ResourceBackupSummary, type ResourceBackupSource } from '@/features/backup-policies/resource-backup-summary';
import { monitoringLink } from '@/lib/monitoring-link';
import {
  PlatformStatus,
  type PlatformView,
  type DashboardWorkloadView,
  type DashboardMetricSource,
} from '@/api/generated/api.types';
import { sourcePath } from './source-path';

function ResourceMenu({
  kind,
  path,
  logs,
  range,
}: {
  kind: DashboardMetricSource['kind'];
  path: string;
  logs?: boolean;
  range?: number;
}) {
  const navigate = useNavigate();
  const [, copy] = useCopyToClipboard();
  return (
    <div className="shrink-0 [@media(hover:hover)]:opacity-0 group-hover:opacity-100 group-focus-within:opacity-100">
      <DropdownActions
        items={[
          { id: 'open', label: 'Open', icon: <ExternalLink />, onClick: () => navigate(path) },
          {
            id: 'stats',
            label: 'Statistics',
            icon: <ChartNoAxesCombined />,
            onClick: () => navigate(monitoringLink(path, kind, 'stats', range)),
          },
          ...(logs
            ? [
                {
                  id: 'logs',
                  label: 'Logs',
                  icon: <FileText />,
                  onClick: () => navigate(monitoringLink(path, kind, 'logs')),
                },
              ]
            : []),
          {
            id: 'copy',
            label: 'Copy link',
            icon: <LinkIcon />,
            onClick: () => void copy(new URL(path, window.location.origin).href),
          },
        ]}
      />
    </div>
  );
}
function Usage({ label, value }: { label: string; value?: number | null }) {
  const measured = value != null && Number.isFinite(value);
  return (
    <div className="min-w-16 text-xs tabular-nums">
      {label} {measured ? value.toFixed(1) + '%' : '—'}
      {measured && (
        <div className="mt-1 h-1 rounded bg-muted" aria-hidden="true">
          <div className="h-full rounded bg-primary/60" style={{ width: Math.min(100, Math.max(0, value)) + '%' }} />
        </div>
      )}
    </div>
  );
}
export function PlatformRow({ platform, range }: { platform: PlatformView; range: number }) {
  const path = '/platforms/edit/' + platform.id;
  const sample = platform.status === PlatformStatus.Online ? platform.stats?.[0] : undefined;
  return (
    <li className="group flex items-center gap-3 py-1">
      <StateIndicator value={platform.status} kind="platform" className="mr-0" />
      <Link to={path} className="table-link min-w-0 flex-1 truncate" title={platform.name}>
        {platform.name}
      </Link>
      <Usage label="CPU" value={sample?.cpuUsage} />
      <Usage label="Memory" value={sample?.memoryUsage} />
      <ResourceMenu path={path} kind="Platform" range={range} />
    </li>
  );
}
type WorkloadKind = Exclude<DashboardMetricSource['kind'], 'Platform'>;

export function WorkloadTable({ kind, rows }: { kind: WorkloadKind; rows: DashboardWorkloadView[] }) {
  return (
    <Table className="[--table-row-height:2rem] [--table-cell-padding-y:0]">
      <TableHeader>
        <TableRow>
          <TableHead>Name</TableHead>
          <TableHead className="w-14">Health</TableHead>
          <TableHead className="w-14">Backup</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((row) => (
          <WorkloadRow key={row.id} kind={kind} row={row} />
        ))}
      </TableBody>
    </Table>
  );
}

function WorkloadRow({ kind, row }: { kind: WorkloadKind; row: DashboardWorkloadView }) {
  const path = sourcePath(kind, row);
  const status =
    healthOrder.find((status) => row.observedProbes?.some((probe: string) => probe === status)) ?? 'Unknown';
  const tone = healthTone(status);
  const coverage =
    backupStatusOptions.find((s) => s.value === row.backupStatus) ??
    backupStatusOptions.find((s) => s.value === 'Unavailable')!;
  return (
    <TableRow className="group">
      <TableCell>
        <div className="flex min-w-0 items-center gap-2">
          <StateIndicator
            kind={kind === 'Container' ? 'container' : undefined}
            value={row.status}
            isProcessing={row.processing}
          />
          <Link title={row.name} className="table-link min-w-0 flex-1 truncate" to={path}>
            {row.name}
          </Link>
          <ResourceMenu path={path} kind={kind} logs={row.canViewLogs} />
        </div>
      </TableCell>
      <TableCell>
        <HealthCard
          title={kind === 'Container' ? 'Container health' : 'Workload health'}
          resourceName={row.name}
          triggerLabel={'Observed probes: ' + healthLabels[status]}
          triggerClassName={tone}
          trigger={<Activity className={'size-4 ' + tone} />}
          status={
            <span>
              {healthLabels[status]}
              {row.moreProbes ? ' · Partial sample' : ''}
            </span>
          }
          summary={<WorkloadDetail kind={kind} row={row} panel="health" />}
        />
      </TableCell>
      <TableCell>
        {coverage.value === 'NotApplicable' ? (
          <span className="text-muted-foreground" title={coverage.label}>
            —
          </span>
        ) : (
          <BackupCoveragePopover
            appearance={{ Icon: coverage.Icon, className: coverage.color, label: coverage.label }}
            showLabel>
            <WorkloadDetail kind={kind} row={row} panel="backup" />
          </BackupCoveragePopover>
        )}
      </TableCell>
    </TableRow>
  );
}

// Only mounted inside an open popover. Both panels reuse the authorized resource query and its cache.
function WorkloadDetail({
  kind,
  row,
  panel,
}: {
  kind: WorkloadKind;
  row: DashboardWorkloadView;
  panel: 'health' | 'backup';
}) {
  const viewOnly = useContext(ViewOnlyContext);
  const editor = useContext(EditorTileSlots);
  const paused = viewOnly && !editor;
  const opts = { staleTime: 15000, retry: false };
  const container = useRead('getContainer', { id: row.id }, { ...opts, enabled: !paused && kind === 'Container' });
  const deployment = useRead(
    'getDeployment',
    { deploymentId: row.id },
    { ...opts, enabled: !paused && kind === 'Deployment' },
  );
  const stack = useRead('getStack', { stackId: row.id }, { ...opts, enabled: !paused && kind === 'Stack' });
  const service = useRead(
    'getManagedSwarmService',
    { id: row.id },
    { ...opts, enabled: !paused && kind === 'SwarmService' },
  );
  const workload = kind === 'Stack' ? stack : kind === 'Deployment' ? deployment : service;
  const query = kind === 'Container' ? container : workload;
  if (paused) return <p>Resume the dashboard to load resource details.</p>;
  if (query.isError) return <p role="status">Resource details unavailable.</p>;
  if (!query.data) return <Loader />;
  if (panel === 'health') {
    if (kind === 'Container' && container.data) return <ContainerHealthSummary container={container.data.data} />;
    const resource = workload.data?.data;
    return resource ? (
      <WorkloadHealthSummary resource={resource} unavailable={workloadHealthStatus(resource) === 'Unknown'} />
    ) : null;
  }
  const source: ResourceBackupSource =
    kind === 'Container'
      ? { $type: 'Container', platformId: row.platformId, containerId: container.data!.data.containerId }
      : kind === 'Stack'
        ? { $type: 'Stack', stackId: row.id }
        : kind === 'Deployment'
          ? { $type: 'Deployment', deploymentId: row.id }
          : { $type: 'SwarmService', swarmServiceId: row.id };
  return (
    <ResourceBackupSummary coverage={query.data.data.backupCoverage} source={source} returnTo={sourcePath(kind, row)} />
  );
}
