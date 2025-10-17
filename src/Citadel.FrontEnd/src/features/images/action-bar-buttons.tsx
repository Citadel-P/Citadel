import { Play, SearchCode, Trash } from 'lucide-react';
import { useAvailableActions } from './hooks/useAvailableActions';
import { useNavigate } from 'react-router';
import { formatId } from '@/lib/utils';
import { IDialogData } from '@/lib/hooks';
import { ActionButtonConfig, ActionButtons } from '@/components/custom/action-bar';
import { ImageView } from '@/api/generated/api.types';

export const ActionBarButtons = ({
  selectedImages,
  setDialogData,
  setRunDialogData,
  showInspectButton,
}: {
  selectedImages: ImageView[] | undefined;
  setDialogData: (data: IDialogData<ImageView>) => void;
  setRunDialogData: (data: IDialogData<ImageView>) => void;
  showInspectButton?: boolean;
}) => {
  const { actions } = useAvailableActions(selectedImages);
  const navigate = useNavigate();
  const imageId = formatId(selectedImages?.at(0)?.dockerImageId);

  const buttons: ActionButtonConfig[] = [
    {
      id: 'run',
      icon: Play,
      label: 'Run',
      onClick: () => setRunDialogData({ open: true, currentSelection: selectedImages }),
      disabled: !actions.canRun,
      ariaLabel: 'Run selected image',
    },
    ...(showInspectButton
      ? [
          {
            id: 'inspect',
            icon: SearchCode,
            label: 'Inspect',
            onClick: () => navigate(`${imageId}`),
            disabled: !actions.canInspect,
            ariaLabel: 'Inspect selected image',
          },
        ]
      : []),
    {
      id: 'delete',
      icon: Trash,
      label: 'Delete',
      onClick: () => setDialogData({ open: true, currentSelection: selectedImages }),
      disabled: !actions.canDelete,
      ariaLabel: 'Delete selected images',
      variant: 'danger' as const,
    },
  ];

  return <ActionButtons buttons={buttons} />;
};
