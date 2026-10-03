import { useCallback, useMemo } from 'react';
import { Link } from 'react-router';
import { ColumnDef, Row } from '@tanstack/react-table';
import { ChevronDown, ChevronRight } from 'lucide-react';
import { ManagedSwarmServiceView, ResourceControlState, SwarmTaskView } from '@/api/generated/api.types';
import { PlatformStatusCell } from '@/components/custom/common';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import SortableCell from '@/components/custom/sortable-cell';
import { StateBadge } from '@/components/custom/state-badge';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable } from '@/components/ui/data-table';
import { TagChips } from '@/features/tags/components';
import { useSelectedResources } from '@/lib/atoms';
import { getTaskName } from '@/lib/utils';
import { truncate } from '@/lib/truncate';
import { ActionData } from '@/pages/types';

type SwarmServiceActions = Record<
  string,
  React.FC<{ resource: ManagedSwarmServiceView; onAction?: (key: string, data?: ActionData) => void }>
>;

type ManagedServiceRow = {
  id: string;
  kind: 'service';
  service: ManagedSwarmServiceView;
  tasks: ManagedTaskRow[];
};

type ManagedTaskRow = {
  id: string;
  kind: 'task';
  platformId: string;
  task: SwarmTaskView;
};

type ManagedServiceTableRow = ManagedServiceRow | ManagedTaskRow;

const isServiceRow = (row: ManagedServiceTableRow): row is ManagedServiceRow => row.kind === 'service';

export const SwarmServicesTable = ({
  items,
  actions,
  isLoading,
  isFiltered,
}: {
  items: ManagedSwarmServiceView[];
  isLoading: boolean;
  isFiltered?: boolean;
  actions: SwarmServiceActions;
}) => {
  const [, setSelected] = useSelectedResources<ManagedSwarmServiceView>('SwarmService');
  const rows = useMemo<ManagedServiceTableRow[]>(
    () =>
      items.map((service) => ({
        id: `service:${service.id}`,
        kind: 'service',
        service,
        tasks: (service.tasks ?? []).map((task) => ({
          id: `task:${service.platformId}:${task.id}`,
          kind: 'task',
          platformId: service.platformId,
          task,
        })),
      })),
    [items],
  );
  const columns = useMemo(() => createColumns(actions), [actions]);
  const getSubRows = useCallback((row: ManagedServiceTableRow) => (isServiceRow(row) ? row.tasks : undefined), []);
  const onSelectionChange = useCallback(
    (selectedRows: ManagedServiceTableRow[]) =>
      setSelected(selectedRows.filter(isServiceRow).map((row) => row.service)),
    [setSelected],
  );

  return (
    <DataTable
      columns={columns}
      data={rows}
      isLoading={isLoading}
      getSubRows={getSubRows}
      enableRowSelection={isServiceRow}
      enableSubRowSelection={false}
      onSelectionChange={onSelectionChange}
      emptyState={
        isFiltered
          ? {
              title: 'No services match the current filters.',
              description: 'Select another overview card or adjust the search and filters.',
            }
          : { title: 'No managed Services found.' }
      }
    />
  );
};

const createColumns = (actions: SwarmServiceActions): ColumnDef<ManagedServiceTableRow>[] => [
  {
    id: 'select',
    header: ({ table }) => (
      <Checkbox
        checked={table.getIsAllPageRowsSelected() || (table.getIsSomePageRowsSelected() && 'indeterminate')}
        onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
        aria-label="Select all Services"
      />
    ),
    cell: ({ row }) =>
      isServiceRow(row.original) ? (
        <Checkbox
          checked={row.getIsSelected()}
          onCheckedChange={(value) => row.toggleSelected(!!value)}
          aria-label="Select Service"
        />
      ) : null,
    enableSorting: false,
    enableHiding: false,
  },
  {
    id: 'name',
    accessorFn: (row) => (isServiceRow(row) ? row.service.name : getTaskName(row.task)),
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <ServiceNameCell row={row} />,
  },
  {
    id: 'image',
    accessorFn: (row) => (isServiceRow(row) ? getServiceImage(row.service) : row.task.image),
    header: ({ column }) => <SortableCell cellName="Image" column={column} />,
    cell: ({ row }) => {
      const value = isServiceRow(row.original) ? getServiceImage(row.original.service) : row.original.task.image;
      return (
        <span className="block max-w-72 truncate" title={value ?? undefined}>
          {value ?? '<not built>'}
        </span>
      );
    },
  },
  {
    id: 'replicas',
    header: 'Mode / Replicas',
    cell: ({ row }) => {
      if (!isServiceRow(row.original)) return <StateBadge value={row.original.task.desiredState} kind="swarmTask" />;
      const service = row.original.service;
      return (
        <span className="whitespace-nowrap">
          {service.spec.schedulingMode === 'Global'
            ? 'Global'
            : `${service.runningTaskCount ?? 0}/${service.desiredTaskCount ?? service.spec.replicas ?? 0}`}
        </span>
      );
    },
  },
  {
    id: 'node',
    header: 'Node',
    cell: ({ row }) => {
      if (isServiceRow(row.original)) return '-';
      const { platformId, task } = row.original;
      return task.nodeId ? (
        <Link className="table-link" to={`/platforms/${platformId}/nodes/${task.nodeId}`}>
          {task.nodeHostname || task.nodeId.slice(0, 12)}
        </Link>
      ) : (
        '-'
      );
    },
  },
  {
    id: 'platformName',
    accessorFn: (row) => (isServiceRow(row) ? row.service.platformName : ''),
    header: ({ column }) => <SortableCell cellName="Platform" column={column} />,
    cell: ({ row }) =>
      isServiceRow(row.original) ? (
        <PlatformStatusCell
          status={row.original.service.platformStatus}
          id={row.original.service.platformId}
          name={row.original.service.platformName ?? ''}
        />
      ) : (
        '-'
      ),
  },
  {
    id: 'tags',
    header: 'Tags',
    cell: ({ row }) => (isServiceRow(row.original) ? <TagChips tags={row.original.service.tags} /> : null),
    enableSorting: false,
  },
  {
    id: 'actions',
    cell: ({ row }) =>
      isServiceRow(row.original) ? <RowActionMenu resource={row.original.service} actions={actions} /> : null,
  },
];

const ServiceNameCell = ({ row }: { row: Row<ManagedServiceTableRow> }) => {
  if (!isServiceRow(row.original)) {
    const { platformId, task } = row.original;
    return (
      <div className="flex min-w-0 items-center gap-2 pl-7">
        <StateIndicator value={task.state} kind="swarmTask" />
        <Link
          className="table-link truncate"
          to={`/platforms/${platformId}/tasks/${task.id}`}
          title={getTaskName(task)}>
          {getTaskName(task)}
        </Link>
        {task.isStale && <Badge variant="secondary">Stale</Badge>}
      </div>
    );
  }

  const service = row.original.service;
  const expanded = row.getIsExpanded();
  return (
    <div className="flex min-w-0 items-center gap-2 whitespace-nowrap">
      {row.getCanExpand() ? (
        <button
          type="button"
          onClick={row.getToggleExpandedHandler()}
          className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-muted-foreground hover:bg-accent/70 hover:text-foreground"
          aria-label={expanded ? `Collapse service ${service.name}` : `Expand service ${service.name}`}>
          {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
        </button>
      ) : (
        <span className="h-5 w-5 shrink-0" />
      )}
      <StateIndicator value={service.health} isProcessing={service.controlState === ResourceControlState.Processing} />
      <Link className="table-link truncate" to={`/swarm-services/edit/${service.id}`} title={service.name}>
        {truncate(service.name, 32, 'right')}
      </Link>
    </div>
  );
};

const getServiceImage = (service: ManagedSwarmServiceView) =>
  service.spec.image.$type === 'External' ? service.spec.image.imageTag : service.spec.image.resolvedImageReference;
