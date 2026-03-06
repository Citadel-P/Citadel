import { ReactNode, useMemo } from 'react';
import { Calendar, Clock, LoaderCircle } from 'lucide-react';
import { ResourceType } from '@/api/types';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { useResourceFilter, useTaskSheet } from '@/lib/atoms';
import { ResourceComponents } from '@/features';
import { useAppContext } from '@/lib/context/app-context';
import { ActorCell, LogViewer, TargetCell } from '@/components/custom/common';
import { useRead, useStreamProgress } from '@/lib/hooks';
import {
  ActivityView,
  ApplyDeploymentInput,
  DeploymentStreamItem,
  ActivityEventInfo,
  PullImageInput,
  PullImageStreamItem,
  RegistryView,
} from '@/api/generated/api.types';
import { formatActivityEvent, serializeData } from '@/lib/utils';
import Loader from '../ui/loader';
import { MonacoDiff, MonacoEditor } from '@/lib/monaco';

interface PullImageParams {
  imageTag: string;
  registryId: string;
}

type DeployParams = { name: string } & ApplyDeploymentInput;

export type TaskSpec =
  | { kind: 'pull'; payload: PullImageParams }
  | { kind: 'deploy'; payload: DeployParams }
  | { kind: 'activity'; payload: ActivityView }
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
              {status === 'pending' && <LoaderCircle className="h-4 w-4 animate-spin" />}
              <span className={status === 'error' ? 'text-destructive' : status === 'success' ? 'text-success' : ''}>
                {Icon}
              </span>
              <span>{refName}</span>
            </span>

            <span className="inline-flex flex-row gap-2 items-center">
              <Clock className="w-4 h-4" />
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
            <ActorCell type={activity.actorType} id={activity.actorId} name={activity.actorName} />
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

function SpecViewer({ spec, title, resourceId }: { spec: unknown; title: string; resourceId: string | null }) {
  return (
    <MonacoEditor
      value={serializeData(spec)}
      filename={`inspect-${resourceId}.yaml`}
      className="my-0 mx-0 min-h-[200px]"
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
    <MonacoDiff original={info.oldSpec} modified={info.newSpec} format="yaml" title="Configuration changes" />
  ),

  DeploymentCreated: (info, activity) => (
    <SpecViewer spec={info.spec} resourceId={activity.resourceId} title="Initial configuration" />
  ),

  DeploymentDeleted: (info, activity) => (
    <SpecViewer spec={info.spec} resourceId={activity.resourceId} title="Deleted configuration" />
  ),

  DeploymentApplied: (info, activity) => (
    <div className="flex flex-col gap-4 text-sm text-muted-foreground">
      {info.containerIds && (
        <>
          <SpecViewer spec={info.spec} resourceId={activity.resourceId} title="Applied configuration" />
          <KeyValueBlock label="Container id" value={info.containerIds} />
        </>
      )}
      {info.reason && <span>{info.reason}</span>}
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

  AlertRuleUpdated: (info) => (
    <MonacoDiff original={info.oldRule} modified={info.newRule} format="json" title="Configuration changes" />
  ),

  AlertRuleCreated: (info, activity) => (
    <SpecViewer spec={info.alertRule} resourceId={activity.resourceId} title="Initial configuration" />
  ),

  AlertRuleDeleted: (info, activity) => (
    <SpecViewer spec={info.alertRule} resourceId={activity.resourceId} title="Deleted configuration" />
  ),
};

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

function PullImageTaskRenderer({ payload, type }: { payload: PullImageParams; type: ResourceType }) {
  const state = useImagePullProgress(payload);
  return <TaskStreamLayout title="Pull Image" refName={payload.imageTag} type={type} state={state as any} />;
}

function ApplyDeployTaskRenderer({ payload, type }: { payload: DeployParams; type: ResourceType }) {
  const state = useApplyDeploymentProgress(payload);
  return <TaskStreamLayout title="Deploy" refName={payload.name} type={type} state={state as any} />;
}

function ActivityTaskRenderer({ payload, type }: { payload: ActivityView; type: ResourceType }) {
  return <TaskActivityLayout activityId={payload.id} />;
}

const taskRenderers: Record<string, (props: { payload: any; type: ResourceType }) => ReactNode> = {
  pull: PullImageTaskRenderer,
  deploy: ApplyDeployTaskRenderer,
  activity: ActivityTaskRenderer,
};

export function TaskSheet({ type }: { type: ResourceType }) {
  const { state, close } = useTaskSheet(type);
  const side = type === 'Activity' ? 'top' : 'bottom';

  if (!state.open || !state.task) return null;

  const Renderer = taskRenderers[state.task.kind];

  return (
    <Sheet open={state.open} onOpenChange={(open) => (!open ? close() : undefined)}>
      <SheetContent
        onOpenAutoFocus={(e) => {
          e.preventDefault();
        }}
        side={side}
        className={`mx-auto w-[1200px] max-w-[100vw] ${side === 'top' ? 'rounded-b-md' : 'rounded-t-md'}`}>
        {Renderer ? (
          <Renderer payload={state.task.payload} type={type} />
        ) : (
          <div className="p-8 text-center text-muted-foreground">Task "{state.task.kind}" is not registered.</div>
        )}
      </SheetContent>
    </Sheet>
  );
}

export default TaskSheet;

function useImagePullProgress(params: PullImageParams) {
  const { currentPlatform } = useAppContext();
  const [registryFilter] = useResourceFilter<{ item: RegistryView }>('Registry');

  const request: PullImageInput = useMemo(
    () => ({
      registryId: params.registryId ?? registryFilter?.item?.name ?? '',
      platformId: currentPlatform?.id ?? '',
      imageTag: params.imageTag,
    }),
    [currentPlatform, registryFilter, params],
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
  const request: ApplyDeploymentInput = useMemo(
    () => ({
      id: params.id,
      recreate: params.recreate,
    }),
    [params],
  );

  return useStreamProgress<ApplyDeploymentInput, DeploymentStreamItem>({
    endpoint: 'api/v1/deployments/apply',
    request,
    successMessage: 'Deployment applied successfully',
    errorMessageDefault: 'Failed to deploy',
    getError: (item) => item.errorMessage,
  });
}
