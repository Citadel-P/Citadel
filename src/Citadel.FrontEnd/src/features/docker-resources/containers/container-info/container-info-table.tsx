import { ContainerInfoView, ContainerStateStatus, ContainerStatView } from '@/api/generated/api.types';
import { DataTable } from '@/components/ui/data-table';
import { PortsDisplay } from '@/components/custom/ports-display';
import { truncate } from '@/lib/truncate';
import { formatId, normalizeDockerId } from '@/lib/utils';
import { ColumnDef } from '@tanstack/react-table';
import { Clock, Database, HardDrive, Network, Server } from 'lucide-react';
import { Link } from 'react-router';
import { DockerContainerView } from '@/api/types';
import { useRead } from '@/lib/hooks';
import { useMemo } from 'react';
import { fromNow } from '@/lib/dayjs.helper';
import { ImageName } from '.';
import { CPUCell, MemoryUsageCell } from '@/components/custom/common';
import { StateIndicator } from '@/components/custom/state-indicator';

type ContainerInfoRow = ContainerInfoView & {
  containerStat: ContainerStatView;
  state: ContainerStateStatus;
  id: string;
};

type DisplayOptions = {
  DisplayContainerName?: boolean;
  DisplayPlatformName?: boolean;
  DisplayDeploymentName?: boolean;
  DisplayStatus?: boolean;
};

const PlatformCell = ({ name, id }: { name: string; id: string }) => (
  <div className="flex flex-wrap gap-2 items-center">
    <Server width={12} height={12} className="text-primary" />
    <Link to={`/platforms/${id}`} className="table-link">
      {name}
    </Link>
  </div>
);

const ContainerCell = ({
  name,
  id,
  state,
  platformId,
}: {
  name: string;
  id: string;
  state: ContainerStateStatus;
  platformId: string;
}) => (
  <div className="flex flex-wrap gap-1 items-center">
    <StateIndicator value={state ?? ContainerStateStatus.Exited} />
    <Link to={`/platforms/${platformId}/containers/${normalizeDockerId(id)}`} className="table-link">
      {name?.slice(1)}
    </Link>
  </div>
);

const ImageCell = ({ image }: { image: ContainerInfoView['imageView'] }) => (
  <div className="flex flex-wrap gap-2 items-center">
    <HardDrive width={13} height={13} className="text-primary" />
    <ImageName image={image ?? undefined} />
  </div>
);

const NetworksCell = ({ networks, platformId }: { networks: ContainerInfoView['networks']; platformId: string }) => (
  <div className="flex flex-wrap gap-2 items-center text-foreground">
    <Network width={13} height={13} className="text-primary" />
    {Object.entries(networks).map(([key, value]) => (
      <Link to={`/platforms/${platformId}/networks/${formatId(value)}`} key={value} className="table-link">
        {key}
      </Link>
    ))}
  </div>
);

const VolumesCell = ({ volumes, platformId }: { volumes: ContainerInfoView['volumes']; platformId: string }) => (
  <div className="flex flex-wrap gap-2 items-center text-foreground">
    {volumes.length > 0 && <Database width={13} height={13} className="text-primary" />}
    {volumes.map((volume) => (
      <Link to={`/platforms/${platformId}/volumes/${volume}`} key={volume} className="table-link">
        {truncate(volume, 12)}
      </Link>
    ))}
  </div>
);

const StatusCell = ({
  state,
  startedAt,
  finishedAt,
}: {
  state: ContainerStateStatus;
  startedAt: string;
  finishedAt?: string | null;
}) => {
  const statusSnapshot = useMemo(() => {
    if (state === ContainerStateStatus.Created) return undefined;

    const baseDate = state === ContainerStateStatus.Running ? new Date(startedAt) : new Date(finishedAt ?? Date.now());

    return fromNow(baseDate);
  }, [state, startedAt, finishedAt]);

  return (
    <div className="flex flex-wrap gap-2 items-center text-foreground">
      <Clock width={13} height={13} className="text-primary" />
      {statusSnapshot}
    </div>
  );
};

const getColumns = (displayOptions: DisplayOptions): ColumnDef<ContainerInfoRow>[] => {
  const cols: ColumnDef<ContainerInfoRow>[] = [];

  if (displayOptions.DisplayContainerName) {
    cols.push({
      accessorKey: 'containerName',
      header: () => <span>Name</span>,
      cell: ({ row }) => (
        <ContainerCell
          name={row.original.name}
          id={row.original.containerId}
          state={row.original.state}
          platformId={row.original.platformId}
        />
      ),
    });
  }

  if (displayOptions.DisplayPlatformName) {
    cols.push({
      accessorKey: 'platformName',
      header: () => <span>Platform</span>,
      cell: ({ row }) => <PlatformCell name={row.original.platformName} id={row.original.platformId} />,
    });
  }

  const coreCols: ColumnDef<ContainerInfoRow>[] = [
    {
      accessorKey: 'image',
      header: () => <span>Image</span>,
      cell: ({ row }) => <ImageCell image={row.original.imageView} />,
    },
    {
      accessorKey: 'networks',
      header: () => <span>Networks</span>,
      cell: ({ row }) => <NetworksCell networks={row.original.networks} platformId={row.original.platformId} />,
    },
    {
      accessorKey: 'volumes',
      header: () => <span>Volumes</span>,
      cell: ({ row }) => <VolumesCell volumes={row.original.volumes} platformId={row.original.platformId} />,
    },
    {
      accessorKey: 'ports',
      header: () => <span>Ports</span>,
      cell: ({ row }) => <PortsDisplay ports={row.original.ports} />,
    },
    {
      accessorKey: 'cpu',
      header: () => <span>Cpu</span>,
      cell: ({ row }) => <CPUCell state={row.original.state} stats={row.original.containerStat} />,
    },
    {
      accessorKey: 'memory',
      header: () => <span>Memory</span>,
      cell: ({ row }) => <MemoryUsageCell state={row.original.state} stats={row.original.containerStat} />,
    },
  ];

  if (displayOptions.DisplayStatus) {
    coreCols.push({
      accessorKey: 'status',
      header: () => <span>Status</span>,
      cell: ({ row }) => (
        <StatusCell
          state={row.original.state}
          startedAt={row.original.startedAt}
          finishedAt={row.original.finishedAt}
        />
      ),
    });
  }

  return [...cols, ...coreCols];
};

export const ContainerInfoTable = ({
  container,
  displayOptions,
}: {
  container: DockerContainerView;
  displayOptions: DisplayOptions;
}) => {
  const { data, isLoading } = useRead('getContainerInfo', { id: container.id });
  const containerInfo = data?.data;

  const columns = useMemo(() => getColumns(displayOptions), [displayOptions]);

  if (isLoading || !containerInfo) return null;

  const tableData: ContainerInfoRow[] = [
    {
      ...containerInfo,
      id: containerInfo.containerId,
      containerStat: container.containerStat,
      state: container.state,
    },
  ];

  return (
    <div className="rounded-sm border p-1 shadow-xs">
      <DataTable columns={columns} data={tableData} isLoading={isLoading} />
    </div>
  );
};
