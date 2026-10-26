import {
  ActivityResourceType,
  BackupSourceSpec,
  BackupPolicyView,
  LookupResourceType,
  ResourceControlState,
} from '@/api/generated/api.types';
import { createResourceTargetColumn, type ResourceTargetValue } from '@/components/custom/resource-target-column';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { LastRunCell } from '@/components/custom/last-run-cell';
import { Timestamp } from '@/components/custom/timestamp';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable } from '@/components/ui/data-table';
import { TagChips } from '@/features/tags/components';
import { useSelectedResources } from '@/lib/atoms';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { useRead } from '@/lib/hooks';
import { ActionData } from '@/pages/types';
import { StockFeatures, ColumnDef } from '@tanstack/react-table';
import { CalendarClock, Database, HardDrive } from 'lucide-react';
import { useMemo } from 'react';
import { Link } from 'react-router';

export function BackupPoliciesTable({
  items,
  actions,
  isLoading,
}: {
  items: BackupPolicyView[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: BackupPolicyView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) {
  const [, setSelectedResources] = useSelectedResources<BackupPolicyView>('BackupPolicy');
  const formatDateTime = useProfileDateTimeFormatter();
  const sourceNames = useSourceNames(items);
  const cols = useMemo(
    () => columns(actions ?? {}, formatDateTime, sourceNames),
    [actions, formatDateTime, sourceNames],
  );

  return <DataTable columns={cols} data={items} isLoading={isLoading} onSelectionChange={setSelectedResources} />;
}

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: BackupPolicyView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
  formatDateTime: DateTimeFormatter,
  sourceNames: ReturnType<typeof useSourceNames>,
): ColumnDef<StockFeatures, BackupPolicyView>[] => [
  {
    id: 'select',
    header: ({ table }) => (
      <Checkbox
        checked={table.getIsAllPageRowsSelected() || (table.getIsSomePageRowsSelected() && 'indeterminate')}
        onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
        aria-label="Select all"
      />
    ),
    cell: ({ row }) => (
      <Checkbox
        checked={row.getIsSelected()}
        onCheckedChange={(value) => row.toggleSelected(!!value)}
        aria-label="Select backup policy"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <PolicyNameRow policy={row.original} />,
    sortFn: (rowA, rowB) => rowA.original.name.localeCompare(rowB.original.name),
  },
  createResourceTargetColumn({
    id: 'source',
    header: 'Source',
    getValue: (policy: BackupPolicyView) => sourceValue(policy.source, sourceNames),
  }),
  {
    accessorKey: 'cron',
    header: ({ column }) => <SortableCell cellName="Schedule" column={column} />,
    cell: ({ row }) => <ScheduleCell policy={row.original} />,
    sortFn: (rowA, rowB) => String(rowA.original.cron ?? '').localeCompare(String(rowB.original.cron ?? '')),
  },
  {
    accessorKey: 'latestRun',
    header: ({ column }) => <SortableCell cellName="Last Run" column={column} />,
    cell: ({ row }) => (
      <LastRunCell status={row.original.latestRun?.status}>
        <Timestamp value={getBackupPolicyLastRunAt(row.original)} formatDateTime={formatDateTime} />
      </LastRunCell>
    ),
    sortFn: (rowA, rowB) =>
      String(getBackupPolicyLastRunAt(rowA.original) ?? '').localeCompare(
        String(getBackupPolicyLastRunAt(rowB.original) ?? ''),
      ),
  },
  {
    accessorKey: 'tags',
    header: ({ column }) => <SortableCell cellName="Tags" column={column} />,
    cell: ({ row }) => <TagChips tags={row.original.tags} />,
    sortFn: (rowA, rowB) =>
      (rowA.original.tags?.map((tag) => tag.name).join(',') ?? '').localeCompare(
        rowB.original.tags?.map((tag) => tag.name).join(',') ?? '',
      ),
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

export const getBackupPolicyLastRunAt = (policy: Pick<BackupPolicyView, 'latestRun'>) =>
  policy.latestRun?.completedAt ?? policy.latestRun?.queuedAt;

const PolicyNameRow = ({ policy }: { policy: BackupPolicyView }) => (
  <div className="flex min-w-0 items-center gap-1">
    <StateIndicator
      value={policy.enabled}
      isProcessing={policy.controlState === ResourceControlState.Processing}
      enableLabel
    />
    <Link to={`../backup-policies/edit/${policy.id}`} title={policy.name} className="truncate text-sm hover:underline">
      {policy.name}
    </Link>
  </div>
);

const sourceValue = (source: BackupSourceSpec, names: ReturnType<typeof useSourceNames>): ResourceTargetValue => {
  const target = workloadTarget(source);
  if (target) {
    const name = names.names.get(`${target.type}:${target.id}`);
    if (name) return { target: { resourceType: target.type, resourceId: target.id, resourceName: name } };
    const label = names.loading.has(target.type)
      ? 'Loading source…'
      : `${target.type === ActivityResourceType.SwarmService ? 'Swarm Service' : target.type} unavailable`;
    return { label, content: <span className="text-sm text-muted-foreground">{label}</span> };
  }
  if (source.$type === 'BindMount')
    return {
      label: source.hostPath,
      content: (
        <span className="inline-flex min-w-0 max-w-80 items-center gap-2 text-sm">
          <HardDrive className="size-3.5 shrink-0 text-muted-foreground" />
          <span className="truncate" title={source.hostPath}>
            Bind: {source.hostPath}
          </span>
        </span>
      ),
    };
  if (source.$type === 'DockerVolume') {
    return {
      target: {
        resourceType: ActivityResourceType.Volume,
        resourceId: source.platformId,
        resourceName: source.volumeName,
        dockerNodeId: source.dockerNodeId,
      },
    };
  }

  return {
    label: 'Citadel backup',
    content: (
      <span className="inline-flex items-center gap-2 text-sm">
        <Database className="size-3.5 text-muted-foreground" />
        Citadel backup
      </span>
    ),
  };
};

const ScheduleCell = ({ policy }: { policy: BackupPolicyView }) => {
  if (!policy.cron) return <span className="text-sm text-muted-foreground">Manual</span>;

  return (
    <span className="inline-flex min-w-0 max-w-72 items-center gap-2 text-sm">
      <CalendarClock className="size-3.5 shrink-0 text-muted-foreground" />
      <span className="truncate" title={`${policy.cron} (${policy.timeZone ?? 'UTC'})`}>
        {policy.cron}
      </span>
      <span className="shrink-0 text-xs text-muted-foreground">{policy.timeZone ?? 'UTC'}</span>
    </span>
  );
};

function workloadTarget(source: BackupSourceSpec) {
  switch (source.$type) {
    case 'Deployment':
      return { type: ActivityResourceType.Deployment, id: source.deploymentId };
    case 'Stack':
      return { type: ActivityResourceType.Stack, id: source.stackId };
    case 'SwarmService':
      return { type: ActivityResourceType.SwarmService, id: source.swarmServiceId };
    default:
      return undefined;
  }
}

function useSourceNames(items: BackupPolicyView[]) {
  // Share the existing authorized lookup cache; never fetch a workload per row.
  const deployments = useRead(
    'lookup',
    { query: { TargetResourceType: LookupResourceType.Deployment } },
    {
      enabled: items.some((policy) => policy.source.$type === 'Deployment'),
    },
  );
  const stacks = useRead(
    'lookup',
    { query: { TargetResourceType: LookupResourceType.Stack } },
    {
      enabled: items.some((policy) => policy.source.$type === 'Stack'),
    },
  );
  const services = useRead(
    'lookup',
    { query: { TargetResourceType: LookupResourceType.SwarmService } },
    {
      enabled: items.some((policy) => policy.source.$type === 'SwarmService'),
    },
  );
  return useMemo(() => {
    const names = new Map<string, string>();
    for (const [type, resources] of [
      ['Deployment', deployments.data?.data],
      ['Stack', stacks.data?.data],
      ['SwarmService', services.data?.data],
    ] as const) {
      for (const resource of resources ?? []) names.set(`${type}:${resource.id}`, resource.name);
    }
    const loading = new Set<string>();
    if (deployments.isPending) loading.add('Deployment');
    if (stacks.isPending) loading.add('Stack');
    if (services.isPending) loading.add('SwarmService');
    return { names, loading };
  }, [deployments.data, stacks.data, services.data, deployments.isPending, stacks.isPending, services.isPending]);
}
