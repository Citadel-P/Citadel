import { useNetworksContext } from './NetworksContext';
import { useLayoutContext } from '@/layout/LayoutContext';
import { NetworkActionButtons } from './NetworkActionButtons';

export const NetworksActionBar = () => {
  const { setDialogData, selectedRows, networks } = useNetworksContext();
  const { sidebarMinimized } = useLayoutContext();

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
      <NetworkActionButtons selectedNetworks={selectedRows} setDialogData={setDialogData} />
    </div>
  );
};
