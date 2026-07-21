import { memo, ReactNode, useCallback, useEffect, useMemo } from 'react';
import {
  Calendar,
  Check,
  CheckCheck,
  Clock,
  GitBranch,
  GitCommitHorizontal,
  LoaderCircle,
  NotepadText,
  Route,
} from 'lucide-react';
import { ResourceType } from '@/api/types';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { useResourceFilter, useTaskSheet } from '@/lib/atoms';
import { useAppContext } from '@/lib/context/app-context';
import {
  ActorCell,
  AlertEventStatusCell,
  LogViewer,
  TargetCell,
  UpdateAvailableNotice,
} from '@/components/custom/common';
import { useMutate, useRead, useStreamProgress } from '@/lib/hooks';
import {
  ActivityView,
  AlertEventStatus,
  AlertEventView,
  ApplyDeploymentInput,
  AcknowledgeAlertEventsInput,
  DeploymentStreamItem,
  ActivityEventInfo,
  PullImageInput,
  PullImageStreamItem,
  ResolveAlertEventsInput,
  RegistryView,
  ActivityStatus,
  ActivityEventInfoGitRepoPulled,
  ActivityEventInfoGitRepoCloned,
  ActivityEventInfoDeploymentApplied,
  ActivityEventInfoStackApplied,
  ActivityEventInfoStackRollback,
  ActivityEventInfoGitRepoWebhookReceived,
  ActivityEventInfoStackWebhookReceived,
  ActivityEventInfoBuildWebhookReceived,
  BuildProjectSnapshot,
  ApplyStackInput,
  AutomationActionRunStreamItem,
  BackupRunItemStatus,
  BackupRunItemView,
  BackupRunStatus,
  BackupRunStreamItem,
  BackupRestoreStatus,
  RollbackStackInput,
  RunAutomationActionInput,
  QueueBackupRunInput,
  RestoreVolumeInput,
  TestAutomationActionInput,
  StackReleaseStatus,
  StackReleaseSource,
  StackSnapshot,
  StackStreamItem,
} from '@/api/generated/api.types';
import { formatActivityEvent, serializeData } from '@/lib/utils';
import Loader from '../ui/loader';
import { MonacoDiff, MonacoEditor } from '@/lib/monaco';
import { Button } from '@/components/ui/button';
import { AlertMessage } from './alert-message';
import { CitadelIcons } from '@/lib/icons';
import { useQueryClient } from '@tanstack/react-query';
import { StateIndicator } from './state-indicator';
import { byteTransform } from '@/lib/bytes.helper';

interface PullImageParams {
  imageTag: string;
  registryId: string;
}

type DeployParams = { name: string } & ApplyDeploymentInput;
type StackDeployParams = { name: string } & ApplyStackInput;
type StackRollbackParams = { name: string; version?: string } & RollbackStackInput;
type BackupRunParams = { id: string; name: string } & QueueBackupRunInput;
type BackupRunLogsParams = {
  id: string;
  name: string;
  trigger: string;
  items: BackupRunItemView[];
};
type BackupRestoreRunParams = { id: string; name: string } & RestoreVolumeInput;
type BackupRestoreRunLogsParams = {
  id: string;
  name: string;
  status: string;
};
type AutomationActionRunParams = {
  id: string;
  name: string;
} & (({ mode: 'run' } & RunAutomationActionInput) | ({ mode: 'test' } & TestAutomationActionInput));

type BackupRestoreRunStreamItem = {
  restoreRunId: string;
  status?: BackupRestoreStatus | null;
  message?: string | null;
  stream?: string | null;
  exitCode?: number | null;
};

export type TaskSpec =
  | { kind: 'pull'; payload: PullImageParams }
  | { kind: 'deploy'; payload: DeployParams }
  | { kind: 'activity'; payload: ActivityView }
  | { kind: 'alertEvent'; payload: AlertEventView }
  | { kind: 'build'; payload: Record<string, unknown> }
  | { kind: 'stack'; payload: Record<string, unknown> }
  | { kind: 'stackRollback'; payload: StackRollbackParams }
  | { kind: 'backupRun'; payload: BackupRunParams }
  | { kind: 'backupRunLogs'; payload: BackupRunLogsParams }
  | { kind: 'backupRestoreRun'; payload: BackupRestoreRunParams }
  | { kind: 'backupRestoreRunLogs'; payload: BackupRestoreRunLogsParams }
  | { kind: 'automationActionRun'; payload: AutomationActionRunParams };

export interface TaskSheetState {
  open: boolean;
  task?: TaskSpec;
}

interface TaskStreamLayoutProps {
  title: string;
  refName: string;
  type: ResourceType;
  state: ReturnType<typeof useStreamProgress>;
}

function TaskStreamLayout({ title, refName, type, state }: TaskStreamLayoutProps) {
  const { status, elapsedLabel } = state;
  const Icon = CitadelIcons[type] ?? CitadelIcons.Platform;
  const iconClassName = getTaskStreamIconClassName(state);

  return (
    <>
      <SheetHeader>
        <SheetTitle>{title}</SheetTitle>
        <SheetDescription>
          <span className="inline-flex flex-wrap items-center gap-x-4 gap-y-1">
            <span className="inline-flex flex-row gap-2 items-center">
              {status === 'pending' && <LoaderCircle className="h-3.5 w-3.5 animate-spin" />}
              <span className={iconClassName}>
                <Icon className="w-3.5 h-3.5" />
              </span>
              <span>{refName}</span>
            </span>

            <span className="inline-flex flex-row gap-2 items-center">
              <Clock className="w-3.5 h-3.5" />
              <span>{elapsedLabel} seconds</span>
            </span>
          </span>
        </SheetDescription>
      </SheetHeader>
      <div className="pt-0 pb-4 px-4">
        <LogViewer logs={state.logs} autoScroll={true} />
      </div>
    </>
  );
}

function getTaskStreamIconClassName(state: ReturnType<typeof useStreamProgress>) {
  if (state.status === 'error') return 'text-destructive';
  if (state.status === 'warning') return 'text-amber-500';
  if (state.status === 'success') return 'text-success';
  return '';
}

function TaskActivityLayout({ activityId }: { activityId: string }) {
  const { data, isLoading } = useRead('getActivity', { id: activityId });

  if (isLoading) return <Loader />;

  const activity = data!.data;
  const releaseSource = getActivityReleaseSource(activity);

  return (
    <div className="p-2">
      <SheetHeader>
        <SheetTitle>{formatActivityEvent(activity.eventType)}</SheetTitle>
        <SheetDescription asChild>
          <div className={releaseSource ? 'flex flex-between items-start gap-x-8 gap-y-4' : 'flex flex-col gap-3'}>
            <div className="flex flex-col gap-3">
              <ActorCell type={activity.actorType} name={activity.actorName} />
              <TargetCell
                resourceType={activity.resourceType}
                resourceId={activity.resourceId ?? ''}
                resourceName={activity.resourceName}
              />
              <RecordedAtCell createdAt={activity.createdAt} />
            </div>
            {releaseSource ? <ReleaseSourceHeader source={releaseSource} /> : null}
          </div>
        </SheetDescription>
      </SheetHeader>

      <div className="p-4 pt-0 pb-2">
        <div className="rounded-lg border shadow-xs p-4">
          <ActivityInfo activity={activity} />
        </div>
      </div>
    </div>
  );
}

function TaskAlertEventLayout({ alertEventId }: { alertEventId: string }) {
  const { data, isLoading } = useRead('getAlertEvent', { id: alertEventId });
  const { liveAlertEvents } = useAppContext();
  const { mutateAsync: acknowledgeAlertEvents, isPending: isAcknowledging } = useMutate('acknowledgeAlertEvents');
  const { mutateAsync: resolveAlertEvents, isPending: isResolving } = useMutate('resolveAlertEvents');

  if (isLoading) return <Loader />;

  const event = data?.data ? (liveAlertEvents[data.data.id] ?? data.data) : undefined;
  if (!event) {
    return <div className="p-8 text-center text-muted-foreground">Alert event not found.</div>;
  }

  const canAcknowledge = event.status === AlertEventStatus.Active;
  const canResolve = event.status !== AlertEventStatus.Resolved;

  const handleAcknowledge = async () => {
    const input: AcknowledgeAlertEventsInput = { ids: [event.id] };
    await acknowledgeAlertEvents({ data: input });
  };

  const handleResolve = async () => {
    const input: ResolveAlertEventsInput = { ids: [event.id], resolutionNote: null };
    await resolveAlertEvents({ data: input });
  };

  return (
    <div className="p-2">
      <SheetHeader>
        <SheetTitle>{event.type}</SheetTitle>
        <SheetDescription asChild>
          <div className="flex flex-col gap-3">
            <TargetCell
              resourceType={event.resourceType}
              resourceId={event.resourceId ?? ''}
              resourceName={event.resourceName}
            />
            <div className="flex flex-row gap-4">
              <AlertEventActions
                status={event.status}
                canAcknowledge={canAcknowledge}
                canResolve={canResolve}
                isAcknowledging={isAcknowledging}
                isResolving={isResolving}
                onAcknowledge={handleAcknowledge}
                onResolve={handleResolve}
              />

              {event.actorType && event.actorName && <ActorCell type={event.actorType} name={event.actorName} />}
            </div>

            <RecordedAtCell createdAt={event.createdAt} />
          </div>
        </SheetDescription>
      </SheetHeader>

      <div className="p-4 pt-0 pb-2">
        <MonacoEditor
          value={serializeData(data!.data.info)}
          filename={`alert-event-info-${event.id}.json`}
          className="my-0 mx-0 min-h-55"
          title="Info"
          readOnly
          folding
        />
      </div>
    </div>
  );
}

function SpecViewer({ spec, title, resourceId }: { spec: unknown; title: string; resourceId: string | null }) {
  return (
    <MonacoEditor
      value={serializeData(spec)}
      filename={`inspect-${resourceId}.yaml`}
      className="my-0 mx-0 min-h-55"
      title={title}
      readOnly
      folding
    />
  );
}

type InfoOf<T extends ActivityEventInfo['$type']> = Extract<ActivityEventInfo, { $type: T }>;

type ActivityInfoRendererMap = {
  [K in ActivityEventInfo['$type']]: (info: InfoOf<K>, activity: ActivityView) => React.ReactNode;
};

const activityInfoRenderers: ActivityInfoRendererMap = {
  DeploymentUpdated: (info) => (
    <MonacoDiff
      original={info.oldDeployment}
      modified={info.newDeployment}
      format="yaml"
      title="Configuration changes"
    />
  ),

  DeploymentCreated: (info, activity) => (
    <SpecViewer spec={info.deployment} resourceId={activity.resourceId} title="Initial configuration" />
  ),

  DeploymentDuplicated: (info, activity) => (
    <div className="flex flex-col gap-4 text-sm text-muted-foreground">
      <DuplicateSource source={info.source} />
      <SpecViewer spec={info.deployment} resourceId={activity.resourceId} title="Duplicated configuration" />
    </div>
  ),

  DeploymentDeleted: (info, activity) => (
    <SpecViewer spec={info.deployment} resourceId={activity.resourceId} title="Deleted configuration" />
  ),

  DeploymentApplied: (info, activity) => (
    <div className="flex flex-col gap-4 text-sm text-muted-foreground">
      <SpecViewer spec={info.deployment} resourceId={activity.resourceId} title="Applied configuration" />
      {activity.status === ActivityStatus.Success && (
        <KeyValueBlock label="Container ID" value={info.result.containerIds ?? []} />
      )}
      <ActivityAlertZone info={info} activity={activity} />
    </div>
  ),

  DeploymentDegraded: (info) => <span className="text-sm text-muted-foreground">{info.reason}</span>,

  DeploymentRenamed: (info) => (
    <span className="text-sm text-muted-foreground">
      Deployment renamed from <b>{info.oldName}</b> to <b>{info.newName}</b>.
    </span>
  ),

  DeploymentStarted: (info) => <KeyValueBlock label="Container id" value={info.containerIds} />,
  DeploymentStopped: (info) => <KeyValueBlock label="Container id" value={info.containerIds} />,
  DeploymentPaused: (info) => <KeyValueBlock label="Container id" value={info.containerIds} />,

  StackCreated: (info, activity) => (
    <SpecViewer
      spec={stripStackReleaseSource(info.stack)}
      resourceId={activity.resourceId}
      title="Initial configuration"
    />
  ),

  StackDuplicated: (info, activity) => (
    <div className="flex flex-col gap-4 text-sm text-muted-foreground">
      <DuplicateSource source={info.source} />
      <SpecViewer
        spec={stripStackReleaseSource(info.stack)}
        resourceId={activity.resourceId}
        title="Duplicated configuration"
      />
    </div>
  ),

  StackUpdated: (info) => (
    <MonacoDiff
      original={stripStackReleaseSource(info.oldStack)}
      modified={stripStackReleaseSource(info.newStack)}
      format="yaml"
      title="Configuration changes"
    />
  ),

  StackApplied: (info, activity) => {
    const containerIds = info.result.containerIds ?? [];
    const label = containerIds.length <= 1 ? 'Container ID' : 'Container IDs';
    return (
      <div className="flex flex-col gap-4 text-sm text-muted-foreground">
        <SpecViewer
          spec={stripStackReleaseSource(info.stack)}
          resourceId={activity.resourceId}
          title="Applied configuration"
        />
        {activity.status === ActivityStatus.Success && (
          <KeyValueBlock label={label} value={info.result.containerIds ?? []} />
        )}
        <ActivityAlertZone info={info} activity={activity} />
      </div>
    );
  },
  StackRollback: (info, activity) => {
    const containerIds = info.result.containerIds ?? [];
    const label = containerIds.length <= 1 ? 'Container ID' : 'Container IDs';
    return (
      <div className="flex flex-col gap-4 text-sm text-muted-foreground">
        <MonacoDiff
          original={stripStackReleaseSource(info.oldStack)}
          modified={stripStackReleaseSource(info.newStack)}
          format="yaml"
          title="Rollback configuration changes"
        />
        {activity.status === ActivityStatus.Success && (
          <KeyValueBlock label={label} value={info.result.containerIds ?? []} />
        )}
        <ActivityAlertZone info={info} activity={activity} />
      </div>
    );
  },
  StackDegraded: (info) => <span className="text-sm text-muted-foreground">{info.reason}</span>,
  StackDriftDetected: (info) => <span className="text-sm text-muted-foreground">{info.reason}</span>,
  StackDriftResolved: (info) => <KeyValueBlock label="Resolved drift fingerprint" value={info.previousFingerprint} />,
  StackReconciliationAttempted: (info) => {
    const actions = info.actions.map((action) => {
      const result = action.succeeded ? 'succeeded' : `failed${action.errorMessage ? `: ${action.errorMessage}` : ''}`;
      return `${action.action} ${action.serviceName} (${action.containerId}) ${result}`;
    });

    return (
      <div className="flex flex-col gap-4 text-sm text-muted-foreground">
        <span>Reconciliation status: {info.status}</span>
        <KeyValueBlock label="Drift fingerprint" value={info.driftFingerprint} />
        {actions.length > 0 && <KeyValueBlock label="Actions" value={actions} />}
      </div>
    );
  },

  StackStarted: (info) => <KeyValueBlock label="Container IDs" value={info.containerIds} />,
  StackStopped: (info) => <KeyValueBlock label="Container IDs" value={info.containerIds} />,
  StackPaused: (info) => <KeyValueBlock label="Container IDs" value={info.containerIds} />,
  StackGitUpdateAvailable: (info) => (
    <UpdateAvailableNotice
      title="Git update available:"
      actionLabel="Deploy"
      targetLabel="stack"
      sourceLabel={`${info.gitRepositoryName}${info.branch ? `/${info.branch}` : ''}`}
      sourceTitle={info.gitRepositoryName}
      currentLabel={shortCommit(info.currentCommitSha)}
      nextLabel={shortCommit(info.remoteCommitSha)}
      currentTitle={info.currentCommitSha}
      nextTitle={info.remoteCommitSha}
      dismissible={false}
    />
  ),
  StackGitAutoUpdated: (info) => (
    <div className="flex flex-col gap-4 text-sm text-muted-foreground">
      <span>
        Stack auto-updated from <b>{shortCommit(info.previousCommitSha)}</b> to{' '}
        <b>{shortCommit(info.updatedCommitSha)}</b>.
      </span>
      <KeyValueBlock label="Repository" value={info.gitRepositoryName} />
      <KeyValueBlock label="Branch" value={info.branch} />
      <KeyValueBlock label="Previous commit" value={info.previousCommitSha} />
      <KeyValueBlock label="Updated commit" value={info.updatedCommitSha} />
    </div>
  ),
  StackGitAutoDeployFailed: (info) => (
    <div className="flex flex-col gap-4 text-sm text-muted-foreground">
      <AlertMessage type="error" title="Auto deploy failed">
        {info.reason}
      </AlertMessage>
      <KeyValueBlock label="Repository" value={info.gitRepositoryName} />
      <KeyValueBlock label="Branch" value={info.branch} />
      <KeyValueBlock label="Current commit" value={info.currentCommitSha} />
      <KeyValueBlock label="Remote commit" value={info.remoteCommitSha} />
    </div>
  ),
  StackWebhookReceived: (info) => <WebhookActivityDetails info={info} />,

  StackRenamed: (info) => (
    <span className="text-sm text-muted-foreground">
      Deployment renamed from <b>{info.oldName}</b> to <b>{info.newName}</b>.
    </span>
  ),
  StackDeleted: (info, activity) => (
    <SpecViewer spec={info.stack} resourceId={activity.resourceId} title="Deleted configuration" />
  ),

  BuildCreated: (info, activity) => (
    <SpecViewer
      spec={stripBuildProjectSecrets(info.build)}
      resourceId={activity.resourceId}
      title="Initial configuration"
    />
  ),

  BuildUpdated: (info) => (
    <MonacoDiff
      original={stripBuildProjectSecrets(info.oldBuild)}
      modified={stripBuildProjectSecrets(info.newBuild)}
      format="yaml"
      title="Configuration changes"
    />
  ),

  BuildRenamed: (info) => (
    <span className="text-sm text-muted-foreground">
      Build renamed from <b>{info.oldName}</b> to <b>{info.newName}</b>.
    </span>
  ),

  BuildDeleted: (info, activity) => (
    <SpecViewer
      spec={stripBuildProjectSecrets(info.build)}
      resourceId={activity.resourceId}
      title="Deleted configuration"
    />
  ),

  BuildRunQueued: (info) => <BuildRunActivityDetails info={info} status="Queued" />,

  BuildRunStarted: (info) => <BuildRunActivityDetails info={info} status="Started" />,

  BuildRunSucceeded: (info) => <BuildRunActivityDetails info={info} status="Succeeded" />,

  BuildRunFailed: (info) => <BuildRunActivityDetails info={info} status="Failed" />,

  BuildRunTimedOut: (info) => <BuildRunActivityDetails info={info} status="Timed out" />,

  BuildRunCancelled: (info) => <BuildRunActivityDetails info={info} status="Cancelled" />,

  BuildWebhookReceived: (info) => <WebhookActivityDetails info={info} />,

  BuildAgentPoolCreated: (info, activity) => (
    <SpecViewer spec={info.pool} resourceId={activity.resourceId} title="Initial configuration" />
  ),

  BuildAgentPoolUpdated: (info) => (
    <MonacoDiff original={info.oldPool} modified={info.newPool} format="yaml" title="Configuration changes" />
  ),

  BuildAgentPoolRenamed: (info) => (
    <span className="text-sm text-muted-foreground">
      Build pool renamed from <b>{info.oldName}</b> to <b>{info.newName}</b>.
    </span>
  ),

  BuildAgentPoolDeleted: (info, activity) => (
    <SpecViewer spec={info.pool} resourceId={activity.resourceId} title="Deleted configuration" />
  ),

  BuildAgentPoolTested: (info) => (
    <div className="flex flex-col gap-2 text-sm text-muted-foreground">
      <KeyValueBlock label="Status" value={info.status} />
      {info.message ? <KeyValueBlock label="Message" value={info.message} /> : null}
    </div>
  ),

  AlertRuleUpdated: (info) => (
    <MonacoDiff original={info.oldRule} modified={info.newRule} format="json" title="Configuration changes" />
  ),

  AlertRuleCreated: (info, activity) => (
    <SpecViewer spec={info.alertRule} resourceId={activity.resourceId} title="Initial configuration" />
  ),

  AlertRuleDeleted: (info, activity) => (
    <SpecViewer spec={info.alertRule} resourceId={activity.resourceId} title="Deleted configuration" />
  ),

  AlertRuleRenamed: (info) => (
    <span className="text-sm text-muted-foreground">
      Alert rule renamed from <b>{info.oldName}</b> to <b>{info.newName}</b>.
    </span>
  ),

  PlatformCreated: (info, activity) => (
    <SpecViewer spec={info.platform} resourceId={activity.resourceId} title="Initial configuration" />
  ),

  PlatformDeleted: (info, activity) => (
    <SpecViewer spec={info.platform} resourceId={activity.resourceId} title="Deleted configuration" />
  ),

  PlatformConnected: (info) => (
    <span className="text-sm text-muted-foreground">
      Platform changed from <b>{info.previousStatus}</b> to <b>{info.platform.status}</b>.
    </span>
  ),

  PlatformDisconnected: (info) => (
    <span className="text-sm text-muted-foreground">
      Platform changed from <b>{info.previousStatus}</b> to <b>{info.platform.status}</b>.
    </span>
  ),

  PlatformRenamed: (info) => (
    <span className="text-sm text-muted-foreground">
      Platform renamed from <b>{info.oldName}</b> to <b>{info.newName}</b>.
    </span>
  ),

  RegistryCreated: (info, activity) => (
    <SpecViewer spec={info.registry} resourceId={activity.resourceId} title="Initial configuration" />
  ),

  RegistryUpdated: (info) => (
    <MonacoDiff original={info.oldRegistry} modified={info.newRegistry} format="json" title="Configuration changes" />
  ),

  RegistryDeleted: (info, activity) => (
    <SpecViewer spec={info.registry} resourceId={activity.resourceId} title="Deleted configuration" />
  ),

  RegistryRenamed: (info) => (
    <span className="text-sm text-muted-foreground">
      Registry renamed from <b>{info.oldName}</b> to <b>{info.newName}</b>.
    </span>
  ),

  GitRepoCreated: (info, activity) => (
    <SpecViewer spec={info.gitRepo} resourceId={activity.resourceId} title="Initial configuration" />
  ),

  GitRepoUpdated: (info) => (
    <MonacoDiff original={info.oldGitRepo} modified={info.newGitRepo} format="json" title="Configuration changes" />
  ),

  GitRepoDeleted: (info, activity) => (
    <SpecViewer spec={info.gitRepo} resourceId={activity.resourceId} title="Deleted configuration" />
  ),

  GitRepoRenamed: (info) => (
    <span className="text-sm text-muted-foreground">
      Repository renamed from <b>{info.oldName}</b> to <b>{info.newName}</b>.
    </span>
  ),

  GitRepoCloned: (info, activity) => (
    <div className="flex flex-col gap-4 text-sm text-muted-foreground">
      <SpecViewer spec={info.gitRepo} resourceId={activity.resourceId} title="Cloned configuration" />
      <ActivityAlertZone info={info} activity={activity} />
    </div>
  ),

  GitRepoPulled: (info, activity) => (
    <div className="flex flex-col gap-4 text-sm text-muted-foreground">
      <SpecViewer spec={info.gitRepo} resourceId={activity.resourceId} title="Pulled configuration" />
      <ActivityAlertZone info={info} activity={activity} />
    </div>
  ),
  GitRepoWebhookReceived: (info) => <WebhookActivityDetails info={info} />,

  OidcProviderCreated: (info, activity) => (
    <SpecViewer spec={info.provider} resourceId={activity.resourceId} title="Initial configuration" />
  ),

  OidcProviderUpdated: (info) => (
    <MonacoDiff original={info.oldProvider} modified={info.newProvider} format="json" title="Configuration changes" />
  ),

  OidcProviderRenamed: (info) => (
    <span className="text-sm text-muted-foreground">
      OIDC provider renamed from <b>{info.oldName}</b> to <b>{info.newName}</b>.
    </span>
  ),

  OidcProviderDeleted: (info, activity) => (
    <SpecViewer spec={info.provider} resourceId={activity.resourceId} title="Deleted configuration" />
  ),

  ActionCreated: (info, activity) => (
    <SpecViewer spec={info.action} resourceId={activity.resourceId} title="Initial configuration" />
  ),

  ActionUpdated: (info) => (
    <MonacoDiff original={info.oldAction} modified={info.newAction} format="json" title="Configuration changes" />
  ),

  ActionRenamed: (info) => (
    <span className="text-sm text-muted-foreground">
      Action renamed from <b>{info.oldName}</b> to <b>{info.newName}</b>.
    </span>
  ),

  ActionDeleted: (info, activity) => (
    <SpecViewer spec={info.action} resourceId={activity.resourceId} title="Deleted configuration" />
  ),

  ActionRunQueued: (info) => <AutomationRunDetails info={info} status="Queued" />,

  ActionRunStarted: (info) => <AutomationRunDetails info={info} status="Started" />,

  ActionRunSucceeded: (info) => <AutomationRunDetails info={info} status="Succeeded" />,

  ActionRunFailed: (info) => <AutomationRunDetails info={info} status="Failed" />,

  ActionRunTimedOut: (info) => <AutomationRunDetails info={info} status="Timed out" />,

  ActionRunCancelled: (info) => <AutomationRunDetails info={info} status="Cancelled" />,

  ActionRunRejected: (info) => <AutomationRunDetails info={info} status="Rejected" />,
};

function DuplicateSource({ source }: { source: Extract<ActivityEventInfo, { source: unknown }>['source'] }) {
  return (
    <div className="flex items-center gap-2">
      <span>Duplicated from</span>
      <TargetCell
        resourceType={source.resourceType}
        resourceId={source.resourceId}
        resourceName={source.resourceName}
      />
    </div>
  );
}

export function ActivityAlertZone({
  info,
  activity,
  title,
  date,
}: {
  info:
    | ActivityEventInfoGitRepoPulled
    | ActivityEventInfoGitRepoCloned
    | ActivityEventInfoDeploymentApplied
    | ActivityEventInfoStackApplied
    | ActivityEventInfoStackRollback
    | null
    | undefined;
  activity: ActivityView | null | undefined;
  title?: string;
  date?: any;
}) {
  if (!activity || !info) return null;
  if (!(activity.status === ActivityStatus.Failure || activity.status === ActivityStatus.Warning)) return null;
  return (
    <AlertMessage
      title={title}
      date={date}
      type={activity.status === ActivityStatus.Failure ? 'error' : 'warning'}>
      {info.result?.message}
    </AlertMessage>
  );
}

function KeyValueBlock({ label, value }: { label: string; value: string | string[] | null | undefined }) {
  return (
    <div className="flex flex-col gap-2 text-sm text-muted-foreground">
      <span className="text-sm font-medium text-foreground">{label}</span>

      <div className="border p-2 rounded-lg">
        {Array.isArray(value) ? (
          value.length > 0 ? (
            <ul className="list-disc pl-4 space-y-1">
              {value.map((item, index) => (
                <li key={`${item}-${index}`}>{item}</li>
              ))}
            </ul>
          ) : (
            <span>-</span>
          )
        ) : value ? (
          <span>{value}</span>
        ) : (
          <span>-</span>
        )}
      </div>
    </div>
  );
}

function AutomationRunDetails({ info, status }: { info: any; status: string }) {
  const details = [
    `Status: ${status}`,
    `Run ID: ${info.runId}`,
    `Trigger: ${info.trigger}`,
    info.exitCode !== undefined && info.exitCode !== null ? `Exit code: ${info.exitCode}` : null,
    info.durationMs !== undefined && info.durationMs !== null ? `Duration: ${formatDurationMs(info.durationMs)}` : null,
    info.reason ? `Reason: ${info.reason}` : null,
    info.errorMessage ? `Error: ${info.errorMessage}` : null,
  ].filter(Boolean) as string[];

  return <KeyValueBlock label="Run details" value={details} />;
}

function BuildRunActivityDetails({ info, status }: { info: any; status: string }) {
  const details = [
    `Status: ${status}`,
    `Run ID: ${info.runId}`,
    `Trigger: ${info.trigger}`,
    info.exitCode !== undefined && info.exitCode !== null ? `Exit code: ${info.exitCode}` : null,
    info.durationMs !== undefined && info.durationMs !== null ? `Duration: ${formatDurationMs(info.durationMs)}` : null,
    info.imageDigest ? `Image digest: ${info.imageDigest}` : null,
    info.errorMessage ? `Error: ${info.errorMessage}` : null,
  ].filter(Boolean) as string[];

  return <KeyValueBlock label="Run details" value={details} />;
}

function formatDurationMs(value: unknown) {
  const ms = Number(value);
  if (!Number.isFinite(ms)) return String(value);
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(1)} s`;
}

function WebhookActivityDetails({
  info,
}: {
  info: ActivityEventInfoGitRepoWebhookReceived | ActivityEventInfoStackWebhookReceived | ActivityEventInfoBuildWebhookReceived;
}) {
  const displayReason = formatWebhookReason(info.reason);
  const title =
    info.status === 'queued'
      ? 'Webhook accepted and queued.'
      : info.status === 'rejected'
        ? 'Webhook rejected.'
        : displayReason || 'Webhook did not trigger an action.';

  return (
    <SpecViewer
      spec={compactWebhookDetails(info, title, displayReason)}
      resourceId={info.requestId}
      title="Webhook details"
    />
  );
}

function compactWebhookDetails(
  info: ActivityEventInfoGitRepoWebhookReceived | ActivityEventInfoStackWebhookReceived | ActivityEventInfoBuildWebhookReceived,
  message: string,
  reason: string | null | undefined,
) {
  return Object.fromEntries(
    Object.entries({
      message,
      reason,
      requestId: info.requestId,
      execution: info.execution,
      authType: info.authType,
      providerEvent: info.eventType,
      deliveryId: info.deliveryId,
      repository: info.repositoryFullName,
      branch: info.branch,
      commit: info.commitSha,
      dispatchedBranch: info.dispatchedBranch,
      dispatchedCommit: info.dispatchedCommitSha,
    }).filter(([, value]) => value),
  );
}

function formatWebhookReason(reason: string | null | undefined) {
  switch (reason) {
    case 'No new commit':
      return 'No deployment needed because the stack already runs the latest commit.';
    case 'No relevant path changes':
      return 'No deployment needed because the commit did not change paths watched by this resource.';
    case 'Branch mismatch':
      return 'No action taken because the webhook branch does not match the configured branch.';
    case 'Unsupported event type':
      return 'No action taken because this provider event is not handled by Citadel.';
    default:
      return reason;
  }
}

function stripStackReleaseSource(stack: StackSnapshot | null | undefined): StackSnapshot | null | undefined {
  if (!stack?.stackRelease || !('source' in stack.stackRelease)) return stack;

  const { source: _source, ...stackRelease } = stack.stackRelease;
  return {
    ...stack,
    stackRelease,
  };
}

function stripBuildProjectSecrets(build: BuildProjectSnapshot | null | undefined): BuildProjectSnapshot | null | undefined {
  if (!build?.webhook?.secret) return build;

  return {
    ...build,
    webhook: {
      ...build.webhook,
      secret: '********',
    },
  };
}

function getActivityReleaseSource(activity: ActivityView): StackReleaseSource | null | undefined {
  switch (activity.info.$type) {
    case 'StackApplied':
      return activity.info.stack?.stackRelease?.source;
    case 'StackRollback':
      return activity.info.newStack?.stackRelease?.source;
    default:
      return null;
  }
}

function ReleaseSourceHeader({ source }: { source: StackReleaseSource }) {
  return (
    <div className="flex flex-col gap-3 ">
      <SourceMetaItem
        icon={<GitBranch className="h-3.5 w-3.5 text-muted-foreground" />}
        value={source.gitRepositoryName}
      />
      <SourceMetaItem icon={<Route className="h-3.5 w-3.5 text-muted-foreground" />} value={source.branch} />
      <SourceMetaItem
        icon={<GitCommitHorizontal className="h-3.5 w-3.5 text-muted-foreground" />}
        value={shortCommit(source.resolvedCommitSha)}
        title={source.resolvedCommitSha}
      />
      {source.workingDirectory ? (
        <SourceMetaItem
          icon={<NotepadText className="h-3.5 w-3.5 text-muted-foreground" />}
          value={source.workingDirectory}
        />
      ) : null}
    </div>
  );
}

function SourceMetaItem({
  icon,
  value,
  title,
}: {
  icon: ReactNode;
  value: string | null | undefined;
  title?: string | null;
}) {
  if (!value) return null;

  return (
    <span className="flex min-w-0 items-center gap-2">
      <span className="shrink-0 text-foreground/80">{icon}</span>
      <span className="truncate" title={title ?? value}>
        {value}
      </span>
    </span>
  );
}

function shortCommit(commit: string | null | undefined) {
  if (!commit) return '-';
  return commit.length > 12 ? commit.slice(0, 12) : commit;
}

function ActivityInfo({ activity }: { activity: ActivityView }) {
  const { info } = activity;

  const renderer = activityInfoRenderers[info.$type];
  if (!renderer) {
    return <SpecViewer spec={info} resourceId={activity.resourceId} title="Activity details" />;
  }

  return renderer(info as never, activity);
}

function RecordedAtCell({ createdAt }: { createdAt: string }) {
  return (
    <div className="flex flex-row items-center gap-2">
      <Calendar className="size-3.5 text-foreground/80" />
      <span>{new Date(createdAt).toLocaleString()}</span>
    </div>
  );
}

interface AlertEventActionsProps {
  status: AlertEventStatus;
  canAcknowledge: boolean;
  canResolve: boolean;
  isAcknowledging: boolean;
  isResolving: boolean;
  onAcknowledge: () => void;
  onResolve: () => void;
}

function AlertEventActions({
  status,
  canAcknowledge,
  canResolve,
  isAcknowledging,
  isResolving,
  onAcknowledge,
  onResolve,
}: AlertEventActionsProps) {
  return (
    <div className="flex flex-row items-center gap-2">
      <NotepadText className="size-3.5 text-foreground/80" />
      <AlertEventStatusCell status={status} />
      {canAcknowledge && (
        <Button
          size="icon-xs"
          variant="outline"
          title="Acknowledge"
          aria-label="Acknowledge"
          disabled={isAcknowledging || isResolving}
          onClick={onAcknowledge}>
          {isAcknowledging ? <LoaderCircle className="h-3.5 w-3.5 animate-spin" /> : <Check className="h-3.5 w-3.5" />}
        </Button>
      )}
      {canResolve && (
        <Button
          size="icon-xs"
          variant="outline"
          title="Resolve"
          aria-label="Resolve"
          disabled={isResolving || isAcknowledging}
          onClick={onResolve}>
          {isResolving ? <LoaderCircle className="h-3.5 w-3.5 animate-spin" /> : <CheckCheck className="h-3.5 w-3.5" />}
        </Button>
      )}
    </div>
  );
}

function PullImageTaskRenderer({ payload, type }: { payload: PullImageParams; type: ResourceType }) {
  const state = useImagePullProgress(payload);
  return <TaskStreamLayout title="Pull Image" refName={payload.imageTag} type={type} state={state as any} />;
}

function ApplyDeployTaskRenderer({ payload, type }: { payload: DeployParams; type: ResourceType }) {
  const state = useApplyDeploymentProgress(payload);
  return <TaskStreamLayout title="Deploy" refName={payload.name} type={type} state={state as any} />;
}

function ApplyStackTaskRenderer({ payload, type }: { payload: StackDeployParams; type: ResourceType }) {
  const state = useApplyStackProgress(payload);
  return <TaskStreamLayout title="Stack" refName={payload.name} type={type} state={state as any} />;
}

function RollbackStackTaskRenderer({ payload, type }: { payload: StackRollbackParams; type: ResourceType }) {
  const state = useRollbackStackProgress(payload);
  const refName = payload.version ? `${payload.name} -> ${payload.version}` : payload.name;
  return <TaskStreamLayout title="Rollback" refName={refName} type={type} state={state as any} />;
}

function BackupRunTaskRenderer({ payload }: { payload: BackupRunParams; type: ResourceType }) {
  const state = useBackupRunProgress(payload);
  return <TaskStreamLayout title="Backup" refName={payload.name} type="BackupPolicy" state={state as any} />;
}

function BackupRunLogsTaskRenderer({ payload }: { payload: BackupRunLogsParams; type: ResourceType }) {
  const { data, isLoading } = useRead('getBackupRunLogs', { id: payload.id }, { enabled: Boolean(payload.id) });
  const logs = data?.data.logs ?? '';

  return (
    <div className="p-2">
      <SheetHeader>
        <SheetTitle>{payload.name} logs</SheetTitle>
        <SheetDescription>{isLoading ? 'Loading backup logs...' : `${payload.trigger} backup run`}</SheetDescription>
      </SheetHeader>

      <div className="p-4 pt-0 pb-2">
        {payload.items.length > 0 && <BackupRunItemsList items={payload.items} />}
        <LogViewer logs={isLoading ? '' : logs} autoScroll={false} />
      </div>
    </div>
  );
}

function BackupRestoreRunTaskRenderer({ payload }: { payload: BackupRestoreRunParams; type: ResourceType }) {
  const state = useBackupRestoreRunProgress(payload);
  return <TaskStreamLayout title="Restore" refName={payload.name} type="BackupPolicy" state={state as any} />;
}

function BackupRestoreRunLogsTaskRenderer({ payload }: { payload: BackupRestoreRunLogsParams; type: ResourceType }) {
  const { data, isLoading } = useRead('getBackupRestoreRunLogs', { id: payload.id }, { enabled: Boolean(payload.id) });
  const logs = data?.data.logs ?? '';

  return (
    <div className="p-2">
      <SheetHeader>
        <SheetTitle>{payload.name} restore logs</SheetTitle>
        <SheetDescription>{isLoading ? 'Loading restore logs...' : `${payload.status} restore run`}</SheetDescription>
      </SheetHeader>

      <div className="p-4 pt-0 pb-2">
        <LogViewer logs={isLoading ? '' : logs} autoScroll={false} />
      </div>
    </div>
  );
}

function AutomationActionRunTaskRenderer({ payload }: { payload: AutomationActionRunParams; type: ResourceType }) {
  const state = useAutomationActionRunProgress(payload);
  const title = payload.mode === 'test' ? 'Test Action' : 'Run Action';
  return <TaskStreamLayout title={title} refName={payload.name} type="AutomationAction" state={state as any} />;
}

function ActivityTaskRenderer({ payload }: { payload: ActivityView; type: ResourceType }) {
  return <TaskActivityLayout activityId={payload.id} />;
}

function AlertEventTaskRenderer({ payload }: { payload: AlertEventView; type: ResourceType }) {
  return <TaskAlertEventLayout alertEventId={payload.id} />;
}

const taskRenderers: Record<string, (props: { payload: any; type: ResourceType }) => ReactNode> = {
  pull: PullImageTaskRenderer,
  deploy: ApplyDeployTaskRenderer,
  stack: ApplyStackTaskRenderer,
  stackRollback: RollbackStackTaskRenderer,
  backupRun: BackupRunTaskRenderer,
  backupRunLogs: BackupRunLogsTaskRenderer,
  backupRestoreRun: BackupRestoreRunTaskRenderer,
  backupRestoreRunLogs: BackupRestoreRunLogsTaskRenderer,
  automationActionRun: AutomationActionRunTaskRenderer,
  activity: ActivityTaskRenderer,
  alertEvent: AlertEventTaskRenderer,
};

export const TaskSheet = memo(function TaskSheet({ type }: { type: ResourceType }) {
  const { state, close } = useTaskSheet(type);

  const handleOpenChange = useCallback(
    (open: boolean) => {
      if (!open) close();
    },
    [close],
  );

  if (!state.open || !state.task) return null;

  const side =
    state.task.kind === 'activity' ||
    state.task.kind === 'alertEvent' ||
    state.task.kind === 'backupRunLogs' ||
    state.task.kind === 'backupRestoreRunLogs'
      ? 'top'
      : 'bottom';
  const Renderer = taskRenderers[state.task.kind];

  return (
    <Sheet open={state.open} onOpenChange={handleOpenChange}>
      <SheetContent
        onOpenAutoFocus={(e) => {
          e.preventDefault();
        }}
        side={side}
        className={`mx-auto w-300 max-w-[100vw] ${side === 'top' ? 'rounded-b-md' : 'rounded-t-md'}`}>
        {Renderer ? (
          <Renderer payload={state.task.payload} type={type} />
        ) : (
          <div className="p-8 text-center text-muted-foreground">
            Task &quot;{state.task.kind}&quot; is not registered.
          </div>
        )}
      </SheetContent>
    </Sheet>
  );
});

export default TaskSheet;

function BackupRunItemsList({ items }: { items: BackupRunItemView[] }) {
  return (
    <div className="mb-3 grid gap-1 rounded-md border p-2">
      {items.map((item) => (
        <div key={item.id} className="grid grid-cols-[1fr_auto_auto] items-center gap-3 text-xs">
          <span className="truncate font-medium">{item.volumeName}</span>
          <span className="text-muted-foreground tabular-nums">
            {(item.bytesAdded ?? 0) > 0 ? byteTransform(item.bytesAdded, 2) : '-'}
          </span>
          <span className="inline-flex items-center gap-1.5">
            <StateIndicator value={item.status} isProcessing={item.status === BackupRunItemStatus.Running} />
            {item.status}
          </span>
        </div>
      ))}
    </div>
  );
}

function useImagePullProgress(params: PullImageParams) {
  const { currentPlatform } = useAppContext();
  const [registryFilter] = useResourceFilter<{ item: RegistryView }>('Registry');

  const registryId = params.registryId ?? registryFilter?.item?.id ?? '';
  const platformId = currentPlatform?.id ?? '';
  const imageTag = params.imageTag;

  const request: PullImageInput = useMemo(
    () => ({
      registryId,
      platformId,
      imageTag,
    }),
    [registryId, platformId, imageTag],
  );

  return useStreamProgress<PullImageInput, PullImageStreamItem>({
    endpoint: 'api/v1/images/pull',
    request,
    successMessage: 'Image pulled successfully',
    errorMessageDefault: 'Failed to pull',
    getError: (item) => item.errorMessage,
  });
}

function useApplyDeploymentProgress(params: DeployParams) {
  const { id, recreate } = params;

  const request: ApplyDeploymentInput = useMemo(() => ({ id, recreate }), [id, recreate]);

  return useStreamProgress<ApplyDeploymentInput, DeploymentStreamItem>({
    endpoint: 'api/v1/deployments/apply',
    request,
    successMessage: 'Deployment applied successfully',
    errorMessageDefault: 'Failed to deploy',
    getError: (item) => item.errorMessage,
  });
}

function useApplyStackProgress(params: StackDeployParams) {
  const { id, recreate } = params;
  const queryClient = useQueryClient();

  const request: ApplyStackInput = useMemo(() => ({ id, recreate }), [id, recreate]);

  const state = useStreamProgress<ApplyStackInput, StackStreamItem>({
    endpoint: 'api/v1/stacks/apply',
    request,
    successMessage: 'Stack applied successfully',
    errorMessageDefault: 'Failed to deploy',
    compactDockerComposeOutput: true,
    getError: (item) => (item.exitCode !== 0 ? item.message : undefined),
    getMessageSeverity: getStackStreamMessageSeverity,
  });

  useEffect(() => {
    if (!state.isSuccess || state.error) return;

    queryClient.invalidateQueries({ queryKey: ['listStacks'] });
    queryClient.invalidateQueries({ queryKey: ['getStack', { stackId: id }] });
  }, [id, queryClient, state.error, state.isSuccess]);

  return state;
}

function getStackStreamMessageSeverity(item: StackStreamItem) {
  if (
    item.stackStatus === StackReleaseStatus.Degraded ||
    item.stackStatus?.toString() === StackReleaseStatus.Degraded ||
    item.stackStatus?.toString() === '6'
  ) {
    return 'warning';
  }

  const message = (item.progressMessage ?? item.message)?.trim();
  if (message === 'Stack applied with status Degraded.') {
    return 'warning';
  }

  return undefined;
}

function useRollbackStackProgress(params: StackRollbackParams) {
  const { stackId, releaseId } = params;
  const queryClient = useQueryClient();

  const request: RollbackStackInput = useMemo(() => ({ stackId, releaseId }), [stackId, releaseId]);

  const state = useStreamProgress<RollbackStackInput, StackStreamItem>({
    endpoint: 'api/v1/stacks/rollback',
    request,
    successMessage: 'Stack rolled back successfully',
    errorMessageDefault: 'Failed to rollback',
    compactDockerComposeOutput: true,
    getError: (item) => (item.exitCode !== 0 ? item.message : undefined),
  });

  useEffect(() => {
    if (!state.isSuccess || state.error) return;

    queryClient.invalidateQueries({ queryKey: ['listStacks'] });
    queryClient.invalidateQueries({ queryKey: ['getStack', { stackId }] });
  }, [queryClient, stackId, state.error, state.isSuccess]);

  return state;
}

function useBackupRunProgress(params: BackupRunParams) {
  const { id, trigger, triggerSourceId } = params;
  const queryClient = useQueryClient();

  const request: QueueBackupRunInput = useMemo(() => ({ trigger, triggerSourceId }), [trigger, triggerSourceId]);

  const state = useStreamProgress<QueueBackupRunInput, BackupRunStreamItem>({
    endpoint: `api/v1/backupPolicies/${encodeURIComponent(id)}/run`,
    request,
    successMessage: 'Backup run finished',
    errorMessageDefault: 'Failed to run backup',
    pendingMessage: 'Starting backup run...',
    streamFieldIsMetadata: true,
    getError: (item) => {
      if (
        item.status === BackupRunStatus.Failed ||
        item.status === BackupRunStatus.Rejected ||
        item.status === BackupRunStatus.TimedOut ||
        item.status === BackupRunStatus.Interrupted
      ) {
        return item.message;
      }

      return null;
    },
  });

  useEffect(() => {
    if (!state.isSuccess && !state.error) return;

    queryClient.invalidateQueries({ queryKey: ['listBackupPolicies'] });
    queryClient.invalidateQueries({ queryKey: ['getBackupPolicy', { id }] });
    queryClient.invalidateQueries({ queryKey: ['listBackupRuns'] });
    queryClient.invalidateQueries({ queryKey: ['listBackupRuns', { query: { policyId: id, limit: 50 } }] });
  }, [id, queryClient, state.error, state.isSuccess]);

  return state;
}

function useBackupRestoreRunProgress(params: BackupRestoreRunParams) {
  const { id, targetPlatformId, targetVolumeName, overwriteExisting } = params;
  const queryClient = useQueryClient();

  const request: RestoreVolumeInput = useMemo(
    () => ({
      targetPlatformId,
      targetVolumeName,
      overwriteExisting,
    }),
    [overwriteExisting, targetPlatformId, targetVolumeName],
  );

  const state = useStreamProgress<RestoreVolumeInput, BackupRestoreRunStreamItem>({
    endpoint: `api/v1/backupRuns/${encodeURIComponent(id)}/restoreVolume/run`,
    request,
    successMessage: 'Restore run finished',
    errorMessageDefault: 'Failed to restore backup',
    pendingMessage: 'Starting restore run...',
    streamFieldIsMetadata: true,
    getError: (item) => {
      if (
        item.status === BackupRestoreStatus.Failed ||
        item.status === BackupRestoreStatus.Rejected ||
        item.status === BackupRestoreStatus.TimedOut ||
        item.status === BackupRestoreStatus.Cancelled
      ) {
        return item.message;
      }

      return null;
    },
  });

  useEffect(() => {
    if (!state.isSuccess && !state.error) return;

    queryClient.invalidateQueries({ queryKey: ['listBackupRestoreRuns'] });
    queryClient.invalidateQueries({ queryKey: ['listBackupRestoreRuns', { query: { backupRunId: id, limit: 50 } }] });
    queryClient.invalidateQueries({ queryKey: ['getBackupRun', { id }] });
    queryClient.invalidateQueries({ queryKey: ['listBackupRuns'] });
  }, [id, queryClient, state.error, state.isSuccess]);

  return state;
}

function useAutomationActionRunProgress(params: AutomationActionRunParams) {
  const { id, mode } = params;
  const queryClient = useQueryClient();

  const request: RunAutomationActionInput | TestAutomationActionInput = useMemo(() => {
    if (params.mode === 'run') {
      return { argsJson: params.argsJson, timeoutSeconds: params.timeoutSeconds };
    }

    return {
      code: params.code,
      argsJson: params.argsJson,
      defaultArgsJson: params.defaultArgsJson,
      timeoutSeconds: params.timeoutSeconds,
      runAsActorId: params.runAsActorId,
    };
  }, [params]);

  const state = useStreamProgress<RunAutomationActionInput | TestAutomationActionInput, AutomationActionRunStreamItem>({
    endpoint: `api/v1/automation/actions/${encodeURIComponent(id)}/${mode}`,
    request,
    successMessage: mode === 'test' ? 'Automation test run finished' : 'Automation action finished',
    errorMessageDefault: mode === 'test' ? 'Failed to run automation test' : 'Failed to run automation action',
    pendingMessage: mode === 'test' ? 'Starting automation test...' : 'Starting automation action...',
    getError: (item) => item.errorMessage ?? item.error?.message,
  });

  useEffect(() => {
    if (!state.isSuccess && !state.error) return;

    queryClient.invalidateQueries({ queryKey: ['listAutomationActions'] });
    queryClient.invalidateQueries({ queryKey: ['getAutomationAction', { id }] });
    queryClient.invalidateQueries({ queryKey: ['listAutomationActionRuns', { id }] });
  }, [id, queryClient, state.error, state.isSuccess]);

  return state;
}
