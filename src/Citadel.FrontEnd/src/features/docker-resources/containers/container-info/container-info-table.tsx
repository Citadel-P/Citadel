import { ContainerInfoView, ContainerStateStatus } from '@/api/generated/api.types';
import { DataTable } from '@/components/ui/data-table';
import { PortsDisplay } from '@/components/custom/ports-display';
import { truncate } from '@/lib/truncate';
import { formatId } from '@/lib/utils';
import { ColumnDef } from '@tanstack/react-table';
import { Clock, Database, HardDrive, Network, Server } from 'lucide-react';
import { Link } from 'react-router';
import { DockerContainerView } from '@/api/types';
import { useRead } from '@/lib/hooks';
import { useEffect, useMemo } from 'react';
import { fromNow } from '@/lib/dayjs.helper';
import { ImageName } from '.';

const columns = (statusSnapshot: string | undefined): ColumnDef<ContainerInfoView>[] => [
  {
    accessorKey: 'platformName',
    header: () => <span>Platform</span>,
    cell: ({ row }) => (
      <div className="flex flex-wrap gap-2  items-center">
        <Server width={12} height={12} className="text-primary" />
        <Link to={`/platforms/${row.original.platformId}`} className="table-link">
          {row.original.platformName}
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
        <ImageName image={row.original.imageView ?? undefined} />
      </div>
    ),
  },
  {
    accessorKey: 'networks',
    header: () => <span>Networks</span>,
    cell: ({ row }) => (
      <div className=" text-foreground gap-2 flex flex-wrap items-center">
        <Network width={13} height={13} className="text-primary" />
        {Object.entries(row.original.networks).map(([key, value]) => (
          <Link
            to={`/platforms/${row.original.platformId}/networks/${formatId(value)}`}
            key={value}
            className="table-link">
            {key}
          </Link>
        ))}
      </div>
    ),
  },
  {
    accessorKey: 'volumes',
    header: () => <span>Volumes</span>,
    cell: ({ row }) => (
      <div className=" text-foreground gap-2 flex flex-wrap items-center">
        {row.original.volumes.length > 0 && <Database width={13} height={13} className="text-primary" />}
        {row.original.volumes.map((volume) => (
          <Link to={`/platforms/${row.original.platformId}/volumes/${volume}`} key={volume} className="table-link">
            {truncate(volume, 12)}
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
  {
    accessorKey: 'status',
    header: () => <span>Status</span>,
    cell: () => (
      <div className=" text-foreground gap-2 flex flex-wrap items-center">
        <Clock width={13} height={13} className="text-primary" />
        {statusSnapshot}
      </div>
    ),
  },
];

export const ContainerInfoTable = ({ container }: { container: DockerContainerView }) => {
  const { data, isLoading, refetch } = useRead('getContainerInfo', { id: container.id });

  useEffect(() => {
    refetch();
  }, [container.state, refetch]);

  const containerInfo = data?.data;
  const statusSnapshot = useMemo(() => {
    if (!containerInfo) return undefined;
    if (container.state === ContainerStateStatus.Created) return undefined;

    const baseDate =
      container.state === ContainerStateStatus.Running
        ? new Date(containerInfo.startedAt)
        : new Date(containerInfo.finishedAt ?? Date.now());

    return fromNow(baseDate);
  }, [container.state, containerInfo]);

  const cols = useMemo(() => columns(statusSnapshot), [statusSnapshot]);

  if (isLoading || !containerInfo) return null;

  return (
    <div className="flex flex-col gap-3">
      <DataTable columns={cols} data={[{ id: containerInfo.containerId, ...containerInfo }]} isLoading={isLoading} />
    </div>
  );
};
