import { useMemo } from 'react';
import { useContainersContext } from './ContainersContext';
import { ContainersActionButtons } from './ContainersActionButtons';
import { useLayoutContext } from '@/layout/LayoutContext';

export const ContainersActionBar = () => {
  const { containers, selectedRows, setDialogData } = useContainersContext();
  const { sidebarMinimized } = useLayoutContext();

  const selectedCount = useMemo(() => selectedRows?.length, [selectedRows]);
  const containerCount = useMemo(() => containers?.length, [containers]);

  if (!selectedCount) return null;
  return (
    <div
      className={`fixed -translate-x-5 inset-x-0 bottom-0 shadow-lg p-2 bg-background flex flex-wrap justify-center items-center gap-x-4 gap-y-2 sm:justify-between ${
        sidebarMinimized ? 'action-bar-left-collapsed' : 'action-bar-left'
      }`}
      style={{
        width: sidebarMinimized ? 'calc(100% - var(--sidebar-minimized-width))' : 'calc(100% - var(--sidebar-width))',
      }}>
      <div className="text-xs text-muted-foreground">
        {selectedCount} of {containerCount} container(s) selected.
      </div>
      <ContainersActionButtons selectedContainers={selectedRows} setDialogData={setDialogData} />
    </div>
  );
};
