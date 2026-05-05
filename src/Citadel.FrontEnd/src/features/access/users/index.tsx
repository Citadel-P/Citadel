import { PagedResultViewOfUserView, UserView } from '@/api/generated/api.types';
import { PagedDataTable } from '@/components/custom/common';
import SortableCell from '@/components/custom/sortable-cell';
import { useSelectedResources, useUserQuery } from '@/lib/atoms';
import { ActionData, DropdownActionComponent } from '@/pages/types';
import { ColumnDef } from '@tanstack/react-table';
import { useMemo } from 'react';
import { Checkbox } from '@/components/ui/checkbox';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Link } from 'react-router';

export const Users = ({
  items,
  actions,
  isLoading,
}: {
  items: PagedResultViewOfUserView;
  actions: Record<string, DropdownActionComponent>;
  isLoading: boolean;
}) => <UsersTable pagedResult={items} isLoading={isLoading} actions={actions} />;

const EMPTY_ROWS: UserView[] = [];

export const UsersTable = ({
  pagedResult,
  actions,
  isLoading,
}: {
  pagedResult: PagedResultViewOfUserView | undefined;
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: UserView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [query, setQuery] = useUserQuery();
  const [_, setSelectedResources] = useSelectedResources<UserView>('User');
  const cols = useMemo(() => columns(actions ?? {}), [actions]);

  return (
    <PagedDataTable
      columns={cols}
      data={pagedResult?.items ?? EMPTY_ROWS}
      isLoading={isLoading}
      query={query}
      setQuery={setQuery}
      totalCount={pagedResult?.totalCount}
      onSelectionChange={setSelectedResources}
    />
  );
};

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: UserView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
): ColumnDef<UserView>[] => [
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
        aria-label="Select user"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <UserNameRow user={row.original} />,
    sortingFn: (rowA, rowB) => String(rowA.original.name).localeCompare(String(rowB.original.name)),
  },
  {
    accessorKey: 'email',
    header: ({ column }) => <SortableCell cellName="Email" column={column} />,
    cell: ({ row }) => <>{row.original.email}</>,
    sortingFn: (rowA, rowB) => String(rowA.original.email).localeCompare(String(rowB.original.email)),
  },
  {
    accessorKey: 'teams',
    header: ({ column }) => <SortableCell cellName="Teams" column={column} />,
    cell: ({ row }) => <span className="text-[13px]">{row.original.teams?.join(',')}</span>,
  },
  {
    accessorKey: 'roles',
    header: ({ column }) => <SortableCell cellName="Roles" column={column} />,
    cell: ({ row }) => <span className="text-[13px]">{row.original.roles?.join(',')}</span>,
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

const UserNameRow = ({ user }: { user: UserView }) => {
  return (
    <div className="flex items-center gap-1">
      <StateIndicator enableLabel={user.isEnabled} value={user.isEnabled} />
      <Link to={`../access/users/edit/${user.id}`} className="hover:underline">
        {user.name}
      </Link>
    </div>
  );
};
