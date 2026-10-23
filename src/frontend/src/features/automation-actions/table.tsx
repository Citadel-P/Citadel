import { AutomationRunStatus, AuthorizedAction, ResourceControlState } from '@/api/generated/api.types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { isFailedRunStatus } from '@/components/custom/state-badge';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable } from '@/components/ui/data-table';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { fromNow } from '@/lib/dayjs.helper';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { StockFeatures, ColumnDef } from '@tanstack/react-table';
import { CalendarClock, PlayCircle } from 'lucide-react';
import { useMemo } from 'react';
import { Link } from 'react-router';
import { TagChips } from '../tags/components';

export function AutomationActionsTable({
  items,
  actions,
  isLoading,
}: {
  items: AuthorizedAction[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: AuthorizedAction; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) {
  const [, setSelectedResources] = useSelectedResources<AuthorizedAction>('AutomationAction');
  const cols = useMemo(() => columns(actions ?? {}), [actions]);

  return <DataTable columns={cols} data={items} isLoading={isLoading} onSelectionChange={setSelectedResources} />;
}

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: AuthorizedAction; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
): ColumnDef<StockFeatures, AuthorizedAction>[] => [
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
        aria-label="Select automation action"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <ActionNameRow action={row.original} />,
    sortFn: (rowA, rowB) => rowA.original.name.localeCompare(rowB.original.name),
  },
  {
    accessorKey: 'schedule',
    header: ({ column }) => <SortableCell cellName="Schedule" column={column} />,
    cell: ({ row }) => <ScheduleCell action={row.original} />,
    sortFn: (rowA, rowB) => Number(rowA.original.scheduleEnabled) - Number(rowB.original.scheduleEnabled),
  },
  {
    accessorKey: 'latestRun',
    header: ({ column }) => <SortableCell cellName="Last Run" column={column} />,
    cell: ({ row }) => <LastRunCell action={row.original} />,
    sortFn: (rowA, rowB) =>
      String(rowA.original.latestRun?.queuedAt ?? '').localeCompare(String(rowB.original.latestRun?.queuedAt ?? '')),
  },
  {
    accessorKey: 'tags',
    header: ({ column }) => <SortableCell cellName="Tags" column={column} />,
    cell: ({ row }) => <TagChips tags={row.original.tags} />,
    sortFn: (rowA, rowB) =>
      (rowA.original.tags?.map((tag) => tag.name).join(',') ?? '').localeCompare(
        rowB.original.tags?.map((tag) => tag.name).join(',') ?? '',
      ),
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

const ActionNameRow = ({ action }: { action: AuthorizedAction }) => {
  return (
    <div className="flex min-w-0 items-center gap-1">
      <StateIndicator
        value={action.enabled}
        isProcessing={action.controlState === ResourceControlState.Processing}
        enableLabel
      />
      <Link to={`../automation/edit/${action.id}`} title={action.name} className="truncate text-sm hover:underline">
        {action.name}
      </Link>
    </div>
  );
};

const ScheduleCell = ({ action }: { action: AuthorizedAction }) => {
  if (!action.scheduleEnabled) return <span className="text-sm text-muted-foreground">Off</span>;

  return (
    <span className="inline-flex max-w-60 items-center gap-2 truncate text-sm">
      <CalendarClock className="size-3.5 shrink-0 text-muted-foreground" />
      <span className="truncate" title={action.scheduleCron ?? undefined}>
        {action.scheduleCron}
      </span>
    </span>
  );
};

const LastRunCell = ({ action }: { action: AuthorizedAction }) => {
  const run = action.latestRun;
  if (!run) return <span className="text-sm text-muted-foreground">No runs</span>;
  const status = run.status === AutomationRunStatus.TimedOut ? 'Timed out' : run.status;
  const iconColor = isFailedRunStatus(run.status)
    ? 'text-red-500'
    : run.status === AutomationRunStatus.Succeeded
      ? 'text-green-500'
      : 'text-muted-foreground';

  return (
    <div className="inline-flex items-center gap-2 text-sm">
      <Tooltip>
        <TooltipTrigger asChild>
          <span role="img" aria-label={`Last run: ${status}`} className="inline-flex">
            <PlayCircle aria-hidden="true" className={`size-3.5 ${iconColor}`} />
          </span>
        </TooltipTrigger>
        <TooltipContent>{status}</TooltipContent>
      </Tooltip>
      <span className="whitespace-nowrap">{fromNow(run.queuedAt)}</span>
    </div>
  );
};
