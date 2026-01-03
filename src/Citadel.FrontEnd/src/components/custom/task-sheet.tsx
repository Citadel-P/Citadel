import { ReactNode, useMemo } from 'react';
import { Clock, LoaderCircle } from 'lucide-react';
import { ResourceType } from '@/api/types';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import CodeHighlight from '@/components/custom/code-highlight';
import { useResourceFilter, useTaskSheet } from '@/lib/atoms';
import { ResourceComponents } from '@/features';
import { useAppContext } from '@/lib/context/app-context';
import { useStreamProgress } from '@/lib/hooks';
import {
  ApplyDeploymentInput,
  DeploymentStreamItem,
  PullImageInput,
  PullImageStreamItem,
  RegistryView,
} from '@/api/generated/api.types';

interface PullImageParams {
  imageTag: string;
  registryId: string;
}

type DeployParams = { name: string; } & ApplyDeploymentInput

export type TaskSpec =
  | { kind: 'pull'; payload: PullImageParams }
  | { kind: 'deploy'; payload: DeployParams }
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
        <CodeHighlight code={text} language="text" autoScroll />
      </div>
    </>
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

const taskRenderers: Record<string, (props: { payload: any; type: ResourceType }) => ReactNode> = {
  pull: PullImageTaskRenderer,
  deploy: ApplyDeployTaskRenderer,
};

export function TaskSheet({ type }: { type: ResourceType }) {
  const { state, close } = useTaskSheet(type);

  if (!state.open || !state.task) return null;

  const Renderer = taskRenderers[state.task.kind];

  return (
    <Sheet open={state.open} onOpenChange={(open) => (!open ? close() : undefined)}>
      <SheetContent side="bottom" className="mx-auto w-[1200px] max-w-[100vw] rounded-t-md">
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
