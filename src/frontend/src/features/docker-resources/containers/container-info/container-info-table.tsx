import { BindMountBackupLinks, VolumeBackupLinks } from '@/features/backup-policies/volume-backup-links';
import {
  ContainerSummaryView,
  ContainerStateStatus,
  ContainerStatView,
  ContainerRuntimeView,
} from '@/api/generated/api.types';
import { DataTable } from '@/components/ui/data-table';
import { PortsDisplay } from '@/components/custom/ports-display';
import { truncate } from '@/lib/truncate';
import { CellContext, ColumnDef } from '@tanstack/react-table';
import { Clock, Server, Info } from 'lucide-react';
import { DetailFacts, DetailSection } from '@/components/custom/resource-detail';
import Loader from '@/components/ui/loader';
import { Link, useLocation } from 'react-router';
import { useLocalStorage, useRead } from '@/lib/hooks';
import { useEffect, useMemo, type ReactNode } from 'react';
import { fromNow } from '@/lib/dayjs.helper';
import { ContainerDetailsView } from '../hooks/useContainerInfoGroup';
import { ContainerOverviewHealth } from './container-overview-health';
import { ImageName } from '.';
import {
  CPUCell,
  DockerContainerCell,
  DockerImageCell,
  DockerNetworksCell,
  DockerVolumesCell,
  MemoryUsageCell,
} from '@/components/custom/common';

type ContainerInfoRow = ContainerSummaryView & {
  health?: ReactNode;
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

// A module-level cell preserves popover/dialog state when realtime data changes.
const ContainerHealthCell = ({ row }: CellContext<ContainerInfoRow, unknown>) => row.original.health;

const getColumns = (displayOptions: DisplayOptions, showHealth: boolean): ColumnDef<ContainerInfoRow>[] => {
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
    ...(showHealth ? [{ id: 'health', header: 'Health', cell: ContainerHealthCell, enableSorting: false }] : []),
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

export const ContainerOverview = ({ container }: { container?: ContainerDetailsView }) => {
  const [overviewOpen, setOverviewOpen] = useLocalStorage('container-overview-open', true);
  const { hash } = useLocation();
  useEffect(() => {
    if (hash === '#health') setOverviewOpen(true);
  }, [hash, setOverviewOpen]);
  const { data, isLoading } = useRead('getContainerInfo', { id: container?.id });
  const containerInfo = data?.data;
  const backupCoverage = container?.backupCoverage ?? containerInfo?.backupCoverage;
  const volumeNames =
    backupCoverage && !backupCoverage.unavailable
      ? backupCoverage.volumes.filter((volume) => !volume.hostPath).map((volume) => volume.name)
      : (containerInfo?.volumes ?? []);
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
        <div className="grid grid-cols-1 items-start gap-6 lg:grid-cols-3">
          <DetailFacts
            layout="stack"
            resource={{ id: container?.id ?? containerInfo.containerId, name: container?.name ?? containerInfo.name }}
            items={[
              {
                label: 'Platform',
                value: <PlatformCell name={containerInfo.platformName} id={containerInfo.platformId} />,
              },
              { label: 'Image', value: <ImageName image={containerInfo.imageView ?? undefined} /> },
              {
                label: running ? 'Started' : 'Finished',
                value:
                  state === ContainerStateStatus.Created ? (
                    'Not started'
                  ) : (
                    <StatusCell
                      state={state}
                      startedAt={containerInfo.startedAt}
                      finishedAt={containerInfo.finishedAt}
                    />
                  ),
              },
            ]}
          />
          <DetailFacts
            layout="stack"
            items={[
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
                label: 'Networks',
                value: Object.keys(containerInfo.networks ?? {}).length ? (
                  <DockerNetworksCell networks={containerInfo.networks} platformId={containerInfo.platformId} />
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
            ]}
          />
          <DetailFacts
            layout="stack"
            items={[
              {
                label: 'Volumes',
                value: volumeNames.length ? (
                  <VolumeBackupLinks
                    volumes={volumeNames}
                    coverage={backupCoverage}
                    platformId={containerInfo.platformId}
                    dockerNodeId={container?.dockerNodeId}
                  />
                ) : (
                  'None'
                ),
              },
              ...(backupCoverage?.volumes.some((volume) => volume.hostPath)
                ? [{ label: 'Bind mounts', value: <BindMountBackupLinks coverage={backupCoverage} /> }]
                : []),

              ...(container
                ? [
                    {
                      label: 'Health',
                      value: <ContainerOverviewHealth key={container.resourceId} resource={container} />,
                    },
                  ]
                : []),
            ]}
          />
        </div>
      )}
    </DetailSection>
  );
};

export const DeploymentContainerInfoTable = ({
  summary,
  deploymentId,
  container,
  displayOptions,
  health,
}: {
  health?: ReactNode;
  deploymentId: string;
  summary?: ContainerSummaryView;
  container?: ContainerRuntimeView | undefined;
  displayOptions: DisplayOptions;
}) => {
  const { data, isLoading } = useRead('getDeploymentContainerInfo', { id: deploymentId }, { enabled: !summary });
  const containerInfo = summary ?? data?.data;

  return (
    <ContainerInfoTableRenderer
      containerInfo={containerInfo}
      container={container}
      displayOptions={displayOptions}
      health={health}
      isLoading={isLoading}
    />
  );
};

const ContainerInfoTableRenderer = ({
  containerInfo,
  container,
  displayOptions,
  health,
  isLoading,
}: {
  health?: ReactNode;
  containerInfo?: ContainerSummaryView | undefined;
  container?: ContainerRuntimeView | undefined;
  displayOptions: DisplayOptions;
  isLoading?: boolean;
}) => {
  const showHealth = Boolean(health);
  const columns = useMemo(() => getColumns(displayOptions, showHealth), [displayOptions, showHealth]);

  const tableData: ContainerInfoRow[] = containerInfo
    ? [
        {
          ...containerInfo,
          health,
          name: container?.name ?? containerInfo.name,
          id: containerInfo.containerId,
          containerStat: container?.containerStat ?? {},
          state: container?.state ?? containerInfo.state,
        },
      ]
    : [];

  const finalIsLoading = !containerInfo && Boolean(isLoading);

  return <DataTable columns={columns} data={tableData} isLoading={finalIsLoading} />;
};
