import { DockerResourceType } from '@/api/types';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import CodeHighlight from '@/components/custom/code-highlight';
import { usePullProgress as useImagePullProgress } from '@/features/docker-resources/images/hooks/usePullProgress';
import { Clock, LoaderCircle } from 'lucide-react';
import { useTaskSheet } from '@/lib/atoms';
import { ReactNode, useMemo } from 'react';
import { DockerResourceComponents } from '@/features/docker-resources';

export function TaskSheet({ type }: { type: DockerResourceType }) {
  const { state, close } = useTaskSheet(type);

  if (!state.open || !state.task) return null;

  const title = getTaskTitleForKind(state.task);
  const Renderer = taskRenderers[state.task.kind as TaskType];

  return (
    <Sheet open={state.open} onOpenChange={(open) => (!open ? close() : undefined)}>
      <SheetContent side="bottom" className="mx-auto w-[1200px] max-w-[100vw] rounded-t-md">
        {Renderer ? (
          <Renderer payload={(state.task as any).payload} type={type}>
            {({ description, content }) => (
              <>
                <SheetHeader>
                  <SheetTitle>{title}</SheetTitle>
                  <SheetDescription>{description}</SheetDescription>
                </SheetHeader>
                <div className="pt-0 pb-4 px-4">{content}</div>
              </>
            )}
          </Renderer>
        ) : (
          <>
            <SheetHeader>
              <SheetTitle>{title}</SheetTitle>
              <SheetDescription></SheetDescription>
            </SheetHeader>
            <div className="pt-0 pb-4 px-4">Not registered</div>
          </>
        )}
      </SheetContent>
    </Sheet>
  );
}

type PullImageTaskRendererProps = {
  type: DockerResourceType;
  payload: PullImageParams;
  children: (slots: TaskRendererSlots) => ReactNode;
};

function PullImageTaskRenderer({ payload, type, children }: PullImageTaskRendererProps) {
  const { text, status, elapsedLabel } = useImagePullProgress({
    imageTag: payload.imageTag,
    repository: payload.repository,
    registryName: payload.registryName,
  });

  const refName = useMemo(
    () => `${payload.repository ? payload.repository + ':' : ''}${payload.imageTag}`,
    [payload.repository, payload.imageTag],
  );

  const description = (
    <span className="inline-flex flex-wrap items-center gap-x-4 gap-y-1">
      <span className="inline-flex flex-row gap-2 items-center">
        {status === 'pending' && <LoaderCircle className="h-4 w-4 animate-spin" />}
        {status === 'error' && <span className="text-destructive">{DockerResourceComponents[type].Icon}</span>}
        {status === 'success' && <span className="text-success">{DockerResourceComponents[type].Icon}</span>}
        <span>{refName}</span>
      </span>

      <span className="inline-flex flex-row gap-2 items-center">
        <Clock className="w-4 h-4" />
        <span>{elapsedLabel} seconds</span>{' '}
      </span>
    </span>
  );

  const content = <CodeHighlight code={text} language="text" autoScroll />;

  return <>{children({ description, content })}</>;
}

type TaskRendererSlots = {
  description: React.ReactElement | string | null;
  content: React.ReactElement;
};
type TaskRendererComponent<P> = (props: {
  payload: P;
  type: DockerResourceType;
  children: (slots: TaskRendererSlots) => ReactNode;
}) => React.ReactElement;

const taskRenderers: Partial<Record<TaskType, TaskRendererComponent<any>>> = {
  pull: PullImageTaskRenderer,
};

function getTaskTitleForKind(task: TaskSpec) {
  switch (task.kind) {
    case 'pull':
      return 'Pull Image';
    case 'build':
      return 'Build Image';
    case 'deploy':
      return 'Deploy';
    case 'stack':
      return 'Stack';
    default:
      return 'Task';
  }
}

export type TaskType = 'pull' | 'build' | 'deploy' | 'stack';

export interface PullImageParams {
  repository: string;
  imageTag: string;
  registryName?: string;
}

export type TaskSpec =
  | { kind: 'pull'; payload: PullImageParams }
  | { kind: 'build'; payload: Record<string, unknown> }
  | { kind: 'deploy'; payload: Record<string, unknown> }
  | { kind: 'stack'; payload: Record<string, unknown> };

export interface TaskSheetState {
  open: boolean;
  task?: TaskSpec;
}

export default TaskSheet;
