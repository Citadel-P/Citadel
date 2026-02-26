import { DataTable } from '@/components/ui/data-table';
import { AlertRuleView, PagedResultViewOfAlertRuleView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { ColumnDef } from '@tanstack/react-table';
import { useRef } from 'react';
import { useActivityQuery } from '@/lib/atoms';
import { ContentCard } from '@/components/custom/content-card';
import { PaginationControls, SelectField, SeverityStatusCell, pageSizeOptions } from '@/components/custom/common';
import { Activity, Clock, Zap } from 'lucide-react';
import { cn } from '@/lib/utils';

const EMPTY_ROWS: AlertRuleView[] = [];

export const AlertRulesTable = ({
  pagedResult,
  isLoading,
  displayPagging = false,
}: {
  pagedResult: PagedResultViewOfAlertRuleView;
  isLoading: boolean;
  displayTarget?: boolean;
  displayPagging?: boolean;
}) => {
  const [query, setQuery] = useActivityQuery();
  const tableTopRef = useRef<HTMLDivElement | null>(null);

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
        <DataTable columns={columns()} data={pagedResult?.items ?? EMPTY_ROWS} isLoading={isLoading} />
      </ContentCard>
      {displayPagging && (
        <div className="flex sm:flex-row flex-col gap-2 sm:items-center sm:justify-between">
          <PaginationControls
            currentPage={query.page}
            totalPages={totalPages}
            onPageChange={goToPage}
            className="justify-start"
          />
          {totalPages > 1 && (
            <SelectField
              value={query.pageSize.toString()}
              options={pageSizeOptions}
              onChange={handlePageSizeChange}
              placeholder="Page Size"
              allLabel="Page Size"
              selectableLabel={false}
            />
          )}
        </div>
      )}
    </div>
  );
};

const columns = (): ColumnDef<AlertRuleView>[] => {
  const cols: ColumnDef<AlertRuleView>[] = [
    {
      accessorKey: 'status',
      header: ({ column }) => <SortableCell cellName="Status" column={column} />,
      cell: ({ row }) => row.original.isEnabled,
      sortingFn: (rowA: any, rowB: any): number => rowA.original?.eventType?.localeCompare(rowB.original?.eventType),
    },
    {
      accessorKey: 'type',
      header: ({ column }) => <SortableCell cellName="Rule Type" column={column} />,
      cell: ({ row }) => row.original.type,
      sortingFn: (rowA: any, rowB: any): number => rowA.original?.type?.localeCompare(rowB.original?.type),
    },
    {
      accessorKey: 'severity',
      header: ({ column }) => <SortableCell cellName="Severity" column={column} />,
      cell: ({ row }) => <SeverityStatusCell severity={row.original.severity} />,
      sortingFn: (rowA: any, rowB: any): number => rowA.original?.severity?.localeCompare(rowB.original?.severity),
    },
    {
      accessorKey: 'conditions',
      header: ({ column }) => <SortableCell cellName="Conditions" column={column} />,
      cell: ({ row }) => <RuleConditionCell rule={row.original} />,
      sortingFn: (rowA: any, rowB: any): number => rowA.original?.threshold?.localeCompare(rowB.original?.threshold),
    },
    {
      accessorKey: 'channels',
      header: ({ column }) => <SortableCell cellName="Channels" column={column} />,
      cell: ({ row }) => <ChannelsCell rule={row.original} />,
      sortingFn: (rowA: any, rowB: any): number => rowA.original?.threshold?.localeCompare(rowB.original?.threshold),
    },
  ];

  return cols;
};

function RuleConditionCell({ rule }: { rule: AlertRuleView }) {
  return (
    <div className="py-3">
      <div className="flex flex-col gap-1.5">
        {rule.threshold ? (
          <div className="flex items-center gap-2 ">
            <Activity className="h-3.5 w-3.5 text-muted-foreground" />
            <span className="font-medium text-xs">
              &gt; {rule.threshold}%<span className="text-muted-foreground font-normal mx-1">for</span>
              {rule.requiredMatches}x
            </span>
          </div>
        ) : (
          <div className="flex items-center gap-2 text-muted-foreground">
            <Zap className="h-3.5 w-3.5" />
            <span className="text-xs">Event Trigger</span>
          </div>
        )}
        {rule.cooldownSeconds && (
          <div className="flex items-center gap-2 text-muted-foreground">
            <Clock className="h-3.5 w-3.5 text-muted-foreground" />
            <span className="text-xs">{rule.cooldownSeconds}s cooldown</span>
          </div>
        )}
      </div>
    </div>
  );
}

function ChannelsCell({ rule }: { rule: AlertRuleView }) {
  return (
    <div className="flex -space-x-2 hover:space-x-1 transition-all duration-300">
      {rule.channels.map((c, i) => (
        <div
          key={i}
          className={cn(
            'h-8 w-8 rounded-full border-2 border-white flex items-center justify-center text-white text-[10px] font-bold shadow-sm relative z-10 transition-transform hover:scale-110 hover:z-20 cursor-help',
            c.alertDestination === 'Slack'
              ? 'bg-[#4A154B]'
              : c.alertDestination === 'Discord'
                ? 'bg-[#5865F2]'
                : c.alertDestination === 'Generic'
                  ? 'bg-blue-600'
                  : 'bg-zinc-900',
          )}
          title={`${c.alertDestination}: ${c.url}`}>
          {c.alertDestination[0]}
        </div>
      ))}
      {rule.channels.length === 0 && (
        <span className="text-xs text-muted-foreground italic py-1 px-2 ">No channels</span>
      )}
    </div>
  );
}
