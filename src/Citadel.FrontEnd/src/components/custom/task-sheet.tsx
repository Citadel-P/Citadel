import { memo, ReactNode, useCallback, useMemo } from 'react';
import { Calendar, Check, CheckCheck, Clock, LoaderCircle, NotepadText } from 'lucide-react';
import { ResourceType } from '@/api/types';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { useResourceFilter, useTaskSheet } from '@/lib/atoms';
import { ResourceComponents } from '@/features';
import { useAppContext } from '@/lib/context/app-context';
import { ActorCell, AlertEventStatusCell, LogViewer, TargetCell } from '@/components/custom/common';
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
  ApplyStackInput,
  StackStreamItem,
} from '@/api/generated/api.types';
import { formatActivityEvent, serializeData } from '@/lib/utils';
import Loader from '../ui/loader';
import { MonacoDiff, MonacoEditor } from '@/lib/monaco';
import { Button } from '@/components/ui/button';
import { AlertMessage } from './alert-message';

interface PullImageParams {
  imageTag: string;
  registryId: string;
}

type DeployParams = { name: string } & ApplyDeploymentInput;
type StackDeployParams = { name: string } & ApplyStackInput;

export type TaskSpec =
  | { kind: 'pull'; payload: PullImageParams }
  | { kind: 'deploy'; payload: DeployParams }
  | { kind: 'activity'; payload: ActivityView }
  | { kind: 'alertEvent'; payload: AlertEventView }
  | { kind: 'build'; payload: Record<string, unknown> }
  | { kind: 'stack'; payload: Record<string, unknown> };

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
  const { status, elapsedLabel, text } = state;
  const Icon = ResourceComponents[type].Icon;

  return (
    <>
      <SheetHeader>
        <SheetTitle>{title}</SheetTitle>
        <SheetDescription>
          <span className="inline-flex flex-wrap items-center gap-x-4 gap-y-1">
            <span className="inline-flex flex-row gap-2 items-center">
              {status === 'pending' && <LoaderCircle className="h-3.5 w-3.5 animate-spin" />}
              <span className={status === 'error' ? 'text-destructive' : status === 'success' ? 'text-success' : ''}>
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
        <LogViewer logs={text} autoScroll={true} />
      </div>
    </>
  );
}

function TaskActivityLayout({ activityId }: { activityId: string }) {
  const { data, isLoading } = useRead('getActivity', { id: activityId });

  if (isLoading) return <Loader />;

  const activity = data!.data;

  return (
    <div className="p-2">
      <SheetHeader>
        <SheetTitle>{formatActivityEvent(activity.eventType)}</SheetTitle>
        <SheetDescription asChild>
          <div className="flex flex-col gap-3">
            <ActorCell type={activity.actorType} name={activity.actorName} />
            <TargetCell
              resourceType={activity.resourceType}
              resourceId={activity.resourceId ?? ''}
              resourceName={activity.resourceName}
            />
            <RecordedAtCell createdAt={activity.createdAt} />
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

  DeploymentDeleted: (info, activity) => (
    <SpecViewer spec={info.deployment} resourceId={activity.resourceId} title="Deleted configuration" />
  ),

  DeploymentApplied: (info, activity) => (
    <div className="flex flex-col gap-4 text-sm text-muted-foreground">
      <SpecViewer spec={info.deployment} resourceId={activity.resourceId} title="Applied configuration" />
      {activity.status === ActivityStatus.Success && (
        <KeyValueBlock label="Container id" value={info.result.containerIds ?? []} />
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
    <SpecViewer spec={info.stack} resourceId={activity.resourceId} title="Initial configuration" />
  ),

  StackUpdated: (info) => (
    <MonacoDiff original={info.oldStack} modified={info.newStack} format="yaml" title="Configuration changes" />
  ),

  StackApplied: (info, activity) => (
    <div className="flex flex-col gap-4 text-sm text-muted-foreground">
      <SpecViewer spec={info.stack} resourceId={activity.resourceId} title="Applied configuration" />
      {activity.status === ActivityStatus.Success && (
        <KeyValueBlock label="Container id" value={info.result.containerIds ?? []} />
      )}
      <ActivityAlertZone info={info} activity={activity} />
    </div>
  ),
  StackDegraded: (info) => <span className="text-sm text-muted-foreground">{info.reason}</span>,

  StackStarted: (info) => <KeyValueBlock label="Output" value={info.result} />,
  StackStopped: (info) => <KeyValueBlock label="Output" value={info.result} />,
  StackPaused: (info) => <KeyValueBlock label="Output" value={info.result} />,

  StackRenamed: (info) => (
    <span className="text-sm text-muted-foreground">
      Deployment renamed from <b>{info.oldName}</b> to <b>{info.newName}</b>.
    </span>
  ),
  StackDeleted: (info, activity) => (
    <SpecViewer spec={info.stack} resourceId={activity.resourceId} title="Deleted configuration" />
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
};

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
    | ActivityEventInfoStackApplied;
  activity: ActivityView;
  title?: string;
  date?: any;
}) {
  if (!(activity.status === ActivityStatus.Failure || activity.status === ActivityStatus.Warning)) return;
  return (
    <AlertMessage
      title={title}
      date={date}
      type={activity.status === ActivityStatus.Failure ? 'error' : 'warning'}
      children={info.result?.message}
    />
  );
}

function KeyValueBlock({ label, value }: { label: string; value: string | any[] }) {
  return (
    <div className="flex flex-col gap-2 text-sm text-muted-foreground">
      <span className="text-sm font-medium text-foreground">{label}</span>
      <div className="border p-2 rounded-lg">
        <span>{value}</span>
      </div>
    </div>
  );
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

  const side = state.task.kind === 'activity' || state.task.kind === 'alertEvent' ? 'top' : 'bottom';
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
          <div className="p-8 text-center text-muted-foreground">Task "{state.task.kind}" is not registered.</div>
        )}
      </SheetContent>
    </Sheet>
  );
});

export default TaskSheet;

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

  const request: ApplyStackInput = useMemo(() => ({ id, recreate }), [id, recreate]);

  return useStreamProgress<ApplyStackInput, StackStreamItem>({
    endpoint: 'api/v1/stacks/apply',
    request,
    successMessage: 'Stack applied successfully',
    errorMessageDefault: 'Failed to deploy',
    getError: (item) => (item.exitCode != 0 ? item.message : undefined),
  });
}
