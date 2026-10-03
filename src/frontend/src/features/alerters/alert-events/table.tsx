import { useMemo } from 'react';
import { ColumnDef } from '@tanstack/react-table';
import SortableCell from '@/components/custom/sortable-cell';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { PagedDataTable, TargetCell } from '@/components/custom/common';
import { StateBadge } from '@/components/custom/state-badge';
import type { AlertEventView, AlertEventPage } from '@/api/generated/api.types';
import { useAlertEventQuery, useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { Checkbox } from '@/components/ui/checkbox';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { useOpenAlertEventSheet } from './alert-task-sheet';

const EMPTY_ROWS: AlertEventView[] = [];

export const AlertEventsTable = ({
  pagedResult,
  actions,
  isLoading,
}: {
  pagedResult: AlertEventPage;
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: AlertEventView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [query, setQuery] = useAlertEventQuery();
  const [_, setSelectedResources] = useSelectedResources<AlertEventView>('Alert');
  const formatDateTime = useProfileDateTimeFormatter();
  const cols = useMemo(() => columns(actions ?? {}, formatDateTime), [actions, formatDateTime]);

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
  formatDateTime: DateTimeFormatter,
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
    cell: ({ row }) => <StateBadge value={row.original.severity} kind="alertSeverity" />,
    sortingFn: (rowA, rowB) => rowA.original.severity.localeCompare(rowB.original.severity),
  },
  {
    accessorKey: 'status',
    header: ({ column }) => <SortableCell cellName="Status" column={column} />,
    cell: ({ row }) => <StateBadge value={row.original.status} kind="alertEvent" />,
    sortingFn: (rowA, rowB) => rowA.original.status.localeCompare(rowB.original.status),
  },
  {
    accessorKey: 'createdAt',
    header: ({ column }) => <SortableCell cellName="Created" column={column} />,
    cell: ({ row }) => <TimestampCell value={row.original.createdAt} formatDateTime={formatDateTime} />,
    sortingFn: (rowA, rowB) => String(rowA.original.createdAt).localeCompare(String(rowB.original.createdAt)),
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original as any} actions={actions as any} />,
  },
];

function AlertTypeCell({ event }: { event: AlertEventView }) {
  const openAlertSheet = useOpenAlertEventSheet();

  return (
    <button type="button" onClick={() => openAlertSheet(event.id)} className="cursor-pointer table-link">
      <span>{event.type}</span>
    </button>
  );
}
