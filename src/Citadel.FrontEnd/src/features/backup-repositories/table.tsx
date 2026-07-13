import {
  BackupRepositorySpec,
  BackupRepositorySpecFileSystemBackupRepositorySpec,
  BackupRepositorySpecS3CompatibleBackupRepositorySpec,
  BackupRepositoryType,
  BackupRepositoryView,
} from '@/api/generated/api.types';
import { ContentCard } from '@/components/custom/content-card';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable } from '@/components/ui/data-table';
import { useSelectedResources } from '@/lib/atoms';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { ActionData } from '@/pages/types';
import { ColumnDef } from '@tanstack/react-table';
import { Cloud, FolderLock } from 'lucide-react';
import { useMemo } from 'react';
import { Link } from 'react-router';

export function BackupRepositoriesTable({
  items,
  actions,
  isLoading,
}: {
  items: BackupRepositoryView[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: BackupRepositoryView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) {
  const [, setSelectedResources] = useSelectedResources<BackupRepositoryView>('BackupRepository');
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
    React.FC<{ resource: BackupRepositoryView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
  formatDateTime: DateTimeFormatter,
): ColumnDef<BackupRepositoryView>[] => [
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
        aria-label="Select backup repository"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <RepositoryNameRow repository={row.original} />,
    sortingFn: (rowA, rowB) => rowA.original.name.localeCompare(rowB.original.name),
  },
  {
    accessorKey: 'type',
    header: ({ column }) => <SortableCell cellName="Type" column={column} />,
    cell: ({ row }) => <RepositoryTypeCell repository={row.original} />,
    sortingFn: (rowA, rowB) => rowA.original.type.localeCompare(rowB.original.type),
  },
  {
    id: 'destination',
    header: ({ column }) => <SortableCell cellName="Destination" column={column} />,
    cell: ({ row }) => <DestinationCell spec={row.original.spec} />,
    sortingFn: (rowA, rowB) => destinationText(rowA.original.spec).localeCompare(destinationText(rowB.original.spec)),
  },
  {
    accessorKey: 'lastCheckedAt',
    header: ({ column }) => <SortableCell cellName="Last Check" column={column} />,
    cell: ({ row }) => <TimestampCell value={row.original.lastCheckedAt} formatDateTime={formatDateTime} />,
    sortingFn: (rowA, rowB) =>
      String(rowA.original.lastCheckedAt ?? '').localeCompare(String(rowB.original.lastCheckedAt ?? '')),
  },
  {
    accessorKey: 'lastPrunedAt',
    header: ({ column }) => <SortableCell cellName="Last Prune" column={column} />,
    cell: ({ row }) => <TimestampCell value={row.original.lastPrunedAt} formatDateTime={formatDateTime} />,
    sortingFn: (rowA, rowB) =>
      String(rowA.original.lastPrunedAt ?? '').localeCompare(String(rowB.original.lastPrunedAt ?? '')),
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

const RepositoryNameRow = ({ repository }: { repository: BackupRepositoryView }) => (
  <div className="flex min-w-0 items-center gap-1">
    <StateIndicator value={repository.status} />
    <Link
      to={`../backup-repositories/edit/${repository.id}`}
      title={repository.name}
      className="truncate text-sm hover:underline">
      {repository.name}
    </Link>
  </div>
);

const RepositoryTypeCell = ({ repository }: { repository: BackupRepositoryView }) => {
  const Icon = repository.type === BackupRepositoryType.S3Compatible ? Cloud : FolderLock;
  const label = repository.type === BackupRepositoryType.S3Compatible ? 'S3-compatible' : 'Filesystem';

  return (
    <span className="inline-flex items-center gap-2 text-sm">
      <Icon className="size-3.5 text-muted-foreground" />
      {label}
    </span>
  );
};

const DestinationCell = ({ spec }: { spec: BackupRepositorySpec }) => (
  <span className="block max-w-100 truncate text-[13px]" title={destinationText(spec)}>
    {destinationText(spec)}
  </span>
);

export const destinationText = (spec: BackupRepositorySpec) => {
  if (isFileSystemSpec(spec)) {
    return spec.path;
  }

  if (isS3Spec(spec)) {
    return `${spec.endpoint.replace(/\/$/, '')}/${spec.bucket}${spec.prefix ? `/${spec.prefix}` : ''}`;
  }

  return '-';
};

const isFileSystemSpec = (spec: BackupRepositorySpec): spec is BackupRepositorySpecFileSystemBackupRepositorySpec =>
  spec?.$type === 'FileSystem';

const isS3Spec = (spec: BackupRepositorySpec): spec is BackupRepositorySpecS3CompatibleBackupRepositorySpec =>
  spec?.$type === 'S3Compatible';
