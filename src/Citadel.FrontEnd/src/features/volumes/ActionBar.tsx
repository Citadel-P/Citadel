import { Trash, SearchCode } from 'lucide-react';
import { ActionBarButton } from '@/components/ui/ActionBarButton';
import { useVolumesContext } from './VolumesContext';

export const ActionBar = () => {
  const { setDialogData, setSheetOpen, setCurrentVolume, selectedRows, volumes } = useVolumesContext();
  const actions: VolumeActionsState = {
    canDelete: (selectedRows?.length ?? 0) > 0 && selectedRows?.find((row) => row.inUse) === undefined,
    canInspect: selectedRows?.length === 1,
  };

  const handleInspectClick = () => {
    setSheetOpen(true);
    if (selectedRows) setCurrentVolume(selectedRows[0]);
  };

  if (!selectedRows?.length) return null;

  return (
    <div
      className="h-14 fixed -translate-x-5 inset-x-0 bottom-0 shadow-lg p-2 bg-background sm:flex sm:justify-between"
      style={{
        left: 'var(--sidebar-width)',
        width: 'calc(100% - var(--sidebar-width))',
      }}>
      <div className="flex-1 text-xs text-muted-foreground mt-2">
        {selectedRows.length} of {volumes.length} volume(s) selected.
      </div>
      <div className="mt-1">
        <ActionBarButton
          onClick={handleInspectClick}
          disabled={!actions.canInspect}
          icon={SearchCode}
          label="Inspect"
          className="rounded-l-lg"
          ariaLabel="Inspect selected volume"
        />
        <ActionBarButton
          onClick={() => setDialogData({ open: true, currentSelection: selectedRows })}
          disabled={!actions.canDelete}
          icon={Trash}
          label="Delete"
          ariaLabel="Delete selected volumes"
          className="inline-flex items-center rounded-r-md border border-border px-2 py-2 text-background bg-danger enabled:hover:bg-danger/85 enabled:hover:text-background font-medium text-xs disabled:cursor-not-allowed disabled:opacity-60"
        />
      </div>
    </div>
  );
};

type VolumeActionsState = {
  canDelete: boolean;
  canInspect: boolean;
};
