import { useImagesContext } from './ImagesContext';
import { ImageActionBarButtons } from './action-bar-buttons';
import { ActionBar } from '@/components/custom/action-bar';

export const ImageActionBar = () => {
  const { setDialogData, selectedRows, localImages: images, setRunDialogData } = useImagesContext();

  if (!selectedRows?.length) return null;

  return (
    <ActionBar
      selectedRows={selectedRows}
      allItems={images}
      resource="Image"
      actionButtons={
        <ImageActionBarButtons
          selectedImages={selectedRows}
          setDialogData={setDialogData}
          setRunDialogData={setRunDialogData}
        />
      }
    />
  );
};
