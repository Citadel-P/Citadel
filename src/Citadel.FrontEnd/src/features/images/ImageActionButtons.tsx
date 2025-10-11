import { Play, SearchCode, Trash } from 'lucide-react';
import { ActionBarButton } from '@/components/ui/ActionBarButton';
import { ImageView } from '@/api/generated/api.types';
import { IDialogData } from '@/hooks/useDialogState';
import { useAvailableActions } from './hooks/useAvailableActions';
import { useMemo } from 'react';
import { useNavigate } from 'react-router';
import { formatId } from '@/lib/utils';

export const ImageActionButtons = ({
  selectedImages,
  setDialogData,
  setRunDialogData,
  showInspectButton = true,
}: {
  selectedImages: ImageView[] | undefined;
  showInspectButton?: boolean;
  setDialogData: (_: IDialogData<ImageView>) => void;
  setRunDialogData: (data: IDialogData<ImageView>) => void;
}) => {
  const navigate = useNavigate();
  const { actions } = useAvailableActions(selectedImages);

  const imageId = useMemo(() => formatId(selectedImages?.at(0)?.imageId), [selectedImages]);

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
      {showInspectButton && (
        <ActionBarButton
          onClick={() => navigate(`${imageId}`)}
          disabled={!actions.canInspect}
          icon={SearchCode}
          label="Inspect"
          ariaLabel="Inspect selected image"
        />
      )}
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
