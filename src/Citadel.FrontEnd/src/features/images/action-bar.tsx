import { useImagesContext } from './ImagesContext';
import { ActionBarButtons } from './action-bar-buttons';
import { GenericActionBar } from '@/components/custom/action-bar';

export const ActionBar = () => {
  const { setDialogData, selectedRows, localImages: images, setRunDialogData } = useImagesContext();

  if (!selectedRows?.length) return null;

  return (
    <GenericActionBar
      selectedRows={selectedRows}
      allItems={images}
      resource="Image"
      actionButtons={
        <ActionBarButtons
          selectedImages={selectedRows}
          setDialogData={setDialogData}
          setRunDialogData={setRunDialogData}
        />
      }
    />
  );
};
