import { Play, Pause, RotateCcw, Ban, Trash } from 'lucide-react';
import { useContextSelector } from 'use-context-selector';
import { ContainersContext } from './ContainersProvider';
import { useAvailableActions } from './hooks/useAvailableActions';
import { useDialog } from '@/hooks/useDialog';
import { DeleteContainerDialog } from './dialogs/DeleteContainerDialog';

export const ActionBar = () => {
  const deleteDialog = useDialog();
  const containers = useContextSelector(ContainersContext, (v) => v?.containers);
  const selectedContainersId = useContextSelector(ContainersContext, (v) => v?.selectedRowsId);
  const { availableActions, isPending, requestPatch } = useAvailableActions(
    containers?.filter((s) => selectedContainersId!.some((i) => i === s.id)) ?? [],
  );

  if (!selectedContainersId!.length) return <></>;

  return (
    <div className="min-h-20 absolute bottom-0 w-full p-2 bg-background sm:flex sm:justify-between">
      <div className="flex-1 text-xs text-muted-foreground mt-2">
        {selectedContainersId!.length} of {containers!.length} container(s) selected.
      </div>
      <div className="mt-1">
        <button
          type="button"
          disabled={!availableActions.canStart || isPending}
          onClick={() => requestPatch('start')}
          className="inline-flex items-center rounded-l-lg border border-border bg-background px-2 py-2 text-xs text-foreground font-medium enabled:hover:bg-foreground/5 enabled:hover:text-blue-700 disabled:cursor-not-allowed disabled:opacity-60">
          <Play className="mr-1 h-3 w-3" />
          Start
        </button>
        <button
          type="button"
          onClick={() => requestPatch('stop')}
          disabled={!availableActions.canStop || isPending}
          className="inline-flex items-center border border-border border-r border-t bg-background px-2 py-2 text-xs text-foreground font-medium enabled:hover:bg-foreground/5 enabled:hover:text-blue-700 disabled:cursor-not-allowed disabled:opacity-60">
          <Ban className="mr-1 h-3 w-3" />
          Stop
        </button>
        <button
          type="button"
          onClickCapture={() => requestPatch('pause')}
          disabled={!availableActions.canPause || isPending}
          className="inline-flex items-center border-border border border-r border-t bg-background px-2 py-2 text-xs text-foreground font-medium enabled:hover:bg-foreground/5 enabled:hover:text-blue-700 disabled:cursor-not-allowed disabled:opacity-60">
          <Pause className="mr-1 h-3 w-3" />
          Pause
        </button>
        <button
          type="button"
          onClick={() => requestPatch('restart')}
          disabled={!availableActions.canRestart || isPending}
          className="inline-flex items-center border-border border border-r border-t bg-background px-2 py-2 text-xs text-foreground font-medium enabled:hover:bg-foreground/5 enabled:hover:text-blue-700 focus:z-10 disabled:cursor-not-allowed disabled:opacity-60">
          <RotateCcw className="mr-1 h-3 w-3" />
          Restart
        </button>
        <button
          type="button"
          {...deleteDialog.triggerProps}
          disabled={!availableActions.canDelete || isPending}
          className="inline-flex items-center rounded-r-md border border-border bg-background px-2 py-2 text-xs font-medium text-danger enabled:hover:text-danger/80 focus:z-10 disabled:cursor-not-allowed disabled:opacity-60">
          <Trash className="mr-1 h-3 w-3" />
          Delete
        </button>
        <DeleteContainerDialog
          dialog={deleteDialog}
          containersIds={selectedContainersId ?? []}
          onDelete={() => requestPatch('delete')}
        />
      </div>
    </div>
  );
};
