import { useVolumesContext } from './VolumesContext';
import { useLayoutContext } from '@/layout/LayoutContext';
import { VolumeActionButtons } from './VolumeActionButtons';

export const VolumesActionBar = () => {
  const { setDialogData, selectedRows, volumes } = useVolumesContext();
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
        {selectedRows.length} of {volumes?.length} volume(s) selected.
      </div>
      <VolumeActionButtons selectedVolumes={selectedRows} setDialogData={setDialogData} />
    </div>
  );
};
