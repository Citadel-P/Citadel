import { SearchCode, Trash } from 'lucide-react';
import { useContextSelector } from 'use-context-selector';
import { ImagesContext } from './ImagesProvider';
import { ActionBarButton } from '@/components/ui/ActionBarButton';

export const ActionBar = () => {
  const setDialogData = useContextSelector(ImagesContext, (v) => v?.setDialogData)!;
  const selectedRows = useContextSelector(ImagesContext, (v) => v?.selectedRows) ?? [];
  const images = useContextSelector(ImagesContext, (v) => v?.localImages) ?? [];
  const setSheetOpen = useContextSelector(ImagesContext, (v) => v?.setSheetOpen)!;
  const setCurrentImage = useContextSelector(ImagesContext, (v) => v?.setCurrentImage)!;

  const actions: ImageActionsState = {
    canInspect: selectedRows?.length === 1,
    canDelete: selectedRows?.length > 0,
  };

  const handleInspectClick = () => {
    setSheetOpen(true);
    setCurrentImage(selectedRows[0]);
  };
  if (!selectedRows.length) return null;

  return (
    <div
      className="h-14 fixed -translate-x-5 inset-x-0 bottom-0 shadow-lg p-2 bg-background sm:flex sm:justify-between"
      style={{
        left: 'var(--sidebar-width)',
        width: 'calc(100% - var(--sidebar-width))',
      }}>
      <div className="flex-1 text-xs text-muted-foreground mt-2">
        {selectedRows.length} of {images.length} image(s) selected.
      </div>
      <div className="mt-1">
        <ActionBarButton
          onClick={handleInspectClick}
          disabled={!actions.canInspect}
          icon={SearchCode}
          label="Inspect"
          className="rounded-l-lg"
          ariaLabel="Inspect selected image"
        />
        <ActionBarButton
          onClick={() => setDialogData({ open: true, currentSelection: selectedRows })}
          disabled={!actions.canDelete}
          icon={Trash}
          label="Delete"
          ariaLabel="Delete selected images"
          className="inline-flex items-center rounded-r-md border border-border px-2 py-2 text-background bg-danger enabled:hover:bg-danger/85 enabled:hover:text-background font-medium text-xs disabled:cursor-not-allowed disabled:opacity-60"
        />
      </div>
    </div>
  );
};

type ImageActionsState = {
  canInspect: boolean;
  canDelete: boolean;
};
