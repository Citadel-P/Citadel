import { Trash, SearchCode } from 'lucide-react';
import { ActionBarButton } from '@/components/ui/ActionBarButton';
import { useNetworksContext } from './NetworksContext';
import { useLayoutContext } from '@/layout/LayoutContext';

export const NetworksActionBar = () => {
  const { setDialogData, setSheetOpen, setCurrentNetwork, selectedRows, networks } = useNetworksContext();
  const { sidebarMinimized } = useLayoutContext();

  const actions: NetworkActionsState = {
    canDelete: (selectedRows?.length ?? 0) > 0 && selectedRows?.find((row) => row.inUse) === undefined,
    canInspect: selectedRows?.length === 1,
  };

  const handleInspectClick = () => {
    setSheetOpen(true);
    if (selectedRows) setCurrentNetwork(selectedRows[0]);
  };

  if (!selectedRows?.length) return null;

  return (
    <div
      className={`fixed -translate-x-5 inset-x-0 bottom-0 shadow-lg p-2 bg-background flex flex-wrap justify-center items-center gap-x-4 gap-y-2 sm:justify-between ${
        sidebarMinimized ? 'action-bar-left-collapsed' : 'action-bar-left'
      }`}
      style={{
        width: sidebarMinimized ? 'calc(100% - var(--sidebar-minimized-width))' : 'calc(100% - var(--sidebar-width))',
      }}>
      <div className="flex-1 text-xs text-muted-foreground mt-2">
        {selectedRows.length} of {networks?.length} network(s) selected.
      </div>
      <div className="mt-1">
        <ActionBarButton
          onClick={handleInspectClick}
          disabled={!actions.canInspect}
          icon={SearchCode}
          label="Inspect"
          className="rounded-l-lg"
          ariaLabel="Inspect selected network"
        />
        <ActionBarButton
          onClick={() => setDialogData({ open: true, currentSelection: selectedRows })}
          disabled={!actions.canDelete}
          icon={Trash}
          label="Delete"
          ariaLabel="Delete selected networks"
          className="inline-flex items-center rounded-r-md border border-border px-2 py-2 text-background bg-danger enabled:hover:bg-danger/85 enabled:hover:text-background font-medium text-xs disabled:cursor-not-allowed disabled:opacity-60"
        />
      </div>
    </div>
  );
};

type NetworkActionsState = {
  canDelete: boolean;
  canInspect: boolean;
};
