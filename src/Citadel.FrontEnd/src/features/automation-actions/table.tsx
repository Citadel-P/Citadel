import { AutomationActionView, ResourceControlState } from '@/api/generated/api.types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable } from '@/components/ui/data-table';
import { fromNow } from '@/lib/dayjs.helper';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { ColumnDef } from '@tanstack/react-table';
import { CalendarClock, PlayCircle } from 'lucide-react';
import { useMemo } from 'react';
import { Link } from 'react-router';
import { TagChips } from '../tags/components';

export function AutomationActionsTable({
  items,
  actions,
  isLoading,
}: {
  items: AutomationActionView[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: AutomationActionView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) {
  const [, setSelectedResources] = useSelectedResources<AutomationActionView>('AutomationAction');
  const cols = useMemo(() => columns(actions ?? {}), [actions]);

  return <DataTable columns={cols} data={items} isLoading={isLoading} onSelectionChange={setSelectedResources} />;
}

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: AutomationActionView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
): ColumnDef<AutomationActionView>[] => [
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
    sortingFn: (rowA, rowB) => rowA.original.name.localeCompare(rowB.original.name),
  },
  {
    accessorKey: 'schedule',
    header: ({ column }) => <SortableCell cellName="Schedule" column={column} />,
    cell: ({ row }) => <ScheduleCell action={row.original} />,
    sortingFn: (rowA, rowB) => Number(rowA.original.scheduleEnabled) - Number(rowB.original.scheduleEnabled),
  },
  {
    accessorKey: 'latestRun',
    header: ({ column }) => <SortableCell cellName="Last Run" column={column} />,
    cell: ({ row }) => <LastRunCell action={row.original} />,
    sortingFn: (rowA, rowB) =>
      String(rowA.original.latestRun?.queuedAt ?? '').localeCompare(String(rowB.original.latestRun?.queuedAt ?? '')),
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
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

const ActionNameRow = ({ action }: { action: AutomationActionView }) => {
  const status = action.enabled ? (action.latestRun?.status ?? true) : false;

  return (
    <div className="flex min-w-0 items-center gap-1">
      <StateIndicator
        value={status}
        isProcessing={action.controlState === ResourceControlState.Processing}
        enableLabel={typeof status === 'boolean'}
        kind={typeof status === 'boolean' ? undefined : 'automationActionRun'}
      />
      <Link to={`../automation/edit/${action.id}`} title={action.name} className="truncate text-sm hover:underline">
        {action.name}
      </Link>
    </div>
  );
};

const ScheduleCell = ({ action }: { action: AutomationActionView }) => {
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

const LastRunCell = ({ action }: { action: AutomationActionView }) => {
  const run = action.latestRun;
  if (!run) return <span className="text-sm text-muted-foreground">No runs</span>;

  return (
    <span className="inline-flex items-center gap-2 text-sm">
      <PlayCircle className="size-3.5 text-muted-foreground" />
      <span>{fromNow(run.queuedAt)}</span>
    </span>
  );
};
