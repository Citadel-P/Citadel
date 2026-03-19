import { ActivityView, PagedResultViewOfActivityView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { ColumnDef } from '@tanstack/react-table';
import { useMemo } from 'react';
import { useActivityQuery, useTaskSheet } from '@/lib/atoms';
import { ActorCell, ActivityStatusCell, PagedDataTable, TargetCell } from '@/components/custom/common';
import { fromNow } from '@/lib/dayjs.helper';

const EMPTY_ROWS: ActivityView[] = [];

export const ActivitiesTable = ({
  pagedResult,
  isLoading,
  displayTarget = false,
  displayPagging = false,
}: {
  pagedResult: PagedResultViewOfActivityView;
  isLoading: boolean;
  displayTarget?: boolean;
  displayPagging?: boolean;
}) => {
  const [query, setQuery] = useActivityQuery();

  const cols = useMemo(() => columns(displayTarget), [displayTarget]);

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

const columns = (displayTarget: boolean): ColumnDef<ActivityView>[] => {
  const cols: ColumnDef<ActivityView>[] = [
    {
      accessorKey: 'eventType',
      header: ({ column }) => <SortableCell cellName="Event Type" column={column} />,
      cell: ({ row }) => <EventTypeCell activity={row.original} />,
      sortingFn: (rowA: any, rowB: any): number => rowA.original?.eventType?.localeCompare(rowB.original?.eventType),
    },
  ];

  if (displayTarget) {
    cols.push({
      accessorKey: 'target',
      header: ({ column }) => <SortableCell cellName="Target" column={column} />,
      cell: ({ row }) => (
        <TargetCell
          resourceType={row.original.resourceType}
          resourceId={row.original.resourceId!}
          resourceName={row.original.resourceName}
        />
      ),
      sortingFn: (rowA, rowB) => (rowA.original.platformName! < rowB.original.platformName! ? 1 : -1),
    });
  }

  cols.push(
    {
      accessorKey: 'status',
      header: ({ column }) => <SortableCell cellName="Status" column={column} />,
      cell: ({ row }) => <ActivityStatusCell status={row.original.status} />,
      sortingFn: (rowA, rowB) => (rowA.original.status! < rowB.original.status! ? 1 : -1),
    },
    {
      accessorKey: 'createdAt',
      header: ({ column }) => <SortableCell cellName="Created" column={column} />,
      cell: ({ row }) => {
        return <span className='text-[13px]'>{fromNow(row.original.createdAt)}</span>;
      },
      sortingFn: (rowA, rowB) => (rowA.original.createdAt! < rowB.original.createdAt! ? 1 : -1),
    },
    {
      accessorKey: 'user',
      header: ({ column }) => <SortableCell cellName="User" column={column} />,
      cell: ({ row }) => <ActorCell type={row.original.actorType} name={row.original.actorName ?? ''} />,
      sortingFn: (rowA, rowB) => (rowA.original.platformName! < rowB.original.platformName! ? 1 : -1),
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
      className="cursor-pointer table-link">
      <span>{activity.eventType}</span>
    </button>
  );
}
