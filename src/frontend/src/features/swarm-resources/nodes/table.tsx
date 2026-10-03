import { SwarmTaskView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { StateBadge } from '@/components/custom/state-badge';
import { getSwarmNodeIndicatorValue, StateIndicator } from '@/components/custom/state-indicator';
import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable } from '@/components/ui/data-table';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { getTaskName } from '@/lib/utils';
import { ColumnDef, Row } from '@tanstack/react-table';
import { ChevronDown, ChevronRight } from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import { Link, useParams } from 'react-router';
import { SwarmNodeListView } from './hooks/useNodesGroup';
import { DropdownActionComponent } from '@/pages/types';
import { NodeEditDialog } from './node-edit-dialog';
import { useSelectedResources } from '@/lib/atoms';

type NodeRow = {
  id: string;
  kind: 'node';
  node: SwarmNodeListView;
  tasks: TaskRow[];
};

type TaskRow = {
  id: string;
  kind: 'task';
  task: SwarmTaskView;
};

type NodeTableRow = NodeRow | TaskRow;

const isNodeRow = (row: NodeTableRow): row is NodeRow => row.kind === 'node';

export const NodesTable = ({
  items,
  actions,
  isLoading,
}: {
  items: SwarmNodeListView[];
  actions: Record<string, DropdownActionComponent<SwarmNodeListView>>;
  isLoading: boolean;
}) => {
  const { platformId = '' } = useParams<{ platformId: string }>();
  const [, setSelectedResources] = useSelectedResources<SwarmNodeListView>('Node');
  const [editing, setEditing] = useState<SwarmNodeListView | null>(null);
  const rows = useMemo<NodeTableRow[]>(
    () =>
      items.map((node) => ({
        id: `node:${node.id}`,
        kind: 'node',
        node,
        tasks: node.tasks.map((task) => ({ id: `task:${task.id}`, kind: 'task', task })),
      })),
    [items],
  );
  const getSubRows = useCallback((row: NodeTableRow) => (isNodeRow(row) ? row.tasks : undefined), []);
  const columns = useMemo<ColumnDef<NodeTableRow>[]>(
    () => [
      {
        id: 'select',
        header: ({ table }) => (
          <Checkbox
            checked={table.getIsAllPageRowsSelected() || (table.getIsSomePageRowsSelected() && 'indeterminate')}
            onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
            aria-label="Select all Nodes"
          />
        ),
        cell: ({ row }) =>
          isNodeRow(row.original) ? (
            <Checkbox
              checked={row.getIsSelected()}
              onCheckedChange={(value) => row.toggleSelected(!!value)}
              aria-label={`Select Node ${row.original.node.name}`}
            />
          ) : null,
        enableSorting: false,
        enableHiding: false,
      },
      {
        id: 'name',
        accessorFn: (row) => (isNodeRow(row) ? row.node.hostname : getTaskName(row.task)),
        header: ({ column }) => <SortableCell cellName="Node" column={column} />,
        cell: ({ row }) => <NodeNameCell row={row} platformId={platformId} />,
      },
      {
        id: 'role',
        accessorFn: (row) => (isNodeRow(row) ? row.node.role : 'Task'),
        header: ({ column }) => <SortableCell cellName="Role" column={column} />,
        cell: ({ row }) => (isNodeRow(row.original) ? row.original.node.role : 'Task'),
      },
      {
        id: 'runtime',
        header: 'Runtime',
        cell: ({ row }) =>
          isNodeRow(row.original) ? (
            `${row.original.node.runningTaskCount}/${row.original.node.desiredTaskCount} tasks`
          ) : (
            <StateBadge value={row.original.task.desiredState} kind="swarmTask" />
          ),
      },
      {
        id: 'service',
        header: 'Service',
        cell: ({ row }) => {
          if (isNodeRow(row.original)) return '-';
          const task = row.original.task;
          return task.serviceId ? (
            <Link className="table-link" to={`/platforms/${platformId}/services/${task.serviceId}`}>
              {task.serviceName || task.serviceId.slice(0, 12)}
            </Link>
          ) : (
            '-'
          );
        },
      },
      {
        id: 'image',
        accessorFn: (row) => (isNodeRow(row) ? '' : row.task.image),
        header: 'Image',
        cell: ({ row }) =>
          isNodeRow(row.original) ? (
            '-'
          ) : (
            <span className="block max-w-64 truncate" title={row.original.task.image}>
              {row.original.task.image}
            </span>
          ),
      },
      {
        id: 'engine',
        accessorFn: (row) => (isNodeRow(row) ? row.node.engineVersion : ''),
        header: ({ column }) => <SortableCell cellName="Engine" column={column} />,
        cell: ({ row }) => (isNodeRow(row.original) ? row.original.node.engineVersion || '-' : '-'),
      },
      {
        id: 'address',
        accessorFn: (row) => (isNodeRow(row) ? row.node.address : ''),
        header: ({ column }) => <SortableCell cellName="Address" column={column} />,
        cell: ({ row }) => (isNodeRow(row.original) ? row.original.node.address || '-' : '-'),
      },
      {
        id: 'actions',
        cell: ({ row }) =>
          isNodeRow(row.original) ? (
            <RowActionMenu
              resource={row.original.node}
              actions={actions}
              onAction={({ key }) => key === 'edit' && isNodeRow(row.original) && setEditing(row.original.node)}
            />
          ) : null,
        enableSorting: false,
        enableHiding: false,
      },
    ],
    [actions, platformId],
  );

  return (
    <>
      <DataTable
        columns={columns}
        data={rows}
        isLoading={isLoading}
        getSubRows={getSubRows}
        enableRowSelection={(row) => isNodeRow(row)}
        enableSubRowSelection={false}
        onSelectionChange={(selected) => setSelectedResources(selected.filter(isNodeRow).map((row) => row.node))}
        emptyState={{ title: 'No nodes found.', description: 'No nodes were returned by the Swarm manager.' }}
      />
      {editing && <NodeEditDialog resource={editing} open onOpenChange={(open) => !open && setEditing(null)} />}
    </>
  );
};

const NodeNameCell = ({ row, platformId }: { row: Row<NodeTableRow>; platformId: string }) => {
  if (!isNodeRow(row.original)) {
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

  const node = row.original.node;
  const expanded = row.getIsExpanded();
  return (
    <div className="flex min-w-0 items-center gap-2">
      {row.getCanExpand() ? (
        <button
          type="button"
          onClick={row.getToggleExpandedHandler()}
          className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-muted-foreground hover:bg-accent/70 hover:text-foreground"
          aria-label={expanded ? `Collapse node ${node.hostname}` : `Expand node ${node.hostname}`}>
          {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
        </button>
      ) : (
        <span className="h-5 w-5 shrink-0" />
      )}
      <StateIndicator value={getSwarmNodeIndicatorValue(node)} kind="swarmNode" />
      <Link className="table-link truncate" to={`/platforms/${platformId}/nodes/${node.id}`}>
        {node.hostname || node.id.slice(0, 12)}
      </Link>
      {node.isLeader && <Badge variant="outline">Leader</Badge>}
      {node.isStale && <Badge variant="secondary">Stale</Badge>}
    </div>
  );
};
