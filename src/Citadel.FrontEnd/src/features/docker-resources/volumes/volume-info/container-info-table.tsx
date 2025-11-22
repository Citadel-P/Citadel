import {
  ContainerVolumeResult,
  ContainerStateStatus,
  DockerVolumeResult,
  PlatformView,
} from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';
import { StateIndicator } from '@/components/custom/state-indicator';
import { DataTable } from '@/components/ui/data-table';
import { PortsDisplay } from '@/components/custom/ports-display';
import { truncate } from '@/lib/truncate';
import { formatId } from '@/lib/utils';
import { ColumnDef } from '@tanstack/react-table';
import { HardDrive, Network } from 'lucide-react';
import { Link } from 'react-router';

const columns = (currentPlatform: PlatformView | undefined): ColumnDef<ContainerVolumeResult>[] => [
  {
    accessorKey: 'name',
    header: () => <span>Name</span>,
    cell: ({ row }) => (
      <div className="flex flex-wrap gap-2 text-sm items-center">
        <StateIndicator value={row.original.state ?? ContainerStateStatus.Exited} />
        <Link to={`/platforms/${currentPlatform?.id}/containers/${formatId(row.original.id)}`} className="table-link">
          {row.original.name?.slice(1)}
        </Link>
      </div>
    ),
  },
  {
    accessorKey: 'image',
    header: () => <span>Image</span>,
    cell: ({ row }) => (
      <div className=" gap-2 flex flex-wrap items-center">
        <HardDrive width={13} height={13} className="text-primary" />
        <Link to={`/platforms/${currentPlatform?.id}/images/${formatId(row.original.imageId)}`} className="table-link">
          {truncate(row.original.image.replace('sha256:', ''), 24)}
        </Link>
      </div>
    ),
  },
  {
    accessorKey: 'networks',
    header: () => <span>Networks</span>,
    cell: ({ row }) => (
      <div className="text-foreground gap-2 flex flex-wrap items-center">
        <Network width={13} height={13} className="text-primary" />
        {Object.entries(row.original.networks).map(([name, id]) => (
          <Link to={`/platforms/${currentPlatform?.id}/networks/${formatId(id)}`} key={id} className="table-link">
            {name}
          </Link>
        ))}
      </div>
    ),
  },

  {
    accessorKey: 'ports',
    header: () => <span>Ports</span>,
    cell: ({ row }) => <PortsDisplay ports={row.original.ports} />,
  },
];

export const ContainerInfoTable = ({ volume }: { volume: DockerVolumeResult | undefined }) => {
  const { currentPlatform } = useAppContext();
  if (!volume) return <></>;
  return (
    <div className="rounded-sm border p-1 shadow-xs">
      <DataTable columns={columns(currentPlatform)} data={volume.containers} isLoading={false} />
    </div>
  );
};
