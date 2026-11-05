import type { ActivityView, PagedActivityView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { StockFeatures, ColumnDef } from '@tanstack/react-table';
import { useMemo } from 'react';
import { useActivityQuery, useTaskSheet } from '@/lib/atoms';
import { ActorCell, PagedDataTable } from '@/components/custom/common';
import { createResourceTargetColumn } from '@/components/custom/resource-target-column';
import { StateBadge } from '@/components/custom/state-badge';
import { Timestamp } from '@/components/custom/timestamp';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { formatActivityEvent } from '@/lib/utils';

const EMPTY_ROWS: ActivityView[] = [];

type ActivitiesTableProps = {
  pagedResult: PagedActivityView;
  isLoading: boolean;
  displayTarget?: boolean;
  displayPagging?: boolean;
};

export const ActivitiesTable = (props: ActivitiesTableProps) => {
  const [query, setQuery] = useActivityQuery();
  return <ScopedActivitiesTable {...props} query={query} setQuery={setQuery} />;
};

export const ScopedActivitiesTable = ({
  pagedResult,
  isLoading,
  displayTarget = false,
  displayPagging = false,
  query,
  setQuery,
}: ActivitiesTableProps & {
  query: ReturnType<typeof useActivityQuery>[0];
  setQuery: ReturnType<typeof useActivityQuery>[1];
}) => {
  const formatDateTime = useProfileDateTimeFormatter();

  const cols = useMemo(() => columns(displayTarget, formatDateTime), [displayTarget, formatDateTime]);

  return (
    <PagedDataTable
      columns={cols}
      data={pagedResult?.items ?? EMPTY_ROWS}
      isLoading={isLoading}
      query={query}
      setQuery={setQuery}
      totalCount={pagedResult?.totalCount}
      showPagination={displayPagging}
    />
  );
};

const columns = (
  displayTarget: boolean,
  formatDateTime: DateTimeFormatter,
): ColumnDef<StockFeatures, ActivityView>[] => {
  const cols: ColumnDef<StockFeatures, ActivityView>[] = [
    {
      accessorKey: 'eventType',
      header: ({ column }) => <SortableCell cellName="Event Type" column={column} />,
      cell: ({ row }) => <EventTypeCell activity={row.original} />,
      sortFn: (rowA: any, rowB: any): number => rowA.original?.eventType?.localeCompare(rowB.original?.eventType),
    },
  ];

  if (displayTarget) {
    cols.push(
      createResourceTargetColumn({
        id: 'target',
        header: 'Target',
        getValue: (activity: ActivityView) => ({
          target: {
            resourceType: activity.resourceType,
            resourceId: activity.resourceId ?? undefined,
            resourceName: activity.resourceName,
          },
        }),
      }),
    );
  }

  cols.push(
    {
      accessorKey: 'status',
      header: ({ column }) => <SortableCell cellName="Status" column={column} />,
      cell: ({ row }) => <StateBadge value={row.original.status} kind="activity" />,
      sortFn: (rowA, rowB) => (rowA.original.status! < rowB.original.status! ? 1 : -1),
    },
    {
      accessorKey: 'createdAt',
      header: ({ column }) => <SortableCell cellName="Created" column={column} />,
      cell: ({ row }) => {
        return <Timestamp value={row.original.createdAt} formatDateTime={formatDateTime} />;
      },
      sortFn: (rowA, rowB) => (rowA.original.createdAt! < rowB.original.createdAt! ? 1 : -1),
    },
    {
      accessorKey: 'user',
      header: ({ column }) => <SortableCell cellName="User" column={column} />,
      cell: ({ row }) => <ActorCell type={row.original.actorType} name={row.original.actorName ?? ''} />,
      sortFn: (rowA, rowB) => (rowA.original.platformName! < rowB.original.platformName! ? 1 : -1),
    },
  );

  return cols;
};

function EventTypeCell({ activity }: { activity: ActivityView }) {
  const { open } = useTaskSheet('Activity');

  return (
    <button
      type="button"
      onClick={() => open({ kind: 'activity', payload: activity })}
      className="table-link block min-w-0 max-w-96 cursor-pointer overflow-hidden text-left">
      <span className="block truncate">{formatActivityEvent(activity.eventType)}</span>
    </button>
  );
}
