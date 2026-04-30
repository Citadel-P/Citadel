import { PagedResultViewOfUserView, UserView } from '@/api/generated/api.types';
import { PagedDataTable } from '@/components/custom/common';
import SortableCell from '@/components/custom/sortable-cell';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { ColumnDef } from '@tanstack/react-table';
import { useMemo } from 'react';
import { Checkbox } from '@/components/ui/checkbox';
import { fromNow } from '@/lib/dayjs.helper';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredFormComponents } from '@/pages/types';

export const Users = () => {
  return <div>Users</div>;
};

export const UserFormComponents: RequiredFormComponents = {
  AddForm: {
    Header: {
      title: 'User',
    },
    Content: () => <></>,
  },
};

const EMPTY_ROWS: UserView[] = [];

export const ActivitiesTable = ({
  pagedResult,
  actions,
  isLoading,
}: {
  pagedResult: PagedResultViewOfUserView;
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: UserView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [query, setQuery] = useAlertEventQuery();
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
        aria-label="Select alert event"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },

  {
    accessorKey: 'status',
    header: ({ column }) => <SortableCell cellName="Status" column={column} />,
    cell: ({ row }) => <StateIndicator value={row.original.isEnabled} enableLabel />,
    sortingFn: (rowA, rowB) => Number(rowA.original.isEnabled) - Number(rowB.original.isEnabled),
  },
  {
    accessorKey: 'createdAt',
    header: ({ column }) => <SortableCell cellName="Created" column={column} />,
    cell: ({ row }) => <span className="text-[13px]">{fromNow(row.original.createdAt)}</span>,
    sortingFn: (rowA, rowB) => String(rowA.original.createdAt).localeCompare(String(rowB.original.createdAt)),
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original as any} actions={actions as any} />,
  },
];
