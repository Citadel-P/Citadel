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
  StackReleaseView,
  StackSource,
  ActorType,
  ResourceBindingScope,
} from '@/api/generated/api.types';
import { ActivitiesTab } from '@/features/activities';
import { hasCapability } from '@/lib/resource-capabilities';
import { AlertMessage } from '@/components/custom/alert-message';
import { ActivityAlertZone } from '@/components/custom/task-sheet';
import { useStackInfoGroup } from './hooks/useStackInfoGroup';
import { DataTable } from '@/components/ui/data-table';
import { ColumnDef } from '@tanstack/react-table';
import { ReactNode, useMemo, useState } from 'react';
import { PortsDisplay } from '@/components/custom/ports-display';
import { CopyToClipboard } from '@/components/custom/copy-to-clipboard';
import {
  CPUCell,
  DockerContainerCell,
  DockerImageCell,
  MemoryUsageCell,
  UpdateAvailableNotice,
} from '@/components/custom/common';
import { truncate } from '@/lib/truncate';
import { formatId } from '@/lib/utils';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { StackLogs } from '@/features/docker-resources/containers/container-info/container-logs';
import { StackInspect } from '@/features/docker-resources/containers/container-info/container-inspect';
import { StackExec } from '@/features/docker-resources/containers/container-info/container-exec';
import { StackStats } from '@/features/docker-resources/containers/container-info/stack-stats';
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Button } from '@/components/ui/button';
import {
  ArrowRight,
  Calendar,
  Check,
  Copy,
  Eye,
  FileText,
  Folder,
  Funnel,
  GitBranch,
  GitCommitHorizontal,
  Route,
  RotateCcw,
  Settings,
  User,
} from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import { useRead } from '@/lib/hooks';
import { useTaskSheet } from '@/lib/atoms';
import { fromNow } from '@/lib/dayjs.helper';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { MonacoDiff } from '@/lib/monaco';
import { ActionWithDialog } from '@/components/custom/action-with-dialog';
import { hasActionableStackDrift } from '../actions';
import { ResourceBindingsTab } from '@/components/custom/resource-bindings-tab';
import { ResourceHeaderTagsEditor } from '@/features/tags/components';
import { useNavigate } from 'react-router';

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
        const groupedActions = Object.values(StackActions).filter((action) => action !== StackActions.checkUpdates);
        return (
          <GenericActionBarButtons
            resource={resource}
            actions={groupedActions}
            standaloneActions={[StackActions.checkUpdates]}
          />
        );
      },
      Tags: ({ resource }: { resource: StackView }) => (
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <ResourceHeaderTagsEditor
            resourceType="Stack"
            resourceId={resource.id}
            tags={resource.tags}
            disabled={!hasCapability(resource, 'canWrite')}
          />
          <DuplicateStackConfigButton stack={resource} />
        </div>
      ),
    },
    SubHeader: ({ resource }: { resource: StackView }) => {
      return <StackSubHeader latestActivity={resource.latestActivityView ?? null} stack={resource} />;
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource, metadataChanged }: { resource: StackView; metadataChanged?: boolean }) => {
          return <StackConfigTab stack={resource} metadataChanged={metadataChanged} />;
        },
      },
      {
        label: 'Services',
        disabled: (resource: StackView): boolean => resource.status === StackReleaseStatus.Created,
        Content: ({ resource }: { resource: StackView }) => {
          return <StackRuntime key={resource.id} stack={resource} />;
        },
      },
      {
        label: 'Releases',
        disabled: (resource: StackView): boolean => !hasCapability(resource, 'canViewReleases'),
        Content: ({ resource }: { resource: StackView }) => {
          return <StackReleasesTab stack={resource} />;
        },
      },
      {
        label: 'Bindings',
        disabled: (resource: StackView): boolean => !hasCapability(resource, 'canViewResourceBindings'),
        Content: ({ resource }: { resource: StackView }) => {
          return (
            <ResourceBindingsTab
              scope={ResourceBindingScope.Stack}
              resourceId={resource.id}
              disabled={!hasCapability(resource, 'canWrite')}
            />
          );
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

const StackConfigTab = ({ stack, metadataChanged }: { stack: StackView; metadataChanged?: boolean }) => {
  return <StackForm mode="edit" metadataChanged={metadataChanged} disabled={!hasCapability(stack, 'canWrite')} />;
};

const DuplicateStackConfigButton = ({ stack }: { stack: StackView }) => {
  const navigate = useNavigate();
  const disabled = !hasCapability(stack, 'canRead') || !hasCapability(stack, 'canWrite');

  return (
    <Button
      type="button"
      variant="outline"
      size="sm"
      className="h-8 rounded-sm text-xs"
      disabled={disabled}
      onClick={() => navigate(`/stacks/add?duplicateFrom=${stack.id}`)}>
      <Copy className="size-3.5" />
      Duplicate Config
    </Button>
  );
};

const StackSubHeader = ({ latestActivity, stack }: { stack: StackView; latestActivity: LatestActivityView | null }) => {
  return (
    <>
      <StackLatestActivity latestActivity={latestActivity} stack={stack} />
      <StackUpdateNotice stack={stack} />
      <StackDriftPanel stack={stack} />
    </>
  );
};

const StackUpdateNotice = ({ stack }: { stack: StackView }) => {
  const updateState = stack.stackUpdateState;
  const gitState =
    updateState && 'recreateStackOnNewCommitState' in updateState ? updateState.recreateStackOnNewCommitState : null;

  if (
    gitState?.remoteCommitSha &&
    gitState.currentCommitSha &&
    gitState.remoteCommitSha !== gitState.currentCommitSha
  ) {
    return (
      <UpdateAvailableNotice
        title="Git update available:"
        actionLabel="Deploy"
        targetLabel="stack"
        sourceLabel={`${stack.source?.gitRepositoryName ?? 'Repository'}${stack.source?.branch ? `/${stack.source.branch}` : ''}`}
        sourceTitle={stack.source?.gitRepositoryName ?? undefined}
        currentLabel={formatId(gitState.currentCommitSha)}
        nextLabel={formatId(gitState.remoteCommitSha)}
        currentTitle={gitState.currentCommitSha}
        nextTitle={gitState.remoteCommitSha}
      />
    );
  }

  const updates = stack.stackUpdateState?.recreateStackOnNewImageState?.autoUpdateStates?.filter(
    (state) => state.updateAvailable,
  );

  if (!updates?.length) return null;

  return (
    <AlertMessage type="info" title={updates.length === 1 ? 'Image update available' : 'Image updates available'}>
      <div className="flex min-w-0 flex-col gap-2">
        <span>Click Redeploy to apply {updates.length === 1 ? 'this update' : 'these updates'}.</span>
        <div className="flex min-w-0 flex-col gap-1.5">
          {updates.map((state) => (
            <div
              key={`${state.serviceName}-${state.imageName}`}
              className="flex min-w-0 flex-wrap items-center gap-1.5 text-xs">
              <span className="font-medium text-foreground">{state.serviceName}</span>
              <span className="max-w-72 truncate" title={state.imageName}>
                {state.imageName}
              </span>
              <span className="font-mono text-muted-foreground" title={state.currentDigest}>
                {formatId(state.currentDigest)}
              </span>
              <ArrowRight className="h-3 w-3 shrink-0 text-muted-foreground" />
              <span className="font-mono text-amber-700 dark:text-amber-500" title={state.remoteDigest ?? undefined}>
                {formatId(state.remoteDigest ?? undefined)}
              </span>
            </div>
          ))}
        </div>
      </div>
    </AlertMessage>
  );
};
const StackLatestActivity = ({
  latestActivity,
  stack,
}: {
  latestActivity: LatestActivityView | null;
  stack: StackView;
}) => {
  if (!latestActivity) return;
  if (latestActivity?.status === ActivityStatus.Success) {
    return;
  }
  if (latestActivity?.info.$type === 'StackDegraded' || latestActivity?.info.$type === 'StackDriftDetected') {
    if (stack.status !== StackReleaseStatus.Degraded) return null;

    return (
      <AlertMessage date={latestActivity?.createdAt} type={'warning'}>
        <div className="flex flex-wrap gap-2 items-center ">{latestActivity?.info.reason}</div>
      </AlertMessage>
    );
  }
  if (latestActivity?.info.$type === 'StackGitUpdateAvailable') {
    return null;
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
  const { containersInfo, isLoading, error } = useStackInfoGroup(stack.id, stack.platformId ?? undefined);

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
      <StackContainersTable
        containers={containersInfo}
        isLoading={isLoading}
        platformId={stack.platformId ?? undefined}
      />
      <StackRuntimeTabs stack={stack} containers={containersInfo} />
    </div>
  );
};

const StackReleasesTab = ({ stack }: { stack: StackView }) => {
  const { data, isLoading } = useRead('listStackReleases', { stackId: stack.id });
  const { open: openSheet } = useTaskSheet('Stack');
  const [previewRelease, setPreviewRelease] = useState<StackReleaseView | null>(null);
  const releases = data?.data.releases ?? [];
  const canRollback = hasCapability(stack, 'canApply') && stack.controlState !== ResourceControlState.Processing;

  const columns = useMemo<ColumnDef<StackReleaseView>[]>(
    () => [
      {
        accessorKey: 'version',
        header: () => <span>Version</span>,
        cell: ({ row }) => <span className="font-normal">v{row.original.version}</span>,
      },

      {
        accessorKey: 'source',
        header: () => <span>Source</span>,
        cell: ({ row }) => <StackReleaseSourceCell release={row.original} />,
      },
      {
        accessorKey: 'actorName',
        header: () => <span>Created By</span>,
        cell: ({ row }) => <StackReleaseActorCell release={row.original} />,
      },
      {
        accessorKey: 'createdAt',
        header: () => <span>Created At</span>,
        cell: ({ row }) => <span className="text-[13px]">{fromNow(row.original.createdAt)}</span>,
      },
      {
        id: 'actions',
        header: () => <span className="sr-only">Actions</span>,
        cell: ({ row }) => {
          const release = row.original;
          const canRollbackRelease = canRollback && isRollbackCandidateRelease(release);

          return (
            <div className="flex items-center justify-end gap-2">
              <Button
                type="button"
                size="icon"
                variant="outline"
                title="Preview Config"
                onClick={() => setPreviewRelease(release)}>
                <Eye className="size-3.5 " />
              </Button>
              <ActionWithDialog
                name={stack.name}
                title="Rollback"
                icon={<RotateCcw className="size-3 " />}
                iconPosition="left"
                variant="outline"
                disabled={!canRollbackRelease}
                targetClassName="h-8 max-w-none flex-none px-3 font-normal "
                onClick={() =>
                  openSheet({
                    kind: 'stackRollback',
                    payload: {
                      stackId: stack.id,
                      releaseId: release.id,
                      name: stack.name,
                      version: release.version,
                    },
                  })
                }
              />
            </div>
          );
        },
      },
    ],
    [canRollback, openSheet, stack.id, stack.name],
  );

  return (
    <div className="flex w-full flex-col gap-3">
      {releases.length === 0 && !isLoading ? (
        <div className="rounded-sm border border-dashed px-4 py-6 text-sm text-center text-muted-foreground">
          No rollback targets yet. Deploy a new successful version to make the previous release available here.
        </div>
      ) : (
        <div className="rounded-sm border p-1 shadow-xs">
          <DataTable columns={columns} data={releases} isLoading={isLoading} />
        </div>
      )}

      <Sheet open={!!previewRelease} onOpenChange={(open) => !open && setPreviewRelease(null)}>
        <SheetContent
          onOpenAutoFocus={(event) => event.preventDefault()}
          side="top"
          className="mx-auto w-300 max-w-[100vw] rounded-b-md">
          <SheetHeader>
            <SheetTitle>Release v{previewRelease?.version} configuration</SheetTitle>
            <SheetDescription>
              Snapshot captured from a previously healthy stack release. Rollback applies this configuration.
            </SheetDescription>
          </SheetHeader>
          {previewRelease && (
            <div className="flex flex-col gap-4 p-4 pt-0 pb-2">
              <StackReleaseSourceDetails release={previewRelease} />
              <MonacoDiff
                original={createStackPreviewConfig(stack)}
                modified={createStackReleasePreviewConfig(stack, previewRelease)}
                format="yaml"
                title="Configuration snapshot"
              />
            </div>
          )}
        </SheetContent>
      </Sheet>
    </div>
  );
};

const createStackPreviewConfig = (stack: StackView) => ({
  name: stack.name,
  description: stack.description,
  stackSource: stack.stackSource,
  platformId: stack.platformId,
  spec: stack.spec,
  resourceBindings: stack.resourceBindings,
});

const createStackReleasePreviewConfig = (stack: StackView, release: StackReleaseView) => ({
  name: stack.name,
  description: stack.description,
  stackSource: stack.stackSource,
  platformId: release.platformId,
  spec: release.spec,
  resourceBindings: release.resourceBindings,
});

const StackReleaseActorCell = ({ release }: { release: StackReleaseView }) => (
  <span className="flex min-w-0 items-center gap-2 text-sm " title={release.createdByActorId}>
    <span className="shrink-0 text-foreground/80 ">{getReleaseActorIcon(release.actorType)}</span>
    <span className="truncate text-[13px]">{release.actorName}</span>
  </span>
);

const StackReleaseSourceCell = ({ release }: { release: StackReleaseView }) => {
  const source = release.source;

  if (!source) {
    return <span className="text-sm text-muted-foreground">Stored spec</span>;
  }

  if (source.sourceType === StackSource.Git) {
    return (
      <div className="flex min-w-0 flex-col py-1 gap-1.5">
        <span className="truncate text-sm font-normal flex flex-row gap-1.5 items-center">
          <GitBranch className="w-3.5 h-3.5 text-muted-foreground" />
          {source.gitRepositoryName ?? 'Git repository'}
          {source.branch ? `/${source.branch}` : ''}
        </span>
        <div className="flex min-w-0 flex-wrap gap-1.5">
          {source.resolvedCommitSha ? (
            <SourceBadge value={formatId(source.resolvedCommitSha)} title={source.resolvedCommitSha} />
          ) : null}
          {source.workingDirectory ? (
            <SourceBadge value={source.workingDirectory} title={`Working directory: ${source.workingDirectory}`} />
          ) : null}
          {source.composePaths?.length ? (
            <SourceBadge value={`${source.composePaths.length} compose`} title={source.composePaths.join('\n')} />
          ) : null}
          {source.envFilePaths?.length ? (
            <SourceBadge value={`${source.envFilePaths.length} env`} title={source.envFilePaths.join('\n')} />
          ) : null}
          {source.watchPaths?.length ? (
            <SourceBadge value={`${source.watchPaths.length} watch`} title={source.watchPaths.join('\n')} />
          ) : null}
        </div>
      </div>
    );
  }

  return <span className="text-sm text-muted-foreground">{source.sourceType}</span>;
};

const StackReleaseSourceDetails = ({ release }: { release: StackReleaseView }) => {
  const source = release.source;

  if (!source || source.sourceType !== StackSource.Git) {
    return (
      <div className="grid gap-3 text-sm">
        <ReleaseMetaItem
          icon={getReleaseActorIcon(release.actorType)}
          value={release.actorName}
          title={release.createdByActorId}
        />
        <ReleaseMetaItem
          icon={<Calendar className="size-3.5" />}
          value={fromNow(release.createdAt)}
          title={String(release.createdAt)}
        />
        <ReleaseMetaItem icon={<FileText className="size-3.5" />} value={source?.sourceType ?? 'Stored spec'} />
      </div>
    );
  }

  return (
    <div className="grid gap-x-8 gap-y-3 text-sm md:grid-cols-2">
      <div className="grid content-start gap-3">
        <ReleaseMetaItem
          icon={getReleaseActorIcon(release.actorType)}
          value={release.actorName}
          title={release.createdByActorId}
        />
        <ReleaseMetaItem
          icon={<Calendar className="size-3.5" />}
          value={fromNow(release.createdAt)}
          title={String(release.createdAt)}
        />
        <ReleaseMetaItem icon={<GitBranch className="size-3.5" />} value={source.gitRepositoryName ?? '-'} />
        <ReleaseMetaItem icon={<Route className="size-3.5" />} value={source.branch ?? '-'} />
        <ReleaseMetaItem
          icon={<GitCommitHorizontal className="size-3.5" />}
          value={source.resolvedCommitSha ? formatId(source.resolvedCommitSha) : '-'}
          title={source.resolvedCommitSha}
        />
      </div>
      <div className="grid content-start gap-3">
        <SourcePathRow
          icon={<Folder className="size-3.5" />}
          paths={source.workingDirectory ? [source.workingDirectory] : []}
        />
        <SourcePathRow icon={<FileText className="size-3.5" />} paths={source.composePaths ?? []} />
        <SourcePathRow icon={<FileText className="size-3.5" />} paths={source.envFilePaths ?? []} />
        <SourcePathRow icon={<FileText className="size-3.5" />} paths={source.watchPaths ?? []} />
      </div>
    </div>
  );
};

const ReleaseMetaItem = ({ icon, value, title }: { icon: ReactNode; value: string; title?: string | null }) => (
  <div className="flex min-w-0 items-center gap-2">
    <span className="shrink-0 text-foreground/80">{icon}</span>
    <span className="min-w-0 truncate text-muted-foreground" title={title ?? value}>
      {value}
    </span>
  </div>
);

const SourcePathRow = ({ icon, paths }: { icon: ReactNode; paths: string[] }) => (
  <div className="flex min-w-0 items-center gap-2">
    <span className="shrink-0 text-foreground/80">{icon}</span>
    {paths.length ? (
      <div className="flex min-w-0 flex-wrap gap-1.5">
        {paths.map((path) => (
          <SourceBadge key={path} value={path} title={path} />
        ))}
      </div>
    ) : (
      <span className="text-muted-foreground">-</span>
    )}
  </div>
);

const getReleaseActorIcon = (actorType: ActorType) =>
  actorType === ActorType.System ? <Settings className="size-3.5" /> : <User className="size-3.5" />;

const SourceBadge = ({ value, title }: { value: string; title?: string }) => (
  <span className="max-w-full truncate rounded-xs bg-muted/25 px-2 py-0.5 text-xs" title={title ?? value}>
    {value}
  </span>
);

const isRollbackCandidateRelease = (release: StackReleaseView) => release.status === StackReleaseStatus.Healthy;

const StackDriftPanel = ({ stack }: { stack: StackView }) => {
  const driftDetectionDisabled = stack.driftPolicy?.mode === StackDriftMode.Disabled;
  const driftEligibleStatus =
    stack.status === StackReleaseStatus.Healthy || stack.status === StackReleaseStatus.Degraded;
  const queryEnabled = !driftDetectionDisabled && driftEligibleStatus;
  const { data, error } = useRead('getStackDrift', { stackId: stack.id }, { enabled: queryEnabled });
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

  if (!report?.hasDrift) return;

  const actionable = hasActionableStackDrift(stack, report);
  const details = report.drifts.map(formatStackDrift).join('; ');

  return (
    <AlertMessage
      type="warning"
      title={`${report.drifts.length} drift item${report.drifts.length === 1 ? '' : 's'}`}
      className="my-0">
      <div className="flex w-full flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <span className="min-w-0">{details}</span>
        <div className="flex shrink-0 items-center gap-2">
          {report.hasStructuralDrift && <span className="text-xs">Reapply required</span>}
          {report.hasAutoFixableDrift && !actionable && (
            <span className="text-xs">Enable safe auto-fix in Config to reconcile</span>
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
      <TabsList className="w-fit max-w-full overflow-x-auto">
        <TabsTrigger value="logs" disabled={!canViewLogs}>
          Logs
        </TabsTrigger>
        <TabsTrigger value="inspect" disabled={!canInspect || !hasContainers}>
          Inspect
        </TabsTrigger>
        <TabsTrigger value="terminal" disabled={!canOpenTerminal || !hasContainers}>
          Terminal
        </TabsTrigger>
        <TabsTrigger value="stats">Stats</TabsTrigger>
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

  return <DataTable columns={columns} data={containers} isLoading={isLoading} />;
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
