import { Play, SearchCode, Trash } from 'lucide-react';
import { ActionBarButton } from '@/components/ui/ActionBarButton';
import { ImageView } from '@/api/_generated';
import { IDialogData } from '@/hooks/useDialogState';
import { useAvailableActions } from './hooks/useAvailableActions';

export const ImageActionButtons = ({
  selectedImages,
  setDialogData,
  setRunDialogData,
}: {
  selectedImages: ImageView[] | undefined;
  setDialogData: (_: IDialogData<ImageView>) => void;
  setRunDialogData: (data: IDialogData<ImageView>) => void;
}) => {
  const { actions } = useAvailableActions(selectedImages);

  return (
    <div className="mt-1">
      <ActionBarButton
        onClick={() => setRunDialogData({ open: true, currentSelection: selectedImages })}
        disabled={!actions.canRun}
        icon={Play}
        label="Run"
        className="rounded-l-lg"
        ariaLabel="Run selected image"
      />
      <ActionBarButton
        onClick={() => {}}
        disabled={!actions.canInspect}
        icon={SearchCode}
        label="Inspect"
        ariaLabel="Inspect selected image"
      />
      <ActionBarButton
        onClick={() => setDialogData({ open: true, currentSelection: selectedImages })}
        disabled={!actions.canDelete}
        icon={Trash}
        label="Delete"
        ariaLabel="Delete selected images"
        className="inline-flex items-center rounded-r-md border border-border px-2 py-2 text-background bg-danger enabled:hover:bg-danger/85 enabled:hover:text-background font-medium text-xs disabled:cursor-not-allowed disabled:opacity-60"
      />
    </div>
  );
};
