import type { ContainerGroupHealth, ContainerHealthItem } from './group-health';
import { ContainerGroupHealthCell } from './group-health-cell';
import { currentHealth } from './health';
import { ContainerTableHealth } from './container-info/container-health-card';
import { backupColumn, ResourceBackupCell } from '@/features/backup-policies/table-coverage';
import { DataTable } from '@/components/ui/data-table';
import { ColumnDef, Row, flexRender } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import {
  ContainerView,
  ContainerStateStatus,
  ContainerStatView,
  PlatformView,
  PlatformType,
  ResourceControlState,
  StackReleaseStatus,
} from '@/api/generated/api.types';
import { truncate } from '@/lib/truncate';
import { cn, formatId } from '@/lib/utils';
import SortableCell from '@/components/custom/sortable-cell';
import { Link } from 'react-router';
import { CopyToClipboard } from '@/components/custom/copy-to-clipboard';
import { PortsDisplay } from '@/components/custom/ports-display';
import { ImageName } from './container-info/';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useCallback, useMemo, useRef } from 'react';
import { useAtomValue } from 'jotai';
import { containerColumnsAtom } from './table-columns';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { CPUCell, MemoryUsageCell, UnmanagedResourceIcon } from '@/components/custom/common';
import { ChevronDown, ChevronRight } from 'lucide-react';
import {
  ContainerActionResource,
  ContainerStackGroupResource,
  isContainerAdoptionAvailable,
  isContainerStackGroup,
} from './actions';
import { useAppContext } from '@/lib/context/app-context';
import { SystemContainerBadge } from './system-container-badge';
import { isVisibleContainer, normalizeContainerSelection } from './selection';
import { StackBackupCell } from '@/features/stacks/backup-cell';
import { groupBackupCoverage } from './group-backup-coverage';

type ContainerTableRow = ContainerView | (ContainerStackGroupResource & { healthSummary?: ContainerGroupHealth });
type PlatformResourceLimits = {
  memoryTotal: number;
  cpuCount: number;
};

export const ContainersTable = ({
  items,
  isLoading,
  isFiltered,
  actions,
}: {
  items: ContainerHealthItem[];
  isLoading: boolean;
  isFiltered?: boolean;
  actions: Record<
    string,
    React.FC<{ resource: ContainerActionResource; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const { currentPlatform } = useAppContext();
  const [_, setSelectedResources] = useSelectedResources<ContainerActionResource>('Container');
  const lastSelectionKeyRef = useRef('');
  const platformLimits = useMemo(() => getPlatformResourceLimits(currentPlatform), [currentPlatform]);
  const rows = useMemo(() => buildContainerRows(items, platformLimits), [items, platformLimits]);
  const visibleColumns = useAtomValue(containerColumnsAtom);
  const cols = useMemo(
    () =>
      columns(actions ?? {}, currentPlatform?.type).filter((column) => {
        const id = column.id ?? ('accessorKey' in column ? String(column.accessorKey) : '');
        return (
          !['containerId', 'image', 'health', 'CPU', 'memory', 'ports'].includes(id) || visibleColumns[id] !== false
        );
      }),
    [actions, currentPlatform?.type, visibleColumns],
  );
  const getSubRows = useCallback(
    (row: ContainerTableRow) => (isContainerStackGroup(row) ? row.containers : undefined),
    [],
  );
  const handleSelectionChange = useCallback(
    (selectedRows: ContainerTableRow[]) => {
      const selectedResources = normalizeContainerSelection(selectedRows);
      const selectionKey = selectedResources.map(getSelectionSignature).sort().join('|');

      if (selectionKey === lastSelectionKeyRef.current) return;

      lastSelectionKeyRef.current = selectionKey;
      setSelectedResources(selectedResources);
    },
    [setSelectedResources],
  );

  return (
    <DataTable
      columns={cols}
      data={rows}
      isLoading={isLoading}
      emptyState={
        isFiltered
          ? {
              title: 'No containers match the current filters.',
              description: 'Adjust the search and filters.',
            }
          : undefined
      }
      getSubRows={getSubRows}
      onSelectionChange={handleSelectionChange}
    />
  );
};

const getSelectionSignature = (resource: ContainerActionResource): string => {
  if (isContainerStackGroup(resource)) {
    return `${resource.id}:${resource.containers.map(getSelectionSignature).sort().join(',')}`;
  }

  return [
    resource.containerId,
    resource.state,
    resource.controlState,
    resource.isSystem,
    resource.capabilities?.canRead,
    resource.capabilities?.canWrite,
    resource.capabilities?.canExecute,
  ].join(':');
};

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: ContainerActionResource; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
  platformType?: PlatformType,
): ColumnDef<ContainerTableRow>[] => {
  const containerBackupColumn = backupColumn<ContainerTableRow>((row) =>
    isContainerStackGroup(row)
      ? undefined
      : {
          coverage: row.backupCoverage,
          source: { $type: 'Container', containerId: row.containerId, platformId: row.platformId },
          returnTo: `/platforms/${row.platformId}/containers/${row.id}`,
        },
  );
  const result: ColumnDef<ContainerTableRow>[] = [
    {
      id: 'select',
      header: ({ table }) => (
        <Checkbox
          checked={table.getIsAllPageRowsSelected() || (table.getIsSomePageRowsSelected() && 'indeterminate')}
          onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
          aria-label="Select all"
        />
      ),
      cell: ({ row }) => (
        <Checkbox
          checked={row.getIsSelected()}
          onCheckedChange={(value) => row.toggleSelected(!!value)}
          aria-label="Select row"
        />
      ),
      enableSorting: false,
      enableHiding: false,
    },
    {
      accessorKey: 'name',
      header: ({ column }) => <SortableCell cellName="Name" column={column} />,
      cell: ({ row }) => (
        <ContainerNameCell row={row.original} depth={row.depth} tableRow={row} platformType={platformType} />
      ),
      sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
        return getDisplayName(rowB.original).localeCompare(getDisplayName(rowA.original));
      },
    },
    {
      id: 'image',
      accessorFn: (row) => (isContainerStackGroup(row) ? '' : (row.imageView?.name ?? '')),
      header: ({ column }) => <SortableCell cellName="Image" column={column} />,
      cell: ({ row }) =>
        isContainerStackGroup(row.original) || !row.original.imageView ? (
          <span className="text-muted-foreground">—</span>
        ) : (
          <ImageName image={row.original.imageView} />
        ),
    },
    {
      id: 'health',
      accessorFn: (row) => (isContainerStackGroup(row) ? (row.healthSummary?.status ?? 'Unknown') : currentHealth(row)),
      header: ({ column }) => <SortableCell cellName="Health" column={column} />,
      cell: ({ row }) =>
        isContainerStackGroup(row.original) ? (
          <ContainerGroupHealthCell name={row.original.name} summary={row.original.healthSummary} />
        ) : (
          <ContainerTableHealth container={row.original} />
        ),
    },
    {
      accessorKey: 'containerId',
      header: ({ column }) => <SortableCell cellName="ID" column={column} />,
      cell: ({ row }) =>
        isContainerStackGroup(row.original) ? (
          <span className="text-muted-foreground">—</span>
        ) : (
          <CopyToClipboard
            textToCopy={row.original.containerId}
            transform={() => row.original.containerId?.slice(0, 12)}
            groupClassName="rowid"
            textClassName="text-sm tabular-nums text-muted-foreground"
          />
        ),
    },
    ...(platformType === PlatformType.DockerSwarm
      ? [
          {
            accessorKey: 'nodeHostname',
            header: ({ column }: any) => <SortableCell cellName="Node" column={column} />,
            cell: ({ row }: any) => {
              if (isContainerStackGroup(row.original))
                return <span className="text-sm text-muted-foreground">Multiple nodes</span>;
              const nodeId = row.original.dockerNodeId;
              return nodeId ? (
                <Link
                  to={`../nodes/${nodeId}`}
                  className="table-link truncate"
                  title={row.original.nodeHostname ?? nodeId}>
                  {row.original.nodeHostname ?? formatId(nodeId)}
                </Link>
              ) : (
                <span className="text-sm text-muted-foreground">Manager</span>
              );
            },
          } as ColumnDef<ContainerTableRow>,
        ]
      : []),
    {
      accessorKey: 'CPU',
      header: ({ column }) => <SortableCell cellName="CPU" column={column} />,
      cell: ({ row }) => (
        <CPUCell
          className="text-sm tabular-nums"
          state={row.original.state}
          stats={row.original.projectionStaleSince ? undefined : row.original.lastStats}
        />
      ),
      sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
        return toNumber(rowA.original.lastStats?.cpuUsage) < toNumber(rowB.original.lastStats?.cpuUsage) ? 1 : -1;
      },
    },
    {
      accessorKey: 'memory',
      header: ({ column }) => <SortableCell cellName="Memory" column={column} />,
      cell: ({ row }) => (
        <MemoryUsageCell
          className="text-sm tabular-nums"
          state={row.original.state}
          stats={row.original.projectionStaleSince ? undefined : row.original.lastStats}
        />
      ),
      sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
        const cA = rowA.original.lastStats as ContainerStatView;
        const cB = rowB.original.lastStats as ContainerStatView;

        return toNumber(cA?.memoryActive) < toNumber(cB?.memoryActive) ? 1 : -1;
      },
    },
    {
      accessorKey: 'ports',
      header: () => <span>Ports</span>,
      cell: ({ row }) =>
        isContainerStackGroup(row.original) ? (
          <span className="text-sm text-muted-foreground">{countPublishedPorts(row.original)} ports</span>
        ) : (
          <PortsDisplay ports={row.original.ports} compact maxVisible={1} className="text-sm tabular-nums" />
        ),
    },
    {
      ...containerBackupColumn,
      cell: (props) => {
        const row = props.row.original;
        if (!isContainerStackGroup(row)) return flexRender(containerBackupColumn.cell, props);
        return row.stackId ? (
          <StackBackupCell stackId={row.stackId} />
        ) : (
          <ResourceBackupCell
            coverage={groupBackupCoverage(row.containers)}
            source={{ $type: 'ContainerGroup', platformId: row.platformId }}
            returnTo={`/platforms/${row.platformId}/containers`}
          />
        );
      },
    },
    {
      id: 'actions',
      cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
    },
  ];

  return result;
};

const ContainerNameCell = ({
  row,
  depth,
  tableRow,
  platformType,
}: {
  row: ContainerTableRow;
  depth: number;
  tableRow: Row<ContainerTableRow>;
  platformType?: PlatformType;
}) => {
  if (isContainerStackGroup(row)) {
    const expanded = tableRow.getIsExpanded();

    return (
      <div className="flex min-w-0 items-center gap-2">
        <button
          type="button"
          onClick={tableRow.getToggleExpandedHandler()}
          className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-muted-foreground hover:bg-accent/70 hover:text-foreground"
          aria-label={expanded ? 'Collapse stack' : 'Expand stack'}>
          {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
        </button>
        <StateIndicator
          value={row.displayStatus === StackReleaseStatus.Healthy ? ContainerStateStatus.Running : row.displayStatus}
          kind={row.displayStatus === StackReleaseStatus.Healthy ? 'container' : undefined}
          tooltip={
            row.displayStatus === StackReleaseStatus.Healthy
              ? 'All containers are running. Health is shown on each container.'
              : undefined
          }
          isProcessing={row.controlState === ResourceControlState.Processing}
        />
        <button
          type="button"
          onClick={tableRow.getToggleExpandedHandler()}
          className="truncate cursor-pointer text-left text-sm font-medium text-foreground hover:underline"
          title={row.name}>
          {truncate(row.name, 28)}
        </button>
        <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
          {row.containers.length} {row.containers.length === 1 ? 'container' : 'containers'}
        </span>
        {row.isSystem ? (
          <SystemContainerBadge stack />
        ) : (
          !row.stackId && (
            <UnmanagedResourceIcon title={row.isSwarmTask ? 'Unmanaged Swarm Stack' : 'Unmanaged stack'} />
          )
        )}
      </div>
    );
  }

  return (
    <div className={cn('flex min-w-0 items-center gap-2', depth > 0 && 'pl-7')}>
      <StateIndicator
        value={row.projectionStaleSince ? ContainerStateStatus.Unknown : (row.state ?? ContainerStateStatus.Exited)}
        isProcessing={row.controlState === ResourceControlState.Processing}
        kind="container"
        tooltip={
          row.projectionStaleSince
            ? (row.projectionStaleReason ?? 'Last-known state; the owning Node Agent is unavailable.')
            : undefined
        }
      />
      <Link to={`./${row.id}`} className="table-link block truncate font-normal" title={row.name}>
        {truncate(getDisplayName(row), 32)}
      </Link>
      {row.isSystem ? (
        <SystemContainerBadge role={row.systemRole} />
      ) : !row.isSwarmTask ? (
        isContainerAdoptionAvailable(row, platformType) && <UnmanagedResourceIcon title="Unmanaged Container" />
      ) : null}
    </div>
  );
};

export const buildContainerRows = (
  containers: ContainerHealthItem[],
  platformLimits: PlatformResourceLimits,
): ContainerTableRow[] => {
  const stackGroups = new Map<string, ContainerHealthItem[]>();
  const orderedRows: Array<{ type: 'stack'; stackName: string } | { type: 'container'; container: ContainerView }> = [];

  for (const container of containers) {
    if (!isVisibleContainer(container)) continue;

    const stackName = container.stack?.trim();

    if (!stackName) {
      orderedRows.push({ type: 'container', container });
      continue;
    }

    if (!stackGroups.has(stackName)) {
      stackGroups.set(stackName, []);
      orderedRows.push({ type: 'stack', stackName });
    }

    stackGroups.get(stackName)?.push(container);
  }

  return orderedRows.flatMap<ContainerTableRow>((row) => {
    if (row.type === 'container') return [row.container];

    const groupContainers = stackGroups.get(row.stackName) ?? [];
    if (groupContainers.length === 0) return [];

    return [createStackGroup(row.stackName, groupContainers, platformLimits)];
  });
};

const createStackGroup = (
  stackName: string,
  containers: ContainerHealthItem[],
  platformLimits: PlatformResourceLimits,
): ContainerStackGroupResource & { healthSummary?: ContainerGroupHealth } => {
  const firstContainer = containers[0];
  const stackIds = Array.from(new Set(containers.map((container) => container.stackId).filter(Boolean)));
  const stackId = stackIds.length === 1 ? stackIds[0] : null;

  return {
    id: `stack:${stackId ?? stackName}`,
    platformId: firstContainer.platformId,
    name: stackName,
    containerId: `stack:${stackId ?? stackName}`,
    state: getStackState(containers),
    controlState: containers.some((container) => container.controlState === ResourceControlState.Processing)
      ? ResourceControlState.Processing
      : ResourceControlState.Idle,
    stackId,
    stack: stackName,
    lastStats: aggregateStats(containers, platformLimits),
    ports: {},
    deploymentId: null,
    isSystem: containers.some((container) => container.isSystem),
    isSwarmTask: containers.length > 0 && containers.every((container) => container.isSwarmTask),
    systemRole: null,
    imageView: null,
    displayStatus: getStackDisplayStatus(containers),
    healthSummary: firstContainer.stackHealth,
    capabilities: firstContainer.capabilities,
    projectionStaleSince: containers.some((container) => container.projectionStaleSince)
      ? (containers.find((container) => container.projectionStaleSince)?.projectionStaleSince ?? null)
      : null,
    containers,
    isStackGroup: true,
  };
};

const getStackDisplayStatus = (containers: ContainerView[]) => {
  if (containers.length === 0) return StackReleaseStatus.Unknown;
  if (containers.some((container) => container.projectionStaleSince)) return StackReleaseStatus.Degraded;
  if (containers.every((container) => container.state === ContainerStateStatus.Running))
    return StackReleaseStatus.Healthy;
  if (containers.every((container) => container.state === ContainerStateStatus.Paused))
    return StackReleaseStatus.Paused;
  if (
    containers.every((container) =>
      [ContainerStateStatus.Exited, ContainerStateStatus.Offline].includes(container.state),
    )
  ) {
    return StackReleaseStatus.Stopped;
  }
  if (
    containers.some((container) => [ContainerStateStatus.Dead, ContainerStateStatus.Offline].includes(container.state))
  ) {
    return StackReleaseStatus.Failed;
  }
  if (
    containers.some((container) =>
      [ContainerStateStatus.Restarting, ContainerStateStatus.Removing, ContainerStateStatus.Created].includes(
        container.state,
      ),
    )
  ) {
    return StackReleaseStatus.Pending;
  }

  return StackReleaseStatus.Degraded;
};

const getStackState = (containers: ContainerView[]) => {
  if (containers.some((container) => container.state === ContainerStateStatus.Running))
    return ContainerStateStatus.Running;
  if (containers.some((container) => container.state === ContainerStateStatus.Restarting)) {
    return ContainerStateStatus.Restarting;
  }
  if (containers.every((container) => container.state === ContainerStateStatus.Paused))
    return ContainerStateStatus.Paused;
  if (containers.some((container) => container.state === ContainerStateStatus.Paused))
    return ContainerStateStatus.Paused;
  if (containers.every((container) => container.state === ContainerStateStatus.Exited))
    return ContainerStateStatus.Exited;
  if (containers.every((container) => container.state === ContainerStateStatus.Offline))
    return ContainerStateStatus.Offline;
  if (containers.some((container) => container.state === ContainerStateStatus.Removing))
    return ContainerStateStatus.Removing;
  if (containers.some((container) => container.state === ContainerStateStatus.Dead)) return ContainerStateStatus.Dead;
  if (containers.some((container) => container.state === ContainerStateStatus.Created))
    return ContainerStateStatus.Created;

  return containers[0]?.state ?? ContainerStateStatus.Unknown;
};

const aggregateStats = (
  containers: ContainerHealthItem[],
  platformLimits: PlatformResourceLimits,
): Omit<ContainerStatView, 'containerId' | 'created'> => {
  containers = containers.filter((container) => !container.projectionStaleSince);
  const memoryLimit = getAggregateMemoryLimit(containers, platformLimits.memoryTotal);
  const cpuLimit = platformLimits.cpuCount > 0 ? platformLimits.cpuCount * 100 : 0;

  return {
    memoryActive: clampToLimit(sumStat(containers, 'memoryActive'), memoryLimit),
    memoryCache: clampToLimit(sumStat(containers, 'memoryCache'), memoryLimit),
    cpuUsage: clampToLimit(sumStat(containers, 'cpuUsage'), cpuLimit),
    memoryLimit,
    rxBytes: sumStat(containers, 'rxBytes'),
    txBytes: sumStat(containers, 'txBytes'),
  };
};

const getAggregateMemoryLimit = (containers: ContainerView[], platformMemoryTotal: number) => {
  const limits = containers.map((container) => toNumber(container.lastStats?.memoryLimit)).filter((limit) => limit > 0);

  if (limits.length === 0) return platformMemoryTotal;

  if (platformMemoryTotal > 0) {
    const sumEffectiveLimits = limits.reduce((sum, limit) => sum + Math.min(limit, platformMemoryTotal), 0);
    return Math.min(sumEffectiveLimits, platformMemoryTotal);
  }

  const firstLimit = limits[0];
  const allLimitsMatch = limits.every((limit) => limit === firstLimit);
  return allLimitsMatch ? firstLimit : limits.reduce((sum, limit) => sum + limit, 0);
};

const getPlatformResourceLimits = (currentPlatform: PlatformView | undefined): PlatformResourceLimits => {
  return {
    memoryTotal: toNumber(currentPlatform?.memTotal),
    cpuCount: toNumber(currentPlatform?.cpuCount),
  };
};

const clampToLimit = (value: number, limit: number) => (limit > 0 ? Math.min(value, limit) : value);

const sumStat = (containers: ContainerView[], key: keyof ContainerStatView) =>
  containers.reduce((sum, container) => sum + toNumber(container.lastStats?.[key]), 0);

const toNumber = (value: number | string | null | undefined) => {
  if (typeof value === 'number') return value;
  if (typeof value === 'string') {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : 0;
  }

  return 0;
};

const getDisplayName = (row: ContainerTableRow) =>
  isContainerStackGroup(row) ? row.name : (row.name?.replace(/^\//, '') ?? '');

const countPublishedPorts = (row: ContainerStackGroupResource) =>
  row.containers.reduce((count, container) => count + Object.keys(container.ports ?? {}).length, 0);
