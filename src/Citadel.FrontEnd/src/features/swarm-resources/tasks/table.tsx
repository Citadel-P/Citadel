import { ContainerView, SwarmTaskView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { CPUCell, MemoryUsageCell } from '@/components/custom/common';
import { StateIndicator } from '@/components/custom/state-indicator';
import { StateBadge } from '@/components/custom/state-badge';
import { Badge } from '@/components/ui/badge';
import { DataTable } from '@/components/ui/data-table';
import { getSwarmTaskIdFromContainerName, getTaskName } from '@/lib/utils';
import { ColumnDef } from '@tanstack/react-table';
import { useMemo } from 'react';
import { Link, useParams } from 'react-router';

export const TasksTable = ({
  items,
  isLoading,
  showService = true,
  showNode = true,
  platformId: platformIdOverride,
  taskContainers,
}: {
  items: SwarmTaskView[];
  isLoading: boolean;
  showService?: boolean;
  showNode?: boolean;
  platformId?: string;
  taskContainers?: ContainerView[];
}) => {
  const { platformId: routePlatformId = '' } = useParams<{ platformId: string }>();
  const platformId = platformIdOverride ?? routePlatformId;
  const taskContainersById = useMemo(() => {
    const result = new Map<string, ContainerView>();
    for (const container of taskContainers ?? []) {
      const taskId = getSwarmTaskIdFromContainerName(container.name);
      if (taskId) result.set(taskId, container);
    }
    return result;
  }, [taskContainers]);
  const columns = useMemo<ColumnDef<SwarmTaskView>[]>(() => {
    const result: ColumnDef<SwarmTaskView>[] = [
      {
        accessorKey: 'name',
        header: ({ column }) => <SortableCell cellName="Task" column={column} />,
        cell: ({ row }) => (
          <div className="flex min-w-0 items-center gap-2">
            <StateIndicator value={row.original.state} kind="swarmTask" />
            <Link
              to={`/platforms/${platformId}/tasks/${row.original.id}`}
              className="table-link truncate"
              title={getTaskName(row.original)}>
              {getTaskName(row.original)}
            </Link>
            {row.original.isStale && <Badge variant="secondary">Stale</Badge>}
          </div>
        ),
      },
    ];

    if (showService) {
      result.push({
        accessorKey: 'serviceName',
        header: ({ column }) => <SortableCell cellName="Service" column={column} />,
        cell: ({ row }) =>
          row.original.serviceId ? (
            <Link className="table-link" to={`/platforms/${platformId}/services/${row.original.serviceId}`}>
              {row.original.serviceName || row.original.serviceId.slice(0, 12)}
            </Link>
          ) : (
            '-'
          ),
      });
    }

    if (showNode) {
      result.push({
        accessorKey: 'nodeHostname',
        header: ({ column }) => <SortableCell cellName="Node" column={column} />,
        cell: ({ row }) =>
          row.original.nodeId ? (
            <Link className="table-link" to={`/platforms/${platformId}/nodes/${row.original.nodeId}`}>
              {row.original.nodeHostname || row.original.nodeId.slice(0, 12)}
            </Link>
          ) : (
            '-'
          ),
      });
    }

    result.push(
      {
        accessorKey: 'desiredState',
        header: ({ column }) => <SortableCell cellName="Desired" column={column} />,
        cell: ({ row }) => <StateBadge value={row.original.desiredState} kind="swarmTask" />,
      },
      {
        accessorKey: 'image',
        header: 'Image',
        cell: ({ row }) => (
          <span className="block max-w-64 truncate" title={row.original.image}>
            {row.original.image}
          </span>
        ),
      },
    );

    if (taskContainers !== undefined) {
      result.push(
        {
          id: 'cpu',
          header: 'CPU',
          cell: ({ row }) => {
            const container = taskContainersById.get(row.original.id);
            return container ? <CPUCell state={container.state} stats={container.lastStats} /> : '-';
          },
        },
        {
          id: 'memory',
          header: 'Memory',
          cell: ({ row }) => {
            const container = taskContainersById.get(row.original.id);
            return container ? <MemoryUsageCell state={container.state} stats={container.lastStats} /> : '-';
          },
        },
      );
    }

    result.push({
      accessorKey: 'statusTimestamp',
      header: 'Updated',
      cell: ({ row }) => (row.original.statusTimestamp ? new Date(row.original.statusTimestamp).toLocaleString() : '-'),
    });
    return result;
  }, [platformId, showNode, showService, taskContainers, taskContainersById]);

  return (
    <DataTable
      columns={columns}
      data={items}
      isLoading={isLoading}
      emptyState={{ title: 'No tasks found.', description: 'No tasks were returned by the Swarm manager.' }}
    />
  );
};
