import { StateBadge } from '@/components/custom/state-badge';
import { DataTable } from '@/components/ui/data-table';
import { ColumnDef } from '@tanstack/react-table';
import { SwarmTaskInfoView } from '../hooks/useTasksGroup';
import { Link } from 'react-router';

const columns: ColumnDef<SwarmTaskInfoView>[] = [
  {
    accessorKey: 'serviceName',
    header: 'Service',
    cell: ({ row }) =>
      row.original.serviceId ? (
        <Link
          className="table-link block max-w-48 truncate"
          title={row.original.serviceName || row.original.serviceId}
          to={`/platforms/${row.original.platformId}/services/${row.original.serviceId}`}>
          {row.original.serviceName || row.original.serviceId}
        </Link>
      ) : (
        '-'
      ),
  },
  {
    accessorKey: 'nodeHostname',
    header: 'Node',
    cell: ({ row }) =>
      row.original.nodeId ? (
        <Link
          className="table-link block max-w-48 truncate"
          title={row.original.nodeHostname || row.original.nodeId}
          to={`/platforms/${row.original.platformId}/nodes/${row.original.nodeId}`}>
          {row.original.nodeHostname || row.original.nodeId}
        </Link>
      ) : (
        '-'
      ),
  },
  {
    accessorKey: 'image',
    header: 'Image',
    cell: ({ row }) => (
      <span className="block max-w-64 truncate" title={row.original.image || undefined}>
        {row.original.image || '-'}
      </span>
    ),
  },
  {
    accessorKey: 'slot',
    header: 'Slot',
    cell: ({ row }) => row.original.slot ?? '-',
  },
  {
    accessorKey: 'desiredState',
    header: 'Desired',
    cell: ({ row }) => <StateBadge value={row.original.desiredState} kind="swarmTask" />,
  },
  {
    accessorKey: 'statusMessage',
    header: 'Message',
    cell: ({ row }) => (
      <span className="block max-w-64 truncate" title={row.original.statusMessage ?? undefined}>
        {row.original.statusMessage ?? '-'}
      </span>
    ),
  },
  {
    accessorKey: 'error',
    header: 'Error',
    cell: ({ row }) => (
      <span className="block max-w-64 truncate text-destructive" title={row.original.error ?? undefined}>
        {row.original.error ?? '-'}
      </span>
    ),
  },
];

export const TaskInfoTable = ({ task }: { task: SwarmTaskInfoView }) => (
  <div className="rounded-sm border p-1 shadow-xs">
    <DataTable columns={columns} data={[task]} isLoading={false} />
  </div>
);
