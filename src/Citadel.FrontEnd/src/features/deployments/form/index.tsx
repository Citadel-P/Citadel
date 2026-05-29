import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { DeploymentForm } from './form';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { DeploymentActions } from './actions';
import { useState } from 'react';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useDeploymentGroup } from './hooks/useDeploymentGroup';
import {
  ActivityStatus,
  AutoUpdateStatus,
  DeploymentStatus,
  DeploymentView,
  LatestActivityView,
  ResourceControlState,
  UpdateBehavior,
} from '@/api/generated/api.types';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { DockerContainerView } from '@/api/types';
import { DeploymentInspect } from '@/features/docker-resources/containers/container-info/container-inspect';
import { formatId, normalizeDockerId } from '@/lib/utils';
import { DeploymentContainerInfoTable } from '@/features/docker-resources/containers/container-info/container-info-table';
import { useContainerInfoGroup } from '@/features/docker-resources/containers/hooks/useContainerInfoGroup';
import { DeploymentExec } from '@/features/docker-resources/containers/container-info/container-exec';
import { DeploymentStats } from '@/features/docker-resources/containers/container-info/container-stats';
import { ActivitiesTab } from '@/features/activities';
import { ArrowRight, ArrowUpCircle, X } from 'lucide-react';
import { AlertMessage } from '@/components/custom/alert-message';
import { ActivityAlertZone } from '@/components/custom/task-sheet';
import { DeploymentLogs } from '@/features/docker-resources/containers/container-info/container-logs';
import { hasCapability } from '@/lib/resource-capabilities';

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
          <StateIndicator
            value={(resource as DeploymentView).status}
            isProcessing={(resource as DeploymentView).controlState === ResourceControlState.Processing}
          />
        );
      },
      ActionButtons: ({ resource }) => {
        return <GenericActionBarButtons resource={resource} actions={Object.values(DeploymentActions)} />;
      },
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
        label: 'Activities',
        Content: ({ resource }: { resource: DeploymentView }) => {
          return <ActivitiesTab resourceId={resource.id} resourceType="Deployment" />;
        },
      },
    ],
    useData: function (id: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { deployment, isLoading } = useDeploymentGroup(id);
      return { item: deployment, isLoading };
    },
  },
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
      <DeploymentLatestActivity latestActivity={latestActivity} />
      <DeploymentUpdateNotice deployment={deployment} />
    </>
  );
};

const DeploymentLatestActivity = ({ latestActivity }: { latestActivity: LatestActivityView | null }) => {
  if (!latestActivity) return;
  if (latestActivity?.status === ActivityStatus.Success) {
    return;
  }
  if (latestActivity?.info.$type === 'DeploymentDegraded') {
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

const DeploymentUpdateNotice = ({ deployment }: { deployment: DeploymentView }) => {
  const [dismissed, setDismissed] = useState(false);
  if (
    dismissed ||
    deployment.spec?.updateBehavior === UpdateBehavior.Disabled ||
    deployment.autoUpdateState.status !== AutoUpdateStatus.UpdateAvailable
  ) {
    return null;
  }

  return (
    <div className="flex items-center justify-between gap-4 rounded-md border border-amber-200 bg-amber-50/50 px-4 py-3 ">
      <div className="flex items-center gap-3 overflow-hidden">
        <ArrowUpCircle className="h-4 w-4 shrink-0 text-amber-500" />
        <div className="flex items-center gap-2 truncate text-sm text-muted-foreground">
          <span className="font-mono text-foreground/80 ">Update available: </span>
          <span className="truncate ">
            Click <span className="font-mono text-foreground/80">Redeploy</span> to apply the update to this deployment.
          </span>
          <div className="flex items-center gap-1.5 shrink-0">
            <span
              className=" text-xs text-muted-foreground line-through"
              title={deployment.autoUpdateState.currentDigest ?? deployment.name}>
              {formatId(deployment.autoUpdateState.currentDigest ?? undefined)}
            </span>
            <ArrowRight className="h-3 w-3 text-muted-foreground" />
            <span
              className="text-xs font-medium text-amber-700"
              title={deployment.autoUpdateState.remoteDigest ?? deployment.name}>
              {formatId(deployment.autoUpdateState.remoteDigest ?? undefined)}
            </span>
          </div>
        </div>
      </div>

      <button
        onClick={() => setDismissed(true)}
        className="shrink-0 rounded-md p-1 text-slate-400 transition-colors hover:bg-amber-100/50 hover:text-slate-600"
        aria-label="Dismiss">
        <X className="h-4 w-4" />
      </button>
    </div>
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
  containerInfo?: DockerContainerView | undefined;
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
  containerInfo?: DockerContainerView | undefined;
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
      <TabsList className="w-fit justify-start">
        <TabsTrigger className="text-xs" value="logs" disabled={!canViewLogs}>
          Logs
        </TabsTrigger>
        <TabsTrigger className="text-xs" value="inspect" disabled={!canInspect}>
          Inspect
        </TabsTrigger>
        <TabsTrigger className="text-xs" value="terminal" disabled={terminalDisabled}>
          Terminal
        </TabsTrigger>
        <TabsTrigger className="text-xs" value="stats">
          Stats
        </TabsTrigger>
      </TabsList>
      {canViewLogs && (
        <TabsContent value="logs" className="w-full mt-2">
          <DeploymentLogs key={deploymentId} containerId={nid} deploymentId={deploymentId} />
        </TabsContent>
      )}
      {canInspect && (
        <TabsContent value="inspect" className="w-full mt-2">
          <DeploymentInspect key={deploymentId} deploymentId={deploymentId} />
        </TabsContent>
      )}
      {!terminalDisabled && (
        <TabsContent value="terminal" className="w-full mt-2">
          <DeploymentExec key={deploymentId} containerId={nid} deploymentId={deploymentId} disabled={disabled} />
        </TabsContent>
      )}
      <TabsContent value="stats" className="w-full mt-2">
        <DeploymentStats key={deploymentId} resource={containerInfo} deploymentId={deploymentId} />
      </TabsContent>
    </Tabs>
  );
};
