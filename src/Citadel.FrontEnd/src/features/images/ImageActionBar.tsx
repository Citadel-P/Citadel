import { useImagesContext } from './ImagesContext';
import { useLayoutContext } from '@/layout/LayoutContext';
import { ImageActionButtons } from './ImageActionButtons';

export const ImageActionBar = () => {
  const { setDialogData, selectedRows, localImages: images, setRunDialogData } = useImagesContext();
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
        {selectedRows.length} of {images.length} image(s) selected.
      </div>
      <ImageActionButtons
        selectedImages={selectedRows}
        setDialogData={setDialogData}
        setRunDialogData={setRunDialogData}
      />
    </div>
  );
};
