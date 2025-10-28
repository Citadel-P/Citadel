import { DockerResourceType } from '@/api/types';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import CodeHighlight from '@/components/custom/code-highlight';
import { usePullProgress as useImagePullProgress } from '@/features/docker-resources/images/hooks/usePullProgress';
import { Check, CircleX, LoaderCircle } from 'lucide-react';
import { useTaskSheet } from '@/lib/atoms';
import { ReactNode } from 'react';

export function TaskSheet({ type }: { type: DockerResourceType }) {
  const { state, close } = useTaskSheet(type);

  if (!state.open || !state.task) return null;

  const title = getTaskTitleForKind(state.task);
  const Renderer = taskRenderers[state.task.kind as TaskType];

  return (
    <Sheet open={state.open} onOpenChange={(open) => (!open ? close() : undefined)}>
      <SheetContent side="bottom" className="mx-auto w-[1200px] max-w-[100vw] rounded-t-md">
        {Renderer ? (
          <Renderer payload={(state.task as any).payload}>
            {({ status, content }) => (
              <>
                <SheetHeader>
                  <SheetTitle>{title}</SheetTitle>
                  <SheetDescription>{status}</SheetDescription>
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
            <div className="pt-0 pb-4 px-4">Not registred</div>
          </>
        )}
      </SheetContent>
    </Sheet>
  );
}

type PullImageTaskRendererProps = {
  payload: PullImageParams;
  children: (slots: TaskRendererSlots) => ReactNode;
};

function PullImageTaskRenderer({ payload, children }: PullImageTaskRendererProps) {
  const { text, status } = useImagePullProgress({
    imageTag: payload.imageTag,
    repository: payload.repository,
    registryName: payload.registryName,
  });

  const statusIcon =
    status === 'pending' ? (
      <LoaderCircle className="ml-1 h-4 w-4 animate-spin" />
    ) : status === 'error' ? (
      <CircleX className="text-red-500 ml-1 h-4 w-4" />
    ) : (
      <Check className="text-green-500 ml-1 h-4 w-4" />
    );

  const statusEl = (
    <div className="flex items-center gap-2">
      <span className="text-sm text-muted-foreground">Status</span>
      {statusIcon}
    </div>
  );

  const bodyEl = (
    <CodeHighlight
      code={text}
      language="text"
      autoScroll
      className="p-6! rounded-md shadow-xs overflow-x-auto overflow-y-auto max-h-[50vh] mx-auto w-full"
      lineWrapperClassName="table-row flex-col-reverse"
    />
  );

  return <>{children({ status: statusEl, content: bodyEl })}</>;
}

type TaskRendererSlots = { status: React.ReactElement | null; content: React.ReactElement };
type TaskRendererComponent<P> = (props: {
  payload: P;
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
