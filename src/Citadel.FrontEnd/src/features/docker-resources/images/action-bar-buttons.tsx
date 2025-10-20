import { Play, SearchCode, Trash } from 'lucide-react';
import { useAvailableActions } from './hooks/useAvailableActions';
import { useNavigate } from 'react-router';
import { formatId } from '@/lib/utils';
import { ActionButtonConfig, ActionButtons } from '@/components/custom/action-bar';
import { ImageView } from '@/api/generated/api.types';

export const ActionBarButtons = ({
  selectedRows,
  openDialog,
  showInspectButton = true,
}: {
  selectedRows: ImageView[] | undefined;
  openDialog: (targets: ImageView[]) => void;
  showInspectButton?: boolean;
}) => {
  const navigate = useNavigate();
  const { actions } = useAvailableActions(selectedRows);
  const imageId = formatId(selectedRows?.at(0)?.dockerImageId);

  const buttons: ActionButtonConfig[] = [
    {
      id: 'run',
      icon: Play,
      label: 'Run',
      onClick: () => null,
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
      onClick: () => openDialog(selectedRows ?? []),
      disabled: !actions.canDelete,
      ariaLabel: 'Delete selected images',
      variant: 'danger' as const,
    },
  ];

  return <ActionButtons buttons={buttons} />;
};
