import { useMemo } from 'react';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { Badge } from '@/components/ui/badge';
import SortableCell from '@/components/custom/sortable-cell';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { PagedDataTable, SeverityStatusCell, TargetCell } from '@/components/custom/common';
import { AlertEventStatus, AlertEventView, PagedResultViewOfAlertEventView } from '@/api/generated/api.types';
import { useAlertEventQuery, useSelectedResources, useTaskSheet } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { fromNow } from '@/lib/dayjs.helper';

const EMPTY_ROWS: AlertEventView[] = [];

export const AlertEventsTable = ({
  pagedResult,
  actions,
  isLoading,
}: {
  pagedResult: PagedResultViewOfAlertEventView;
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: AlertEventView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [query, setQuery] = useAlertEventQuery();
  const [_, setSelectedResources] = useSelectedResources<AlertEventView>('Alert');
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
    React.FC<{ resource: AlertEventView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
): ColumnDef<AlertEventView>[] => [
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
    accessorKey: 'type',
    header: ({ column }) => <SortableCell cellName="Alert Type" column={column} />,
    cell: ({ row }) => <AlertTypeCell event={row.original} />,
    sortingFn: (rowA, rowB) => rowA.original.type.localeCompare(rowB.original.type),
  },
  {
    accessorKey: 'target',
    header: ({ column }) => <SortableCell cellName="Target" column={column} />,
    cell: ({ row }) => (
      <TargetCell
        resourceType={row.original.resourceType}
        resourceId={row.original.resourceId!}
        resourceName={row.original.resourceName}
      />
    ),
    sortingFn: (rowA, rowB) => (rowA.original.resourcePath ?? '').localeCompare(rowB.original.resourcePath ?? ''),
  },
  {
    accessorKey: 'severity',
    header: ({ column }) => <SortableCell cellName="Severity" column={column} />,
    cell: ({ row }) => <SeverityStatusCell severity={row.original.severity} />,
    sortingFn: (rowA, rowB) => rowA.original.severity.localeCompare(rowB.original.severity),
  },
  {
    accessorKey: 'status',
    header: ({ column }) => <SortableCell cellName="Status" column={column} />,
    cell: ({ row }) => <AlertEventStatusCell status={row.original.status} />,
    sortingFn: (rowA, rowB) => rowA.original.status.localeCompare(rowB.original.status),
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

function AlertTypeCell({ event }: { event: AlertEventView }) {
  const { open } = useTaskSheet('Alert');

  return (
    <button
      type="button"
      onClick={() => open({ kind: 'alertEvent', payload: event })}
      className="cursor-pointer table-link">
      <span>{event.type}</span>
    </button>
  );
}

function AlertEventStatusCell({ status }: { status: AlertEventStatus }) {
  const config: Record<AlertEventStatus, { label: string; className: string }> = {
    [AlertEventStatus.Active]: { label: 'Active', className: 'bg-red-200/25 text-red-700 border-red-500/20' },
    [AlertEventStatus.Acknowledged]: {
      label: 'Acknowledged',
      className: 'bg-orange-200/25 text-orange-600 border-orange-500/20',
    },
    [AlertEventStatus.Resolved]: {
      label: 'Resolved',
      className: 'bg-green-200/25 text-green-700 border-green-500/20',
    },
  };

  const value = config[status];
  return <Badge className={value.className}>{value.label}</Badge>;
}
