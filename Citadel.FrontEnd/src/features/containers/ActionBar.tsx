import { Play, Pause, RotateCcw, Ban, Trash } from 'lucide-react';
import { useContextSelector } from 'use-context-selector';
import { ContainersContext } from './ContainersProvider';
import { useAvailableActions } from './hooks/useAvailableActions';

export const ActionBar = () => {
  const containers = useContextSelector(ContainersContext, (v) => v?.containers)!;
  const selectedRows = useContextSelector(ContainersContext, (v) => v?.selectedRows)!;
  const setDialogData = useContextSelector(ContainersContext, (v) => v?.setDialogData)!;
  const { availableActions, isPending, requestPatch } = useAvailableActions(selectedRows);

  if (!selectedRows.length) return <></>;

  return (
    <div
      className="h-14 fixed -translate-x-6 inset-x-0 inset-shadow-xs bottom-0 shadow-lg p-2 bg-background sm:flex sm:justify-between"
      style={{
        left: 'var(--sidebar-width)',
        width: 'calc(100% - var(--sidebar-width))',
      }}>
      <div className="flex-1 text-xs text-muted-foreground mt-2">
        {selectedRows.length} of {containers!.length} container(s) selected.
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
          onClick={() => setDialogData({ open: true, currentSelection: selectedRows })}
          disabled={!availableActions.canDelete || isPending}
          className="inline-flex items-center rounded-r-md border border-border px-2 py-2 text-background bg-danger hover:bg-danger/85 font-medium text-xs disabled:cursor-not-allowed disabled:opacity-60">
          <Trash className="mr-1 h-3 w-3" />
          Delete
        </button>
      </div>
    </div>
  );
};
