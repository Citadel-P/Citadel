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
  ContainerDataView,
  ResourceBindingScope,
  LatestActivityView,
  PlatformStatus,
  ResourceControlState,
  UpdateBehavior,
} from '@/api/generated/api.types';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
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
import { ResourceHeaderTagsEditor } from '@/features/tags/components';
import { Button } from '@/components/ui/button';
import { Copy } from 'lucide-react';
import { useNavigate } from 'react-router';

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
        disabled: (resource: DeploymentView): boolean =>
          resource.status === DeploymentStatus.Degraded || resource.status === DeploymentStatus.Created,
        Content: ({ resource }: { resource: DeploymentView }) => {
          return <DeploymentRuntime key={resource.id} deployment={resource} />;
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
  const navigate = useNavigate();
  const disabled = !hasCapability(deployment, 'canRead') || !hasCapability(deployment, 'canWrite');

  return (
    <Button
      type="button"
      variant="outline"
      size="sm"
      className="h-8 rounded-sm text-xs"
      disabled={disabled}
      onClick={() => navigate(`/deployments/add?duplicateFrom=${deployment.id}`)}>
      <Copy className="size-3.5" />
      Duplicate Config
    </Button>
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
  const { containerInfo, isLoading, error } = useContainerInfoGroup(
    deployment.dockerContainerId ?? undefined,
    deployment.platformId,
  );

  if (deployment.status === DeploymentStatus.Degraded) return null;
  const disabled = deployment.status !== DeploymentStatus.Healthy;

  if (!deployment.dockerContainerId)
    return <div className="text-sm text-muted-foreground mb-2">No container assigned</div>;

  if (error)
    return (
      <div className="mb-2">
        <AlertMessage type="warning">
          <div className="truncate">{(error as any)?.error?.detail ?? 'Platform unavailable'}</div>
        </AlertMessage>
      </div>
    );

  return (
    <RuntimeView
      containerInfo={containerInfo}
      containerId={deployment.dockerContainerId}
      deploymentId={deployment.id}
      deployment={deployment}
      containerLoading={isLoading}
      disabled={disabled}
    />
  );
};

const RuntimeView = ({
  containerInfo,
  containerId,
  deploymentId,
  deployment,
  disabled,
}: {
  containerInfo?: ContainerDataView | undefined;
  containerId?: string | undefined;
  deploymentId: string;
  deployment: DeploymentView;
  containerLoading?: boolean;
  disabled?: boolean;
}) => {
  return (
    <div className="flex flex-col gap-4 w-full">
      <DeploymentContainerInfoTable
        deploymentId={deploymentId}
        container={containerInfo}
        displayOptions={{
          DisplayContainerName: true,
          DisplayStatus: false,
        }}
      />
      <RuntimeTabs
        containerInfo={containerInfo}
        containerId={containerId}
        deploymentId={deploymentId}
        deployment={deployment}
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
}: {
  containerInfo?: ContainerDataView | undefined;
  containerId?: string | undefined;
  deploymentId: string;
  deployment: DeploymentView;
  disabled?: boolean;
}) => {
  const nid = normalizeDockerId(containerInfo?.id ?? containerId);
  const canViewLogs = hasCapability(deployment, 'canViewLogs');
  const canInspect = hasCapability(deployment, 'canInspect');
  const terminalDisabled = disabled || !hasCapability(deployment, 'canOpenTerminal');
  const defaultValue = canViewLogs ? 'logs' : canInspect ? 'inspect' : terminalDisabled ? 'stats' : 'terminal';

  return (
    <Tabs defaultValue={defaultValue} className="w-full">
      <TabsList className="w-fit max-w-full overflow-x-auto">
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
      {canViewLogs && (
        <TabsContent value="logs" className="mt-2 w-full">
          <DeploymentLogs key={deploymentId} containerId={nid} deploymentId={deploymentId} />
        </TabsContent>
      )}
      {canInspect && (
        <TabsContent value="inspect" className="mt-2 w-full">
          <DeploymentInspect key={deploymentId} deploymentId={deploymentId} />
        </TabsContent>
      )}
      {!terminalDisabled && (
        <TabsContent value="terminal" className="mt-2 w-full">
          <DeploymentExec key={deploymentId} containerId={nid} deploymentId={deploymentId} disabled={disabled} />
        </TabsContent>
      )}
      <TabsContent value="stats" className="mt-2 w-full">
        <DeploymentStats key={deploymentId} resource={containerInfo} deploymentId={deploymentId} />
      </TabsContent>
    </Tabs>
  );
};
