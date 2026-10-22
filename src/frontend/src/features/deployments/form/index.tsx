import { WorkloadNetworkAction } from '@/features/docker-resources/networks/workload-network-action';
import { WorkloadHealthCard, WorkloadContainerHealthCard } from '@/features/workload-health/card';
import { DuplicateWorkload } from '@/features/resource-copies/duplicate-workload';
import { ResourceCopyKind } from '@/api/generated/api.types';
import { ResourceBackupIndicator } from '@/features/backup-policies/resource-backup-indicator';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredFormComponents, ResourceFormDataHookResult, RequiredFormFields } from '@/pages/types';
import { DeploymentForm } from './form';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { DeploymentActions } from './actions';
import { StateBadge } from '@/components/custom/state-badge';
import { useDeploymentGroup } from './hooks/useDeploymentGroup';
import {
  ActivityStatus,
  AutoUpdateStatus,
  DeploymentStatus,
  DeploymentView,
  ContainerRuntimeView,
  ContainerSummaryView,
  ContainerStateStatus,
  ResourceBindingScope,
  LatestActivityView,
  PlatformStatus,
  ResourceControlState,
  UpdateBehavior,
} from '@/api/generated/api.types';
import { TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { TabsWithActions as Tabs, TabActionsSlot } from '@/components/custom/tab-actions';
import { DeploymentInspect } from '@/features/docker-resources/containers/container-info/container-inspect';
import { formatId, normalizeDockerId } from '@/lib/utils';
import { DeploymentContainerInfoTable } from '@/features/docker-resources/containers/container-info/container-info-table';
import { useContainerInfoGroup } from '@/features/docker-resources/containers/hooks/useContainerInfoGroup';
import { DeploymentExec } from '@/features/docker-resources/containers/container-info/container-exec';
import { DeploymentStats } from '@/features/docker-resources/containers/container-info/container-stats';
import { ActivitiesTab } from '@/features/activities';
import { AlertMessage } from '@/components/custom/alert-message';
import { ActivityAlertZone } from '@/components/custom/task-sheet';
import { DeploymentLogs } from '@/features/docker-resources/containers/container-info/container-logs';
import { hasCapability } from '@/lib/resource-capabilities';
import { UpdateAvailableNotice } from '@/components/custom/common';
import { ResourceBindingsTab } from '@/components/custom/resource-bindings-tab';
import { useState } from 'react';
import { useRead } from '@/lib/hooks';
import { canShowCachedResource, requestErrorMessage, requestErrorStatus } from '@/lib/request-error';
import Loader from '@/components/ui/loader';
import { ResourceHeaderTagsEditor } from '@/features/tags/components';

export const DeploymentFormComponents: RequiredFormComponents = {
  AddForm: {
    Content: () => {
      return <DeploymentForm mode="add" />;
    },
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }: { resource: RequiredFormFields }) => {
        return (
          <StateBadge
            indicator={<StateIndicator value={(resource as DeploymentView).status} className="mr-0" />}
            value={(resource as DeploymentView).status}
            isProcessing={(resource as DeploymentView).controlState === ResourceControlState.Processing}
          />
        );
      },
      ActionButtons: ({ resource }) => {
        const groupedActions = Object.values(DeploymentActions).filter(
          (action) => action !== DeploymentActions.checkUpdates,
        );
        return <GenericActionBarButtons resource={resource} actions={groupedActions} />;
      },
      Tags: ({ resource }: { resource: DeploymentView }) => (
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <ResourceHeaderTagsEditor
            resourceType="Deployment"
            resourceId={resource.id}
            tags={resource.tags}
            disabled={!hasCapability(resource, 'canWrite')}
          />
          <DuplicateDeploymentConfigButton deployment={resource} />
          <DeploymentActions.checkUpdates resource={resource} />
        </div>
      ),
    },
    SubHeader: ({ resource }: { resource: DeploymentView }) => {
      return <DeploymentSubHeader latestActivity={resource.latestActivityView ?? null} deployment={resource} />;
    },
    TabActions: ({ resource }: { resource: DeploymentView }) => (
      <div className="flex items-center gap-3">
        <WorkloadNetworkAction resource={resource} kind="Deployment" />
        <WorkloadHealthCard resource={resource} kind="Deployment" triggerVariant="icon" />
        <ResourceBackupIndicator
          source={{ $type: 'Deployment', deploymentId: resource.id }}
          coverage={resource.backupCoverage}
        />
      </div>
    ),
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource, metadataChanged }: { resource: DeploymentView; metadataChanged?: boolean }) => {
          return (
            <DeploymentForm
              mode="edit"
              metadataChanged={metadataChanged}
              disabled={!hasCapability(resource, 'canWrite')}
            />
          );
        },
      },
      {
        label: 'Container',
        Content: ({ resource }: { resource: DeploymentView }) => {
          return (
            <DeploymentRuntime
              key={`${resource.id}:${resource.platformId}:${resource.dockerContainerId ?? 'none'}`}
              deployment={resource}
            />
          );
        },
      },
      {
        label: 'Bindings',
        disabled: (resource: DeploymentView): boolean => !hasCapability(resource, 'canViewResourceBindings'),
        Content: ({ resource }: { resource: DeploymentView }) => {
          return (
            <ResourceBindingsTab
              scope={ResourceBindingScope.Deployment}
              resourceId={resource.id}
              disabled={!hasCapability(resource, 'canWrite')}
            />
          );
        },
      },
      {
        label: 'Activities',
        Content: ({ resource }: { resource: DeploymentView }) => {
          return <ActivitiesTab resourceId={resource.id} resourceType="Deployment" />;
        },
      },
    ],
    useData: function (id: string): ResourceFormDataHookResult {
      const { deployment, isLoading, error, refetch, isFetching } = useDeploymentGroup(id);
      return { error, refetch, isFetching, item: deployment, isLoading };
    },
  },
};

const DuplicateDeploymentConfigButton = ({ deployment }: { deployment: DeploymentView }) => {
  return (
    <DuplicateWorkload
      kind={ResourceCopyKind.Deployment}
      sourceId={deployment.id}
      sourceName={deployment.name}
      copyEligibility={deployment.copyEligibility}
      configurationHref={`/deployments/add?duplicateFrom=${deployment.id}`}
      disabled={!hasCapability(deployment, 'canRead') || !hasCapability(deployment, 'canWrite')}
    />
  );
};

const DeploymentSubHeader = ({
  deployment,
  latestActivity,
}: {
  deployment: DeploymentView;
  latestActivity: LatestActivityView | null;
}) => {
  return (
    <>
      <DeploymentHealthNotice deployment={deployment} latestActivity={latestActivity} />
      <DeploymentLatestActivity latestActivity={latestActivity} deployment={deployment} />
      <DeploymentUpdateNotice deployment={deployment} />
    </>
  );
};

const DeploymentHealthNotice = ({
  deployment,
  latestActivity,
}: {
  deployment: DeploymentView;
  latestActivity: LatestActivityView | null;
}) => {
  if (deployment.status !== DeploymentStatus.Degraded) return null;

  if (deployment.platformStatus === PlatformStatus.Offline) {
    return (
      <AlertMessage type="error" title="Platform unavailable">
        The platform is offline. Citadel cannot confirm the container state for this deployment.
      </AlertMessage>
    );
  }

  if (!deployment.dockerContainerId) {
    return (
      <AlertMessage type="error" title="Container missing">
        No container is currently linked to this deployment. Its previous container may have been removed or replaced
        outside Citadel. Redeploy this configuration, or adopt the replacement container as a deployment.
      </AlertMessage>
    );
  }

  return (
    <AlertMessage type="error" title="Deployment degraded">
      {latestActivity?.info.$type === 'DeploymentDegraded'
        ? latestActivity.info.reason
        : 'The associated container is unavailable. Check the platform and container state.'}
    </AlertMessage>
  );
};

const DeploymentLatestActivity = ({
  latestActivity,
  deployment,
}: {
  latestActivity: LatestActivityView | null;
  deployment: DeploymentView;
}) => {
  if (!latestActivity) return;
  if (latestActivity?.status === ActivityStatus.Success) {
    return;
  }
  if (latestActivity.info.$type === 'DeploymentDegraded') return null;
  if (
    deployment.status === DeploymentStatus.Healthy &&
    latestActivity.status === ActivityStatus.Failure &&
    latestActivity.info.$type === 'DeploymentApplied'
  ) {
    return (
      <AlertMessage title="Previous deployment attempt failed" date={latestActivity.createdAt} type="warning">
        The container is currently healthy. The last deployment attempt failed; see Activities for details.
      </AlertMessage>
    );
  }
  return (
    <ActivityAlertZone
      info={latestActivity?.info as any}
      activity={latestActivity as any}
      title={latestActivity.status === ActivityStatus.Failure ? 'Last operation failed' : 'Last operation warning'}
      date={latestActivity?.createdAt}
    />
  );
};

const DeploymentUpdateNotice = ({ deployment }: { deployment: DeploymentView }) => {
  if (
    deployment.spec?.updateBehavior === UpdateBehavior.Disabled ||
    deployment.autoUpdateState?.status !== AutoUpdateStatus.UpdateAvailable
  ) {
    return null;
  }

  return (
    <UpdateAvailableNotice
      actionLabel="Redeploy"
      targetLabel="deployment"
      currentLabel={formatId(deployment.autoUpdateState.currentDigest ?? undefined)}
      nextLabel={formatId(deployment.autoUpdateState.remoteDigest ?? undefined)}
      currentTitle={deployment.autoUpdateState.currentDigest ?? deployment.name}
      nextTitle={deployment.autoUpdateState.remoteDigest ?? deployment.name}
    />
  );
};

const DeploymentRuntime = ({ deployment }: { deployment: DeploymentView }) => {
  const online = deployment.platformStatus === PlatformStatus.Online;
  const canRead = hasCapability(deployment, 'canRead');
  const { data, dataUpdatedAt, isLoading, isFetching, error } = useRead(
    'getDeploymentContainerInfo',
    { id: deployment.id },
    { enabled: canRead && online && !!deployment.dockerContainerId },
  );
  const summary =
    canRead &&
    canShowCachedResource(error) &&
    data &&
    data.data.containerId === deployment.dockerContainerId &&
    data.data.platformId === deployment.platformId
      ? data.data
      : undefined;
  // Host access is optional; workload diagnostics use Deployment authorization.
  // Use the persisted identity for optional live metadata, never a global short-ID lookup.
  const canReadHost = online && summary?.capabilities?.canRead === true;
  const { containerInfo: live, error: liveError } = useContainerInfoGroup(
    canReadHost ? (deployment.containerId ?? undefined) : undefined,
    canReadHost ? deployment.platformId : undefined,
  );
  const containerInfo =
    live?.id === deployment.dockerContainerId && live?.platformId === deployment.platformId ? live : undefined;
  const terminalDisabled =
    !online ||
    !!error ||
    (canReadHost && !!liveError) ||
    isFetching ||
    !summary ||
    !!containerInfo?.projectionStaleSince ||
    (containerInfo?.state ?? summary?.state) !== ContainerStateStatus.Running ||
    deployment.controlState === ResourceControlState.Processing ||
    containerInfo?.controlState === ResourceControlState.Processing;

  if (!canRead) return <AlertMessage type="warning">You do not have permission to view this Deployment.</AlertMessage>;
  if (!deployment.dockerContainerId) {
    return (
      <AlertMessage
        type="info"
        title={deployment.status === DeploymentStatus.Created ? 'Not deployed' : 'Container unavailable'}>
        {deployment.status === DeploymentStatus.Created
          ? 'Apply this Deployment to create its container. Configuration and Activities are still available.'
          : 'No container is currently linked. Review Activities for previous operations. Logs and inspection of a deleted container may no longer be available.'}
      </AlertMessage>
    );
  }
  if (error && [401, 403].includes(requestErrorStatus(error) ?? 0)) {
    return (
      <AlertMessage type="warning">
        {requestErrorMessage(error)} Review Activities for previous operations.
      </AlertMessage>
    );
  }

  return (
    <div className="flex w-full flex-col gap-4">
      {(!online || error) && (
        <AlertMessage
          type="warning"
          title={
            !online || requestErrorStatus(error) === 404
              ? 'Live diagnostics unavailable'
              : 'Container details unavailable'
          }>
          {!online ? 'The Platform is offline.' : requestErrorMessage(error)} Container: {deployment.dockerContainerId}.
          Retained statistics and Activities remain available. Any displayed container metadata is last known.
          {summary && dataUpdatedAt > 0 && (
            <span className="block mt-1">
              Details last loaded:{' '}
              <time dateTime={new Date(dataUpdatedAt).toISOString()}>{new Date(dataUpdatedAt).toLocaleString()}</time>.
            </span>
          )}
        </AlertMessage>
      )}
      {isLoading && <Loader />}
      <RuntimeView
        containerInfo={containerInfo}
        summary={summary}
        containerId={deployment.dockerContainerId}
        deploymentId={deployment.id}
        deployment={deployment}
        liveAvailable={online && requestErrorStatus(error) !== 404}
        disabled={terminalDisabled}
      />
    </div>
  );
};

const RuntimeView = ({
  containerInfo,
  containerId,
  deploymentId,
  deployment,
  disabled,
  summary,
  liveAvailable,
}: {
  summary?: ContainerSummaryView;
  liveAvailable: boolean;
  containerInfo?: ContainerRuntimeView | undefined;
  containerId?: string | undefined;
  deploymentId: string;
  deployment: DeploymentView;
  disabled?: boolean;
}) => {
  return (
    <div className="flex flex-col gap-4 w-full">
      {summary && (
        <DeploymentContainerInfoTable
          summary={summary}
          deploymentId={deploymentId}
          health={
            <WorkloadContainerHealthCard
              resource={deployment}
              container={{
                id: summary.containerId,
                platformId: summary.platformId,
                name: summary.name,
                dockerNodeId: containerInfo?.dockerNodeId ?? null,
              }}
            />
          }
          container={containerInfo}
          displayOptions={{
            DisplayContainerName: true,
            DisplayStatus: false,
          }}
        />
      )}
      <RuntimeTabs
        containerInfo={containerInfo}
        containerId={containerId}
        deploymentId={deploymentId}
        deployment={deployment}
        liveAvailable={liveAvailable}
        disabled={disabled}
      />
    </div>
  );
};

const RuntimeTabs = ({
  containerInfo,
  containerId,
  deploymentId,
  deployment,
  disabled,
  liveAvailable,
}: {
  liveAvailable: boolean;
  containerInfo?: ContainerRuntimeView | undefined;
  containerId?: string | undefined;
  deploymentId: string;
  deployment: DeploymentView;
  disabled?: boolean;
}) => {
  const nid = normalizeDockerId(containerInfo?.id ?? containerId);
  const canViewLogs = liveAvailable && hasCapability(deployment, 'canViewLogs');
  const canInspect = liveAvailable && hasCapability(deployment, 'canInspect');
  const terminalDisabled = disabled || !hasCapability(deployment, 'canOpenTerminal');
  const defaultValue = canViewLogs ? 'logs' : canInspect ? 'inspect' : terminalDisabled ? 'stats' : 'terminal';
  const [selected, setSelected] = useState(defaultValue);
  const allowed = { logs: canViewLogs, inspect: canInspect, terminal: !terminalDisabled, stats: true };
  const activeTab = allowed[selected as keyof typeof allowed] ? selected : defaultValue;

  return (
    <Tabs value={activeTab} onValueChange={setSelected} className="w-full">
      <div className="flex min-w-0 items-center gap-2 border-b">
        <TabsList className="min-w-0 flex-1 overflow-x-auto border-0">
          <TabsTrigger value="logs" disabled={!canViewLogs}>
            Logs
          </TabsTrigger>
          <TabsTrigger value="inspect" disabled={!canInspect}>
            Inspect
          </TabsTrigger>
          <TabsTrigger value="terminal" disabled={terminalDisabled}>
            Terminal
          </TabsTrigger>
          <TabsTrigger value="stats">Stats</TabsTrigger>
        </TabsList>
        <TabActionsSlot />
      </div>
      {canViewLogs && (
        <TabsContent value="logs" className="mt-2 w-full">
          <DeploymentLogs key={`${deploymentId}:${nid}`} containerId={nid} deploymentId={deploymentId} />
        </TabsContent>
      )}
      {canInspect && (
        <TabsContent value="inspect" className="mt-2 w-full">
          <DeploymentInspect key={deploymentId} deploymentId={deploymentId} resourceId={deployment.containerId} />
        </TabsContent>
      )}
      {!terminalDisabled && (
        <TabsContent value="terminal" className="mt-2 w-full">
          <DeploymentExec
            key={`${deploymentId}:${nid}`}
            containerId={nid}
            deploymentId={deploymentId}
            disabled={disabled}
          />
        </TabsContent>
      )}
      <TabsContent value="stats" className="mt-2 w-full">
        <DeploymentStats key={deploymentId} resource={containerInfo} deploymentId={deploymentId} />
      </TabsContent>
    </Tabs>
  );
};
