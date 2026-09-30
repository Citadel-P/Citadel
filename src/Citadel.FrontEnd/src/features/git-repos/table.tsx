import { DataTable } from '@/components/ui/data-table';
import {
  AuthorizedGitRepositoryView,
  GitRepositoryStatus,
  ResourceControlState,
} from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { Link } from 'react-router';
import { ActionData } from '@/pages/types';
import { useSelectedResources } from '@/lib/atoms';
import { useMemo } from 'react';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { StateIndicator } from '@/components/custom/state-indicator';
import { TagChips } from '@/features/tags/components';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: AuthorizedGitRepositoryView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
  formatDateTime: DateTimeFormatter,
): ColumnDef<AuthorizedGitRepositoryView>[] => [
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
        aria-label="Select repository"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <RepoNameRow repo={row.original} />,
    sortingFn: (rowA, rowB) => rowA.original.name.localeCompare(rowB.original.name),
  },
  {
    accessorKey: 'url',
    header: ({ column }) => <SortableCell cellName="URL" column={column} />,
    cell: ({ row }) => (
      <span className="text-[13px] text-muted-foreground truncate max-w-75 block">{row.original.url}</span>
    ),
    sortingFn: (rowA, rowB) => rowA.original.url.localeCompare(rowB.original.url),
  },
  {
    accessorKey: 'defaultBranch',
    header: ({ column }) => <SortableCell cellName="Branch" column={column} />,
    cell: ({ row }) => <span className="text-[13px]">{row.original.defaultBranch ?? '-'}</span>,
  },
  {
    accessorKey: 'createdAt',
    header: ({ column }) => <SortableCell cellName="Created" column={column} />,
    cell: ({ row }) => <TimestampCell value={row.original.createdAt} formatDateTime={formatDateTime} />,
    sortingFn: (rowA, rowB) => String(rowA.original.createdAt).localeCompare(String(rowB.original.createdAt)),
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
    cell: ({ row }) => <RowActionMenu resource={row.original as any} actions={actions as any} />,
  },
];

export const GitReposTable = ({
  items,
  actions,
  isLoading,
}: {
  items: AuthorizedGitRepositoryView[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: AuthorizedGitRepositoryView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [_, setSelectedResources] = useSelectedResources<AuthorizedGitRepositoryView>('GitRepository');
  const formatDateTime = useProfileDateTimeFormatter();
  const cols = useMemo(() => columns(actions ?? {}, formatDateTime), [actions, formatDateTime]);

  return <DataTable columns={cols} data={items} isLoading={isLoading} onSelectionChange={setSelectedResources} />;
};

const RepoNameRow = ({ repo }: { repo: AuthorizedGitRepositoryView }) => {
  const status = repo.status ?? GitRepositoryStatus.Unknown;

  return (
    <div className="flex items-center gap-1">
      <StateIndicator
        value={status}
        isProcessing={repo.controlState === ResourceControlState.Processing || status === GitRepositoryStatus.Pending}
      />
      <Link to={`../git-repos/edit/${repo.id}`} className="hover:underline">
        {repo.name}
      </Link>
    </div>
  );
};
