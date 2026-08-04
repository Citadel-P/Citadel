import { SwarmNodeView } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import SortableCell from '@/components/custom/sortable-cell';
import { Badge } from '@/components/ui/badge';
import { DataTable } from '@/components/ui/data-table';
import { ProblemDetails } from '@/api/generated/api.types';
import { ColumnDef } from '@tanstack/react-table';
import { Network } from 'lucide-react';
import { useMemo } from 'react';
import { useNavigate, useParams } from 'react-router';
import { useSwarmNodes } from './use-swarm-nodes';

const statusColor = (node: SwarmNodeView) => {
  if (node.isStale) return 'bg-gray-400';
  if (node.status.toLowerCase() === 'ready') return 'bg-green-500';
  if (node.status.toLowerCase() === 'down' || node.status.toLowerCase() === 'disconnected') return 'bg-red-500';
  return 'bg-orange-400';
};

export const SwarmNodesPage = () => {
  const { platformId = '' } = useParams<{ platformId: string }>();
  const { nodes, isLoading, error } = useSwarmNodes(platformId);
  const navigate = useNavigate();
  const columns = useMemo<ColumnDef<SwarmNodeView>[]>(
    () => [
      {
        accessorKey: 'hostname',
        header: ({ column }) => <SortableCell cellName="Node" column={column} />,
        cell: ({ row }) => (
          <button
            type="button"
            className="flex items-center gap-2 text-left font-medium hover:underline"
            onClick={() => navigate(`/platforms/${platformId}/swarm/nodes/${row.original.id}`)}>
            <span className={`h-2 w-2 shrink-0 rounded-full ${statusColor(row.original)}`} />
            <span>{row.original.hostname || row.original.id.slice(0, 12)}</span>
            {row.original.isLeader && <Badge variant="outline">Leader</Badge>}
            {row.original.isStale && <Badge variant="secondary">Stale</Badge>}
          </button>
        ),
      },
      {
        accessorKey: 'role',
        header: ({ column }) => <SortableCell cellName="Role" column={column} />,
      },
      {
        accessorKey: 'status',
        header: ({ column }) => <SortableCell cellName="Status" column={column} />,
      },
      {
        accessorKey: 'availability',
        header: ({ column }) => <SortableCell cellName="Availability" column={column} />,
      },
      {
        accessorKey: 'runningTaskCount',
        header: ({ column }) => <SortableCell cellName="Tasks" column={column} />,
        cell: ({ row }) => `${row.original.runningTaskCount}/${row.original.desiredTaskCount}`,
      },
      {
        accessorKey: 'engineVersion',
        header: ({ column }) => <SortableCell cellName="Engine" column={column} />,
      },
      {
        accessorKey: 'address',
        header: ({ column }) => <SortableCell cellName="Address" column={column} />,
      },
      {
        accessorKey: 'observedAt',
        header: ({ column }) => <SortableCell cellName="Last observed" column={column} />,
        cell: ({ row }) => new Date(row.original.observedAt).toLocaleString(),
      },
    ],
    [navigate, platformId],
  );
  const problem = (error as unknown as { error?: ProblemDetails } | undefined)?.error;

  return (
    <div className="flex-col justify-between relative">
      <div className="mx-auto w-full max-w-[var(--layout-content-width)] px-4 py-4 sm:px-6">
        <div className="flex w-full flex-col gap-4 rounded-lg bg-background p-4">
          <div className="flex min-w-0 items-center gap-3">
            <div className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
              <Network className="h-4 w-4" />
            </div>
            <div>
              <h1 className="text-md font-bold text-foreground">Nodes</h1>
              <p className="text-xs text-muted-foreground">Docker Swarm node inventory and task placement.</p>
            </div>
          </div>
          {problem ? (
            <AlertMessage title={problem.title ?? 'Unable to load nodes'} type="error">
              {problem.detail ?? 'The Swarm node inventory could not be loaded.'}
            </AlertMessage>
          ) : (
            <DataTable
              columns={columns}
              data={nodes?.items ?? []}
              isLoading={isLoading}
              emptyState={{ title: 'No nodes found.', description: 'No nodes were returned by the Swarm manager.' }}
            />
          )}
        </div>
      </div>
    </div>
  );
};

export default SwarmNodesPage;
