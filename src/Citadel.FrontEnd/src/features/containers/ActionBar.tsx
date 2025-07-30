import { Play, Pause, RotateCcw, Ban, Trash } from 'lucide-react';
import { useAvailableActions } from './hooks/useAvailableActions';
import { useMemo } from 'react';
import { ActionBarButton } from '@/components/ui/ActionBarButton';
import { useContainersContext } from './ContainersContext';

export const ActionBar = () => {
  const { containers, selectedRows, setDialogData } = useContainersContext();
  const { availableActions, isPending, requestPatch } = useAvailableActions(selectedRows);

  const selectedCount = useMemo(() => selectedRows?.length, [selectedRows]);
  const containerCount = useMemo(() => containers?.length, [containers]);

  if (!selectedCount) return null;

  return (
    <div
      className="h-14 fixed -translate-x-5 inset-x-0 bottom-0 shadow-lg p-2 bg-background sm:flex sm:justify-between"
      style={{
        left: 'var(--sidebar-width)',
        width: 'calc(100% - var(--sidebar-width))',
      }}>
      <div className="flex-1 text-xs text-muted-foreground mt-2">
        {selectedCount} of {containerCount} container(s) selected.
      </div>
      <div className="mt-1">
        <ActionBarButton
          onClick={() => requestPatch('start')}
          disabled={!availableActions?.canStart || isPending}
          icon={Play}
          label="Start"
          className="rounded-l-lg"
          ariaLabel="Start selected containers"
        />
        <ActionBarButton
          onClick={() => requestPatch('stop')}
          disabled={!availableActions?.canStop || isPending}
          icon={Ban}
          label="Stop"
          ariaLabel="Stop selected containers"
        />
        <ActionBarButton
          onClick={() => requestPatch('pause')}
          disabled={!availableActions?.canPause || isPending}
          icon={Pause}
          label="Pause"
          ariaLabel="Pause selected containers"
        />
        <ActionBarButton
          onClick={() => requestPatch('restart')}
          disabled={!availableActions?.canRestart || isPending}
          icon={RotateCcw}
          label="Restart"
          ariaLabel="Restart selected containers"
        />
        <ActionBarButton
          onClick={() => setDialogData({ open: true, currentSelection: selectedRows })}
          disabled={!availableActions?.canDelete || isPending}
          icon={Trash}
          label="Delete"
          ariaLabel="Delete selected containers"
          className="inline-flex items-center rounded-r-md border border-border px-2 py-2 text-background bg-danger enabled:hover:bg-danger/85 enabled:hover:text-background font-medium text-xs disabled:cursor-not-allowed disabled:opacity-60"
        />
      </div>
    </div>
  );
};
