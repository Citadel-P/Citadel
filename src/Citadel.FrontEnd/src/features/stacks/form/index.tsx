import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { StackForm } from './form';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { StackActions } from './actions';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useStackGroup } from './hooks/useStackGroup';
import {
  ContainerDataView,
  StackView,
  ResourceControlState,
  LatestActivityView,
  ActivityStatus,
  StackReleaseStatus,
  ContainerStateStatus,
  StackDriftMode,
  StackDrift,
} from '@/api/generated/api.types';
import { ActivitiesTab } from '@/features/activities';
import { hasCapability } from '@/lib/resource-capabilities';
import { AlertMessage } from '@/components/custom/alert-message';
import { ActivityAlertZone } from '@/components/custom/task-sheet';
import { useStackInfoGroup } from './hooks/useStackInfoGroup';
import { DataTable } from '@/components/ui/data-table';
import { ColumnDef } from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { PortsDisplay } from '@/components/custom/ports-display';
import { CopyToClipboard } from '@/components/custom/copy-to-clipboard';
import { CPUCell, DockerContainerCell, DockerImageCell, MemoryUsageCell } from '@/components/custom/common';
import { truncate } from '@/lib/truncate';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { StackLogs } from '@/features/docker-resources/containers/container-info/container-logs';
import { StackInspect } from '@/features/docker-resources/containers/container-info/container-inspect';
import { StackExec } from '@/features/docker-resources/containers/container-info/container-exec';
import { StackStats } from '@/features/docker-resources/containers/container-info/stack-stats';
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Button } from '@/components/ui/button';
import { Check, Funnel, Loader2, RefreshCw } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import { useMutate, useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';

export const StackFormComponents: RequiredFormComponents = {
  AddForm: {
    Content: () => {
      return <StackForm mode="add" />;
    },
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }: { resource: RequiredFormFields }) => {
        return (
          <StateIndicator
            value={(resource as StackView).status}
            isProcessing={(resource as StackView).controlState === ResourceControlState.Processing}
          />
        );
      },
      ActionButtons: ({ resource }) => {
        return <GenericActionBarButtons resource={resource} actions={Object.values(StackActions)} />;
      },
    },
    SubHeader: ({ resource }: { resource: StackView }) => {
      return <StackSubHeader latestActivity={resource.latestActivityView ?? null} stack={resource} />;
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource, metadataChanged }: { resource: StackView; metadataChanged?: boolean }) => {
          return (
            <StackForm mode="edit" metadataChanged={metadataChanged} disabled={!hasCapability(resource, 'canWrite')} />
          );
        },
      },
      {
        label: 'Containers',
        disabled: (resource: StackView): boolean => resource.status === StackReleaseStatus.Created,
        Content: ({ resource }: { resource: StackView }) => {
          return <StackRuntime key={resource.id} stack={resource} />;
        },
      },
      {
        label: 'Activities',
        Content: ({ resource }: { resource: StackView }) => {
          return <ActivitiesTab resourceId={resource.id} resourceType="Stack" />;
        },
      },
    ],
    useData: function (id: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { stack, isLoading } = useStackGroup(id);
      return { item: stack, isLoading };
    },
  },
};

const StackSubHeader = ({ latestActivity }: { stack: StackView; latestActivity: LatestActivityView | null }) => {
  return (
    <>
      <StackLatestActivity latestActivity={latestActivity} />
      {/* <StackUpdateNotice stack={stack} /> */}
    </>
  );
};
const StackLatestActivity = ({ latestActivity }: { latestActivity: LatestActivityView | null }) => {
  if (!latestActivity) return;
  if (latestActivity?.status === ActivityStatus.Success) {
    return;
  }
  if (latestActivity?.info.$type === 'StackDegraded' || latestActivity?.info.$type === 'StackDriftDetected') {
    return (
      <AlertMessage date={latestActivity?.createdAt} type={'warning'}>
        <div className="flex flex-wrap gap-2 items-center ">{latestActivity?.info.reason}</div>
      </AlertMessage>
    );
  }
  return (
    <ActivityAlertZone
      info={latestActivity?.info as any}
      activity={latestActivity as any}
      title="Error"
      date={latestActivity?.createdAt}
    />
  );
};

const StackRuntime = ({ stack }: { stack: StackView }) => {
  const { containersInfo, isLoading, error } = useStackInfoGroup(stack.id, stack.platformId ?? undefined, stack.name);

  if (error)
    return (
      <div className="mb-2">
        <AlertMessage type="warning">
          <div className="truncate">{(error as any)?.error?.detail ?? 'Platform unavailable'}</div>
        </AlertMessage>
      </div>
    );

  return (
    <div className="flex w-full flex-col gap-4">
      <StackDriftPanel stack={stack} />
      <StackContainersTable
        containers={containersInfo}
        isLoading={isLoading}
        platformId={stack.platformId ?? undefined}
      />
      <StackRuntimeTabs stack={stack} containers={containersInfo} />
    </div>
  );
};

const StackDriftPanel = ({ stack }: { stack: StackView }) => {
  const queryClient = useQueryClient();
  const driftDetectionDisabled = stack.driftPolicy?.mode === StackDriftMode.Disabled;
  const queryEnabled = !driftDetectionDisabled && stack.status !== StackReleaseStatus.Created;
  const { data, isLoading, isFetching, error, refetch } = useRead(
    'getStackDrift',
    { stackId: stack.id },
    { enabled: queryEnabled },
  );
  const { mutateAsync: reconcileStack, isPending } = useMutate('reconcileStack', {
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['getStack', { stackId: stack.id }] });
      refetch();
    },
  });
  const report = data?.data;

  if (driftDetectionDisabled) {
    return (
      <AlertMessage type="info" title="Drift detection disabled" className="my-0">
        Enable drift management in Config to compare the compose file with the running stack containers.
      </AlertMessage>
    );
  }

  if (!queryEnabled) return null;

  if (error) {
    return (
      <AlertMessage type="warning" title="Drift check failed" className="my-0">
        {(error as any)?.error?.detail ?? 'Unable to check stack drift.'}
      </AlertMessage>
    );
  }

  if (isLoading && !report) {
    return (
      <div className="flex items-center gap-2 rounded-sm border px-3 py-2 text-sm text-muted-foreground">
        <Loader2 className="size-4 animate-spin" />
        Checking stack drift...
      </div>
    );
  }

  if (!report?.hasDrift) {
    return (
      <AlertMessage type="success" title="No drift detected" className="my-0">
        Running containers match the stack compose state.
      </AlertMessage>
    );
  }

  const actionable = report.drifts.some(
    (drift) =>
      (drift.$type === 'ContainerStopped' && stack.driftPolicy.autoStartStoppedContainers) ||
      (drift.$type === 'ContainerPaused' && stack.driftPolicy.autoResumePausedContainers),
  );
  const canReconcile = actionable && !report.hasStructuralDrift && hasCapability(stack, 'canWrite');
  const details = report.drifts.map(formatStackDrift).join('; ');

  return (
    <AlertMessage type="warning" title={`${report.drifts.length} drift item${report.drifts.length === 1 ? '' : 's'}`} className="my-0">
      <div className="flex w-full flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <span className="min-w-0">{details}</span>
        <div className="flex shrink-0 items-center gap-2">
          {report.hasStructuralDrift && <span className="text-xs">Reapply required</span>}
          {report.hasAutoFixableDrift && !actionable && (
            <span className="text-xs">Enable safe auto-fix in Config to reconcile</span>
          )}
          {canReconcile && (
            <Button
              size="sm"
              variant="outline"
              className="h-8 bg-background"
              disabled={isPending || isFetching}
              onClick={() => reconcileStack({ stackId: stack.id })}>
              {isPending ? <Loader2 className="size-3.5 animate-spin" /> : <RefreshCw className="size-3.5" />}
              Reconcile
            </Button>
          )}
        </div>
      </div>
    </AlertMessage>
  );
};

const StackRuntimeTabs = ({ stack, containers }: { stack: StackView; containers: ContainerDataView[] }) => {
  const canViewLogs = hasCapability(stack, 'canViewLogs');
  const canInspect = hasCapability(stack, 'canInspect');
  const canOpenTerminal = hasCapability(stack, 'canOpenTerminal');
  const [inspectContainerId, setInspectContainerId] = useState<string | undefined>();
  const [terminalContainerId, setTerminalContainerId] = useState<string | undefined>();
  const containerNames = useMemo(
    () =>
      containers.map((container) => container.name?.replace(/^\//, '')).filter((name): name is string => Boolean(name)),
    [containers],
  );
  const hasContainers = containers.length > 0;
  const inspectContainer = getSelectedStackContainer(containers, inspectContainerId);
  const terminalContainer = getSelectedStackContainer(containers, terminalContainerId);
  const terminalDisabled =
    !canOpenTerminal ||
    !terminalContainer ||
    terminalContainer.state !== ContainerStateStatus.Running ||
    !hasCapability(terminalContainer, 'canOpenTerminal');
  const defaultValue = canViewLogs
    ? 'logs'
    : canInspect && hasContainers
      ? 'inspect'
      : canOpenTerminal && hasContainers
        ? 'terminal'
        : 'stats';

  return (
    <Tabs defaultValue={defaultValue} className="w-full">
      <TabsList className="w-fit justify-start">
        <TabsTrigger className="text-xs" value="logs" disabled={!canViewLogs}>
          Logs
        </TabsTrigger>
        <TabsTrigger className="text-xs" value="inspect" disabled={!canInspect || !hasContainers}>
          Inspect
        </TabsTrigger>
        <TabsTrigger className="text-xs" value="terminal" disabled={!canOpenTerminal || !hasContainers}>
          Terminal
        </TabsTrigger>
        <TabsTrigger className="text-xs" value="stats">
          Stats
        </TabsTrigger>
      </TabsList>
      {canViewLogs && (
        <TabsContent value="logs" className="mt-2 w-full">
          <StackLogs key={stack.id} stackId={stack.id} containers={containerNames} />
        </TabsContent>
      )}
      {canInspect && (
        <TabsContent value="inspect" className="mt-2 w-full">
          <StackContainerInspect
            stackId={stack.id}
            containers={containers}
            selectedContainer={inspectContainer}
            onSelectContainer={setInspectContainerId}
          />
        </TabsContent>
      )}
      {canOpenTerminal && (
        <TabsContent value="terminal" className="mt-2 w-full">
          <StackContainerTerminal
            stackId={stack.id}
            containers={containers}
            selectedContainer={terminalContainer}
            onSelectContainer={setTerminalContainerId}
            disabled={terminalDisabled}
          />
        </TabsContent>
      )}
      <TabsContent value="stats" className="mt-2 w-full">
        <StackStats key={stack.id} stackId={stack.id} containers={containers} />
      </TabsContent>
    </Tabs>
  );
};

const StackContainerInspect = ({
  stackId,
  containers,
  selectedContainer,
  onSelectContainer,
}: {
  stackId: string;
  containers: ContainerDataView[];
  selectedContainer?: ContainerDataView;
  onSelectContainer: (containerId: string) => void;
}) => {
  if (!selectedContainer) {
    return <div className="text-sm text-muted-foreground">No containers available</div>;
  }

  return (
    <div className="relative">
      <StackInspectContainerFilter
        containers={containers}
        selectedContainer={selectedContainer}
        onSelectContainer={onSelectContainer}
      />
      <StackInspect key={`${stackId}-${selectedContainer.id}`} stackId={stackId} containerId={selectedContainer.id} />
    </div>
  );
};

const StackContainerTerminal = ({
  stackId,
  containers,
  selectedContainer,
  onSelectContainer,
  disabled,
}: {
  stackId: string;
  containers: ContainerDataView[];
  selectedContainer?: ContainerDataView;
  onSelectContainer: (containerId: string) => void;
  disabled?: boolean;
}) => {
  if (!selectedContainer) {
    return <div className="text-sm text-muted-foreground">No containers available</div>;
  }

  return (
    <div className="flex flex-col gap-3">
      {selectedContainer.state !== ContainerStateStatus.Running && (
        <AlertMessage type="warning">
          <div className="truncate">Selected container is not running</div>
        </AlertMessage>
      )}
      <StackExec
        key={`${stackId}-${selectedContainer.id}`}
        stackId={stackId}
        containerId={selectedContainer.id}
        disabled={disabled}
        toolbarStart={
          <StackTerminalContainerSelect
            containers={containers}
            selectedContainer={selectedContainer}
            onSelectContainer={onSelectContainer}
          />
        }
      />
    </div>
  );
};

const StackTerminalContainerSelect = ({
  containers,
  selectedContainer,
  onSelectContainer,
}: {
  containers: ContainerDataView[];
  selectedContainer: ContainerDataView;
  onSelectContainer: (containerId: string) => void;
}) => {
  return (
    <Select value={selectedContainer.id} onValueChange={onSelectContainer}>
      <SelectTrigger className="h-8 min-w-42 rounded-sm bg-background shadow-xs">
        <SelectValue placeholder="Select a container" />
      </SelectTrigger>
      <SelectContent className="bg-background">
        <SelectGroup>
          {containers.map((container) => (
            <SelectItem key={container.id} value={container.id}>
              {getStackContainerLabel(container)}
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  );
};

const StackInspectContainerFilter = ({
  containers,
  selectedContainer,
  onSelectContainer,
}: {
  containers: ContainerDataView[];
  selectedContainer: ContainerDataView;
  onSelectContainer: (containerId: string) => void;
}) => {
  return (
    <div className="absolute top-4 right-6 z-10">
      <Popover>
        <TooltipProvider delayDuration={200}>
          <Tooltip>
            <TooltipTrigger asChild>
              <PopoverTrigger asChild>
                <Button size="icon-sm" variant="outline" className="h-7 w-7 rounded-full bg-background shadow-sm">
                  <Funnel className="h-3.5 w-3.5" />
                </Button>
              </PopoverTrigger>
            </TooltipTrigger>
            <TooltipContent side="left">Container</TooltipContent>
          </Tooltip>
        </TooltipProvider>
        <PopoverContent align="end" side="left" className="w-64 p-2 bg-background">
          <div className="px-2 pb-2 text-xs font-medium text-muted-foreground">Inspect container</div>
          <div className="max-h-64 overflow-auto">
            {containers.map((container) => {
              const selected = selectedContainer.id === container.id;
              return (
                <button
                  type="button"
                  key={container.id}
                  className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent"
                  onClick={() => onSelectContainer(container.id)}>
                  <span
                    className={cn(
                      'flex size-4 shrink-0 items-center justify-center border',
                      selected && 'bg-primary text-primary-foreground',
                    )}>
                    {selected && <Check className="size-3" />}
                  </span>
                  <span className="truncate" title={getStackContainerLabel(container)}>
                    {getStackContainerLabel(container)}
                  </span>
                </button>
              );
            })}
          </div>
        </PopoverContent>
      </Popover>
    </div>
  );
};

const formatStackDrift = (drift: StackDrift) => {
  switch (drift.$type) {
    case 'MissingContainer':
      return `${drift.serviceName} is missing`;
    case 'ExtraContainer':
      return `${drift.serviceName} has an extra container`;
    case 'ContainerStopped':
      return `${drift.serviceName} is stopped`;
    case 'ContainerPaused':
      return `${drift.serviceName} is paused`;
    case 'ContainerUnhealthy':
      return `${drift.serviceName} is unhealthy${drift.healthStatus ? ` (${drift.healthStatus})` : ''}`;
    case 'ImageMismatch':
      return `${drift.serviceName} image changed`;
    case 'ConfigHashMismatch':
      return `${drift.serviceName} config changed`;
    default:
      return 'Unknown drift';
  }
};

const getSelectedStackContainer = (containers: ContainerDataView[], selectedContainerId?: string) => {
  return containers.find((container) => container.id === selectedContainerId) ?? containers[0];
};

const getStackContainerLabel = (container: ContainerDataView) => {
  const name = container.name?.replace(/^\//, '');
  if (name) return name;

  return container.id?.slice(0, 12) ?? 'Container';
};

const StackContainersTable = ({
  containers,
  isLoading,
  platformId,
}: {
  containers: ContainerDataView[];
  isLoading: boolean;
  platformId?: string;
}) => {
  const columns = useMemo(() => getStackContainerColumns(platformId), [platformId]);

  return (
    <div className="rounded-sm border p-1 shadow-xs">
      <DataTable columns={columns} data={containers} isLoading={isLoading} />
    </div>
  );
};

const getStackContainerColumns = (platformId?: string): ColumnDef<ContainerDataView>[] => [
  {
    accessorKey: 'name',
    header: () => <span>Name</span>,
    cell: ({ row }) =>
      platformId ? (
        <DockerContainerCell
          name={row.original.name}
          id={row.original.id}
          state={row.original.state}
          platformId={platformId}
        />
      ) : (
        <span>{truncate(row.original.name?.replace(/^\//, '') ?? '', 24)}</span>
      ),
  },
  {
    accessorKey: 'image',
    header: () => <span>Image</span>,
    cell: ({ row }) => (
      <DockerImageCell>
        <span title={row.original.image}>{truncate(row.original.image || row.original.imageId, 32)}</span>
      </DockerImageCell>
    ),
  },
  {
    accessorKey: 'id',
    header: () => <span>ID</span>,
    cell: ({ row }) => (
      <CopyToClipboard
        textToCopy={row.original.id}
        transform={() => row.original.id?.slice(0, 12)}
        groupClassName="rowid"
      />
    ),
  },
  {
    accessorKey: 'ports',
    header: () => <span>Ports</span>,
    cell: ({ row }) => <PortsDisplay ports={row.original.ports ?? {}} />,
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
