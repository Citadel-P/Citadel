import type { ColumnDef } from '@tanstack/react-table';
import { useNavigate, useParams } from 'react-router';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { truncate } from '@/lib/truncate';

export type NodeScopedResource = {
  dockerNodeId?: string | null;
  nodeHostname?: string | null;
  isStale?: boolean;
  staleReason?: string | null;
};

export const createNodeResourceColumn = <T extends NodeScopedResource>(): ColumnDef<T> => ({
  accessorKey: 'nodeHostname',
  header: ({ column }) => <SortableCell cellName="Node" column={column} />,
  cell: ({ row }) => <NodeResourceCell resource={row.original} />,
  sortingFn: (rowA, rowB) =>
    (rowA.original.nodeHostname ?? rowA.original.dockerNodeId ?? '').localeCompare(
      rowB.original.nodeHostname ?? rowB.original.dockerNodeId ?? '',
    ),
});

const NodeResourceCell = ({ resource }: { resource: NodeScopedResource }) => {
  const { platformId } = useParams<{ platformId: string }>();
  const navigate = useNavigate();
  const label = resource.nodeHostname ?? resource.dockerNodeId ?? '-';
  const isLinked = !!platformId && !!resource.dockerNodeId;

  const openNode = () => {
    if (isLinked) navigate(`/platforms/${platformId}/nodes/${resource.dockerNodeId}`);
  };

  return (
    <div className="flex items-center whitespace-nowrap">
      <StateIndicator
        value={!resource.isStale}
        tooltip={resource.isStale ? resource.staleReason || 'Node observation is stale' : 'Fresh Node observation'}
      />
      <span
        className={isLinked ? 'table-link cursor-pointer' : undefined}
        title={label}
        onClick={openNode}
        onKeyDown={(event) => {
          if (isLinked && (event.key === 'Enter' || event.key === ' ')) openNode();
        }}
        tabIndex={isLinked ? 0 : undefined}
        role={isLinked ? 'button' : undefined}>
        {truncate(label, 28, 'right')}
      </span>
    </div>
  );
};
