import { DataTable } from '@/components/ui/data-table';
import { ActivityView, PagedResultViewOfActivityView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { ColumnDef } from '@tanstack/react-table';
import { useMemo, useRef } from 'react';
import { useActivityQuery, useTaskSheet } from '@/lib/atoms';
import { ContentCard } from '@/components/custom/content-card';
import {
  ActorCell,
  ActivityStatusCell,
  PaginationControls,
  TargetCell,
  SelectField,
  pageSizeOptions,
} from '@/components/custom/common';

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
  const tableTopRef = useRef<HTMLDivElement | null>(null);

  const cols = useMemo(() => columns(displayTarget), [displayTarget]);

  const totalCount = Number(pagedResult?.totalCount ?? 0);
  const totalPages = Math.max(1, Math.ceil(totalCount / query.pageSize));

  const goToPage = (page: number) => {
    if (page < 1 || page > totalPages || page === query.page) return;
    setQuery({ page });
    requestAnimationFrame(() => {
      tableTopRef.current?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    });
  };

  const handlePageSizeChange = (value: string) => {
    setQuery({
      ...query,
      pageSize: Number(value),
    });
  };

  return (
    <div className="flex flex-col gap-4" ref={tableTopRef}>
      <ContentCard>
        <DataTable columns={cols} data={pagedResult?.items ?? EMPTY_ROWS} isLoading={isLoading} />
      </ContentCard>
      {displayPagging && (
        <div className="flex sm:flex-row flex-col gap-2 sm:items-center sm:justify-between">
          <PaginationControls
            currentPage={query.page}
            totalPages={totalPages}
            onPageChange={goToPage}
            className="justify-start"
          />
          {totalPages > 1 && <SelectField
            value={query.pageSize.toString()}
            options={pageSizeOptions}
            onChange={handlePageSizeChange}
            placeholder="Page Size"
            allLabel="Page Size"
            selectableLabel={false}
          />}
        </div>
      )}
    </div>
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
      header: ({ column }) => <SortableCell cellName="Start Time" column={column} />,
      cell: ({ row }) => {
        return <span>{new Date(row.original.createdAt).toLocaleString()}</span>;
      },
      sortingFn: (rowA, rowB) => (rowA.original.createdAt! < rowB.original.createdAt! ? 1 : -1),
    },
    {
      accessorKey: 'user',
      header: ({ column }) => <SortableCell cellName="User" column={column} />,
      cell: ({ row }) => (
        <ActorCell type={row.original.actorType} id={row.original.actorId ?? ''} name={row.original.actorName ?? ''} />
      ),
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
