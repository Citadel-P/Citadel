import { SwarmTaskView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { StateBadge } from '@/components/custom/state-badge';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable } from '@/components/ui/data-table';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { getTaskName } from '@/lib/utils';
import { useSelectedResources } from '@/lib/atoms';
import { DropdownActionComponent } from '@/pages/types';
import { ColumnDef, Row } from '@tanstack/react-table';
import { ChevronDown, ChevronRight } from 'lucide-react';
import { useCallback, useMemo } from 'react';
import { Link, useParams } from 'react-router';
import { getServiceAvailability, isServiceUpdatePaused, swarmOwnershipLabel } from '../shared';
import { SwarmServiceListView } from './hooks/useServicesGroup';
import { UnmanagedResourceIcon } from '@/components/custom/common';
import { getServiceViewRoute } from './actions';

type ServiceRow = {
  id: string;
  kind: 'service';
  service: SwarmServiceListView;
  tasks: TaskRow[];
};

type TaskRow = {
  id: string;
  kind: 'task';
  task: SwarmTaskView;
};

type ServiceTableRow = ServiceRow | TaskRow;

const isServiceRow = (row: ServiceTableRow): row is ServiceRow => row.kind === 'service';

export const ServicesTable = ({
  items,
  isLoading,
  actions,
}: {
  items: SwarmServiceListView[];
  isLoading: boolean;
  actions: Record<string, DropdownActionComponent<SwarmServiceListView>>;
}) => {
  const { platformId = '' } = useParams<{ platformId: string }>();
  const [, setSelectedResources] = useSelectedResources<SwarmServiceListView>('Service');
  const rows = useMemo<ServiceTableRow[]>(
    () =>
      items.map((service) => ({
        id: `service:${service.id}`,
        kind: 'service',
        service,
        tasks: service.tasks.map((task) => ({ id: `task:${task.id}`, kind: 'task', task })),
      })),
    [items],
  );
  const getSubRows = useCallback((row: ServiceTableRow) => (isServiceRow(row) ? row.tasks : undefined), []);
  const columns = useMemo<ColumnDef<ServiceTableRow>[]>(
    () => [
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
              aria-label={`Select Service ${row.original.service.name}`}
            />
          ) : null,
        enableSorting: false,
        enableHiding: false,
      },
      {
        id: 'name',
        accessorFn: (row) => (isServiceRow(row) ? row.service.name : getTaskName(row.task)),
        header: ({ column }) => <SortableCell cellName="Service" column={column} />,
        cell: ({ row }) => <ServiceNameCell row={row} platformId={platformId} />,
      },
      {
        id: 'mode',
        accessorFn: (row) => (isServiceRow(row) ? row.service.mode : 'Task'),
        header: ({ column }) => <SortableCell cellName="Mode" column={column} />,
        cell: ({ row }) => (isServiceRow(row.original) ? row.original.service.mode : 'Task'),
      },
      {
        id: 'runtime',
        header: 'Runtime',
        cell: ({ row }) =>
          isServiceRow(row.original) ? (
            <div className="flex items-center gap-2">
              <span>
                {row.original.service.runningTaskCount}/{row.original.service.desiredTaskCount} replicas
              </span>
              {isServiceUpdatePaused(row.original.service.updateState) && (
                <StateBadge value={row.original.service.updateState} />
              )}
            </div>
          ) : (
            <StateBadge value={row.original.task.desiredState} kind="swarmTask" />
          ),
      },
      {
        id: 'node',
        header: 'Node',
        cell: ({ row }) => {
          if (isServiceRow(row.original)) return '-';
          const task = row.original.task;
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
        id: 'image',
        accessorFn: (row) => (isServiceRow(row) ? row.service.image : row.task.image),
        header: 'Image',
        cell: ({ row }) => {
          const image = isServiceRow(row.original) ? row.original.service.image : row.original.task.image;
          return (
            <span className="block max-w-72 truncate" title={image}>
              {image}
            </span>
          );
        },
      },
      {
        id: 'ownership',
        accessorFn: (row) => (isServiceRow(row) ? swarmOwnershipLabel(row.service.ownership) : ''),
        header: ({ column }) => <SortableCell cellName="Ownership" column={column} />,
        cell: ({ row }) =>
          isServiceRow(row.original) ? (
            <span title={row.original.service.ownershipDiagnostic ?? undefined}>
              {swarmOwnershipLabel(row.original.service.ownership)}
              {row.original.service.ownershipDiagnostic ? ' (orphaned)' : ''}
            </span>
          ) : (
            '-'
          ),
      },
      {
        id: 'actions',
        cell: ({ row }) =>
          isServiceRow(row.original) ? <RowActionMenu resource={row.original.service} actions={actions} /> : null,
      },
    ],
    [actions, platformId],
  );

  return (
    <DataTable
      columns={columns}
      data={rows}
      isLoading={isLoading}
      getSubRows={getSubRows}
      enableRowSelection={(row) => isServiceRow(row)}
      enableSubRowSelection={false}
      onSelectionChange={(selected) => setSelectedResources(selected.filter(isServiceRow).map((row) => row.service))}
      emptyState={{ title: 'No services found.', description: 'No services were returned by the Swarm manager.' }}
    />
  );
};

const ServiceNameCell = ({ row, platformId }: { row: Row<ServiceTableRow>; platformId: string }) => {
  if (!isServiceRow(row.original)) {
    const task = row.original.task;
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
  const availability = service.isManagedDraft
    ? { status: 'Unknown', tooltip: 'This managed Service has not been deployed yet.' }
    : getServiceAvailability(service);
  const expanded = row.getIsExpanded();
  return (
    <div className="flex min-w-0 items-center gap-2">
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
      <StateIndicator value={availability.status} tooltip={availability.tooltip} />

      <Link
        className="table-link truncate"
        to={getServiceViewRoute(service, platformId)}>
        {service.name || service.id.slice(0, 12)}
      </Link>
      {!service.managedServiceId && service.ownership === 'Unmanaged' && (
        <UnmanagedResourceIcon title={'Unmanaged Service'} />
      )}
      {service.isStale && <Badge variant="secondary">Stale</Badge>}
    </div>
  );
};
