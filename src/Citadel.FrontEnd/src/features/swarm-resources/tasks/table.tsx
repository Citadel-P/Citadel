import { SwarmTaskView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { StateBadge } from '@/components/custom/state-badge';
import { Badge } from '@/components/ui/badge';
import { DataTable } from '@/components/ui/data-table';
import { getTaskName } from '@/lib/utils';
import { ColumnDef } from '@tanstack/react-table';
import { useMemo } from 'react';
import { Link, useParams } from 'react-router';

export const TasksTable = ({
  items,
  isLoading,
  showService = true,
  showNode = true,
}: {
  items: SwarmTaskView[];
  isLoading: boolean;
  showService?: boolean;
  showNode?: boolean;
}) => {
  const { platformId = '' } = useParams<{ platformId: string }>();
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
      {
        accessorKey: 'statusTimestamp',
        header: 'Updated',
        cell: ({ row }) =>
          row.original.statusTimestamp ? new Date(row.original.statusTimestamp).toLocaleString() : '-',
      },
    );
    return result;
  }, [platformId, showNode, showService]);

  return (
    <DataTable
      columns={columns}
      data={items}
      isLoading={isLoading}
      emptyState={{ title: 'No tasks found.', description: 'No tasks were returned by the Swarm manager.' }}
    />
  );
};
