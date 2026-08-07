import { DataTable } from '@/components/ui/data-table';
import { ColumnDef, Row } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import {
  ContainerView,
  ContainerStateStatus,
  ContainerStatView,
  PlatformView,
  ResourceControlState,
  StackReleaseStatus,
} from '@/api/generated/api.types';
import { truncate } from '@/lib/truncate';
import { cn, formatId, isUnmanagedContainer } from '@/lib/utils';
import SortableCell from '@/components/custom/sortable-cell';
import { Link } from 'react-router';
import { CopyToClipboard } from '@/components/custom/copy-to-clipboard';
import { PortsDisplay } from '@/components/custom/ports-display';
import { ImageName } from './container-info/';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useCallback, useMemo, useRef } from 'react';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { CPUCell, MemoryUsageCell, UnmanagedResourceIcon } from '@/components/custom/common';
import { ChevronDown, ChevronRight } from 'lucide-react';
import { ContainerActionResource, ContainerStackGroupResource, isContainerStackGroup } from './actions';
import { useAppContext } from '@/lib/context/app-context';
import { SystemContainerBadge } from './system-container-badge';
import { normalizeContainerSelection } from './selection';

type ContainerTableRow = ContainerActionResource;
type PlatformResourceLimits = {
  memoryTotal: number;
  cpuCount: number;
};

export const ContainersTable = ({
  items,
  isLoading,
  actions,
}: {
  items: ContainerView[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: ContainerActionResource; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const { currentPlatform } = useAppContext();
  const [_, setSelectedResources] = useSelectedResources<ContainerActionResource>('Container');
  const lastSelectionKeyRef = useRef('');
  const platformLimits = useMemo(
    () => getPlatformResourceLimits(currentPlatform, items ?? []),
    [currentPlatform, items],
  );
  const rows = useMemo(() => buildContainerRows(items ?? [], platformLimits), [items, platformLimits]);
  const cols = useMemo(() => columns(actions ?? {}), [actions]);
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
): ColumnDef<ContainerTableRow>[] => [
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
    cell: ({ row }) => <ContainerNameCell row={row.original} depth={row.depth} tableRow={row} />,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return getDisplayName(rowB.original).localeCompare(getDisplayName(rowA.original));
    },
  },
  {
    accessorKey: 'image',
    header: ({ column }) => <SortableCell cellName="Image" column={column} />,
    cell: ({ row }) => {
      if (isContainerStackGroup(row.original)) {
        return <span className="text-xs text-muted-foreground">{countDistinctImages(row.original)} images</span>;
      }

      return <ImageName image={row.original.imageView ?? undefined} />;
    },
  },
  {
    accessorKey: 'containerId',
    header: ({ column }) => <SortableCell cellName="ID" column={column} />,
    cell: ({ row }) =>
      isContainerStackGroup(row.original) ? (
        <span className="text-xs text-muted-foreground">Stack</span>
      ) : (
        <CopyToClipboard
          textToCopy={row.original.containerId}
          transform={() => row.original.containerId?.slice(0, 12)}
          groupClassName="rowid"
        />
      ),
  },
  {
    accessorKey: 'CPU',
    header: ({ column }) => <SortableCell cellName="Cpu" column={column} />,
    cell: ({ row }) => <CPUCell state={row.original.state} stats={row.original.lastStats} />,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return toNumber(rowA.original.lastStats?.cpuUsage) < toNumber(rowB.original.lastStats?.cpuUsage) ? 1 : -1;
    },
  },
  {
    accessorKey: 'memory',
    header: ({ column }) => <SortableCell cellName="Memory" column={column} />,
    cell: ({ row }) => <MemoryUsageCell state={row.original.state} stats={row.original.lastStats} />,
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
        <span className="text-xs text-muted-foreground">{countPublishedPorts(row.original)} ports</span>
      ) : (
        <PortsDisplay ports={row.original.ports} compact maxVisible={1} />
      ),
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

const ContainerNameCell = ({
  row,
  depth,
  tableRow,
}: {
  row: ContainerTableRow;
  depth: number;
  tableRow: Row<ContainerTableRow>;
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
        <StateIndicator value={row.displayStatus} isProcessing={row.controlState === ResourceControlState.Processing} />
        <button
          type="button"
          onClick={tableRow.getToggleExpandedHandler()}
          className="truncate cursor-pointer text-left text-sm text-foreground hover:underline"
          title={row.name}>
          {truncate(row.name, 28)}
        </button>
        {row.isSystem ? (
          <SystemContainerBadge stack />
        ) : (
          !row.stackId && <UnmanagedResourceIcon title={'Unmanaged stack'} />
        )}
      </div>
    );
  }

  return (
    <div className={cn('flex min-w-0 items-center gap-2', depth > 0 && 'pl-7')}>
      <StateIndicator
        value={row.state ?? ContainerStateStatus.Exited}
        isProcessing={row.controlState === ResourceControlState.Processing}
        kind="container"
      />
      <Link to={`./${formatId(row.containerId)}`} className="table-link truncate" title={row.name}>
        {row.name ? truncate(row.name?.slice(1), 24) : ''}
      </Link>
      {row.isSystem ? (
        <SystemContainerBadge role={row.systemRole} />
      ) : (
        isUnmanagedContainer(row) && <UnmanagedResourceIcon title={'Unmanaged Container'} />
      )}
    </div>
  );
};

const buildContainerRows = (
  containers: ContainerView[],
  platformLimits: PlatformResourceLimits,
): ContainerTableRow[] => {
  const stackGroups = new Map<string, ContainerView[]>();
  const orderedRows: Array<{ type: 'stack'; stackName: string } | { type: 'container'; container: ContainerView }> = [];

  for (const container of containers) {
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

  return orderedRows.flatMap((row) => {
    if (row.type === 'container') return [row.container];

    const groupContainers = stackGroups.get(row.stackName) ?? [];
    if (groupContainers.length === 0) return [];

    return [createStackGroup(row.stackName, groupContainers, platformLimits)];
  });
};

const createStackGroup = (
  stackName: string,
  containers: ContainerView[],
  platformLimits: PlatformResourceLimits,
): ContainerStackGroupResource => {
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
    systemRole: null,
    imageView: null,
    displayStatus: getStackDisplayStatus(containers),
    capabilities: firstContainer.capabilities,
    containers,
    isStackGroup: true,
  };
};

const getStackDisplayStatus = (containers: ContainerView[]) => {
  if (containers.length === 0) return StackReleaseStatus.Unknown;
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

const aggregateStats = (containers: ContainerView[], platformLimits: PlatformResourceLimits): ContainerStatView => {
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

const getPlatformResourceLimits = (
  currentPlatform: PlatformView | undefined,
  containers: ContainerView[],
): PlatformResourceLimits => {
  const platformFromContainer = containers.find((container) => container.platform)?.platform;

  return {
    memoryTotal: toNumber(currentPlatform?.memTotal ?? platformFromContainer?.memTotal),
    cpuCount: toNumber(currentPlatform?.cpuCount ?? platformFromContainer?.cpuCount),
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
  isContainerStackGroup(row) ? row.name : row.name ? row.name.slice(1) : '';

const countDistinctImages = (row: ContainerStackGroupResource) =>
  new Set(row.containers.map((container) => container.imageView?.name ?? container.dockerImageId).filter(Boolean)).size;

const countPublishedPorts = (row: ContainerStackGroupResource) =>
  row.containers.reduce((count, container) => count + Object.keys(container.ports ?? {}).length, 0);
