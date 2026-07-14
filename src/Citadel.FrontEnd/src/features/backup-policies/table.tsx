import {
  BackupPolicyView,
  BackupSourceSpec,
  BackupSourceSpecDockerVolumeBackupSource,
  BackupSourceSpecStackBackupSource,
  ResourceControlState,
} from '@/api/generated/api.types';
import { ContentCard } from '@/components/custom/content-card';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable } from '@/components/ui/data-table';
import { TagChips } from '@/features/tags/components';
import { useSelectedResources } from '@/lib/atoms';
import type { DateTimeFormatter } from '@/lib/date-time';
import { formatId } from '@/lib/utils';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { ActionData } from '@/pages/types';
import { ColumnDef } from '@tanstack/react-table';
import { CalendarClock, Database, HardDrive, Layers } from 'lucide-react';
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
  const cols = useMemo(() => columns(actions ?? {}, formatDateTime), [actions, formatDateTime]);

  return (
    <ContentCard>
      <DataTable columns={cols} data={items} isLoading={isLoading} onSelectionChange={setSelectedResources} />
    </ContentCard>
  );
}

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: BackupPolicyView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
  formatDateTime: DateTimeFormatter,
): ColumnDef<BackupPolicyView>[] => [
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
    sortingFn: (rowA, rowB) => rowA.original.name.localeCompare(rowB.original.name),
  },
  {
    accessorKey: 'source',
    header: ({ column }) => <SortableCell cellName="Source" column={column} />,
    cell: ({ row }) => <SourceCell source={row.original.source} />,
    sortingFn: (rowA, rowB) => sourceText(rowA.original.source).localeCompare(sourceText(rowB.original.source)),
  },
  {
    accessorKey: 'cron',
    header: ({ column }) => <SortableCell cellName="Schedule" column={column} />,
    cell: ({ row }) => <ScheduleCell policy={row.original} />,
    sortingFn: (rowA, rowB) => String(rowA.original.cron ?? '').localeCompare(String(rowB.original.cron ?? '')),
  },
  {
    accessorKey: 'lastScheduledRunAt',
    header: ({ column }) => <SortableCell cellName="Last Run" column={column} />,
    cell: ({ row }) => <TimestampCell value={row.original.lastScheduledRunAt} formatDateTime={formatDateTime} />,
    sortingFn: (rowA, rowB) =>
      String(rowA.original.lastScheduledRunAt ?? '').localeCompare(String(rowB.original.lastScheduledRunAt ?? '')),
  },
  {
    accessorKey: 'tags',
    header: ({ column }) => <SortableCell cellName="Tags" column={column} />,
    cell: ({ row }) => <TagChips tags={row.original.tags} />,
    sortingFn: (rowA, rowB) =>
      (rowA.original.tags?.map((tag) => tag.name).join(',') ?? '').localeCompare(
        rowB.original.tags?.map((tag) => tag.name).join(',') ?? '',
      ),
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

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

const SourceCell = ({ source }: { source: BackupSourceSpec }) => {
  if (source.$type === 'DockerVolume') {
    const dockerSource = source as BackupSourceSpecDockerVolumeBackupSource;

    return (
      <span className="inline-flex min-w-0 max-w-80 items-center gap-2 text-sm">
        <HardDrive className="size-3.5 shrink-0 text-muted-foreground" />
        <span className="truncate" title={dockerSource.volumeName}>
          {dockerSource.volumeName}
        </span>
      </span>
    );
  }

  if (source.$type === 'Stack') {
    const stackSource = source as BackupSourceSpecStackBackupSource;

    return (
      <span className="inline-flex min-w-0 max-w-80 items-center gap-2 text-sm">
        <Layers className="size-3.5 shrink-0 text-muted-foreground" />
        <span className="truncate" title={stackSource.stackId}>
          Stack {formatId(stackSource.stackId)}
        </span>
      </span>
    );
  }

  return (
    <span className="inline-flex items-center gap-2 text-sm">
      <Database className="size-3.5 text-muted-foreground" />
      Citadel system
    </span>
  );
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

const sourceText = (source: BackupSourceSpec) => {
  if (source.$type === 'DockerVolume') return (source as BackupSourceSpecDockerVolumeBackupSource).volumeName;
  if (source.$type === 'Stack') return (source as BackupSourceSpecStackBackupSource).stackId;
  if (source.$type === 'CitadelSystem') return 'Citadel system';
  return '';
};
