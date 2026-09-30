import {
  ContainerInfoView,
  ContainerStateStatus,
  ContainerStatView,
  ContainerDataView,
} from '@/api/generated/api.types';
import { DataTable } from '@/components/ui/data-table';
import { PortsDisplay } from '@/components/custom/ports-display';
import { truncate } from '@/lib/truncate';
import { ColumnDef } from '@tanstack/react-table';
import { Clock, Server, Info } from 'lucide-react';
import { DetailFacts, DetailSection } from '@/components/custom/resource-detail';
import Loader from '@/components/ui/loader';
import { Link } from 'react-router';
import { useLocalStorage, useRead } from '@/lib/hooks';
import { useMemo } from 'react';
import { fromNow } from '@/lib/dayjs.helper';
import { ImageName } from '.';
import {
  CPUCell,
  DockerContainerCell,
  DockerImageCell,
  DockerNetworksCell,
  DockerVolumesCell,
  MemoryUsageCell,
} from '@/components/custom/common';

type ContainerInfoRow = ContainerInfoView & {
  containerStat: Partial<ContainerStatView>;
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
    <Link to={`/platforms/edit/${id}`} className="table-link" title={name}>
      {truncate(name, 24)}
    </Link>
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
    if (state === ContainerStateStatus.Created) return 'Not started';

    const baseDate = state === ContainerStateStatus.Running ? new Date(startedAt) : new Date(finishedAt!);
    if (!Number.isFinite(baseDate.getTime()) || baseDate.getUTCFullYear() <= 1970) return 'Not available';
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
        <DockerContainerCell
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
      cell: ({ row }) => (
        <DockerImageCell>
          <ImageName image={row.original.imageView ?? undefined} />
        </DockerImageCell>
      ),
    },
    {
      accessorKey: 'networks',
      header: () => <span>Networks</span>,
      cell: ({ row }) => <DockerNetworksCell networks={row.original.networks} platformId={row.original.platformId} />,
    },
    {
      accessorKey: 'volumes',
      header: () => <span>Volumes</span>,
      cell: ({ row }) => <DockerVolumesCell volumes={row.original.volumes} platformId={row.original.platformId} />,
    },
    {
      accessorKey: 'ports',
      header: () => <span>Ports</span>,
      cell: ({ row }) => <PortsDisplay ports={row.original.ports ?? {}} compact maxVisible={1} />,
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

export const ContainerOverview = ({ container }: { container?: ContainerDataView | undefined }) => {
  const [overviewOpen, setOverviewOpen] = useLocalStorage('container-overview-open', true);
  const { data, isLoading } = useRead('getContainerInfo', { id: container?.id });
  const containerInfo = data?.data;
  const state = container?.state ?? containerInfo?.state ?? ContainerStateStatus.Unknown;
  const stats = container?.containerStat;
  const running = state === ContainerStateStatus.Running;

  return (
    <DetailSection
      title="Container overview"
      description="Runtime usage, image, and resource connections."
      icon={Info}
      collapse={{ open: overviewOpen !== false, onOpenChange: setOverviewOpen }}>
      {isLoading ? (
        <>
          <DetailFacts resource={container} items={[]} />
          <Loader />
        </>
      ) : !containerInfo ? (
        <>
          <DetailFacts resource={container} items={[]} />
          <p className="text-sm text-muted-foreground">Container details are not available.</p>
        </>
      ) : (
        <DetailFacts
          resource={{ id: container?.id ?? containerInfo.containerId, name: container?.name ?? containerInfo.name }}
          items={[
            {
              label: 'Platform',
              value: <PlatformCell name={containerInfo.platformName} id={containerInfo.platformId} />,
            },
            { label: 'Image', value: <ImageName image={containerInfo.imageView ?? undefined} /> },
            {
              label: 'Networks',
              value: Object.keys(containerInfo.networks ?? {}).length ? (
                <DockerNetworksCell networks={containerInfo.networks} platformId={containerInfo.platformId} />
              ) : (
                'None'
              ),
            },
            {
              label: 'Volumes',
              value: containerInfo.volumes?.length ? (
                <DockerVolumesCell volumes={containerInfo.volumes} platformId={containerInfo.platformId} />
              ) : (
                'None'
              ),
            },
            {
              label: 'Ports',
              value: Object.keys(containerInfo.ports ?? {}).length ? (
                <PortsDisplay ports={containerInfo.ports} compact maxVisible={3} />
              ) : (
                'None published'
              ),
            },
            {
              label: 'CPU usage',
              value: !running ? (
                'Not running'
              ) : stats?.cpuUsage == null ? (
                'Waiting for sample'
              ) : (
                <CPUCell state={state} stats={stats} />
              ),
            },
            {
              label: 'Memory usage',
              value: !running ? (
                'Not running'
              ) : stats?.memoryActive == null ? (
                'Waiting for sample'
              ) : (
                <MemoryUsageCell state={state} stats={stats} />
              ),
            },
            {
              label: running ? 'Started' : 'Finished',
              value:
                state === ContainerStateStatus.Created ? (
                  'Not started'
                ) : (
                  <StatusCell state={state} startedAt={containerInfo.startedAt} finishedAt={containerInfo.finishedAt} />
                ),
            },
          ]}
        />
      )}
    </DetailSection>
  );
};

export const DeploymentContainerInfoTable = ({
  deploymentId,
  container,
  displayOptions,
}: {
  deploymentId: string;
  container?: ContainerDataView | undefined;
  displayOptions: DisplayOptions;
}) => {
  const { data, isLoading } = useRead('getDeploymentContainerInfo', { id: deploymentId });
  const containerInfo = data?.data;

  return (
    <ContainerInfoTableRenderer
      containerInfo={containerInfo}
      container={container}
      displayOptions={displayOptions}
      isLoading={isLoading}
    />
  );
};

const ContainerInfoTableRenderer = ({
  containerInfo,
  container,
  displayOptions,
  isLoading,
}: {
  containerInfo?: ContainerInfoView | undefined;
  container?: ContainerDataView | undefined;
  displayOptions: DisplayOptions;
  isLoading?: boolean;
}) => {
  const columns = useMemo(() => getColumns(displayOptions), [displayOptions]);

  const tableData: ContainerInfoRow[] = containerInfo
    ? [
        {
          ...containerInfo,
          name: container?.name ?? containerInfo.name,
          id: containerInfo.containerId,
          containerStat: container?.containerStat ?? {},
          state: container?.state ?? containerInfo.state,
        },
      ]
    : [];

  const finalIsLoading = !container?.id || Boolean(isLoading);

  return <DataTable columns={columns} data={tableData} isLoading={finalIsLoading} />;
};
