import { DockerVolumeResult } from '@/api/generated/api.types';
import { ActionButtonConfig, ActionButtons } from '@/components/custom/action-bar';
import { IDialogData } from '@/lib/hooks';
import { formatId } from '@/lib/utils';
import { SearchCode, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';

export const VolumeActionBarButtons = ({
  selectedRows,
  setDialogData,
  showInspectButton,
}: {
  selectedRows: DockerVolumeResult[] | undefined;
  setDialogData: (data: IDialogData<DockerVolumeResult>) => void;
  showInspectButton?: boolean;
}) => {
  const navigate = useNavigate();
  const volumeId = formatId(selectedRows?.at(0)?.id);

  const canDelete = (selectedRows?.length ?? 0) > 0 && selectedRows?.find((row) => row.inUse) === undefined;
  const canInspect = selectedRows?.length === 1;

  const buttons: ActionButtonConfig[] = [
    ...(showInspectButton
      ? [
          {
            id: 'inspect',
            icon: SearchCode,
            label: 'Inspect',
            onClick: () => navigate(`${volumeId}`),
            disabled: !canInspect,
            ariaLabel: 'Inspect selected volume',
          },
        ]
      : []),
    {
      id: 'delete',
      icon: Trash,
      label: 'Delete',
      onClick: () => setDialogData({ open: true, currentSelection: selectedRows }),
      disabled: !canDelete,
      ariaLabel: 'Delete selected volumes',
      variant: 'danger' as const,
    },
  ];

  return <ActionButtons buttons={buttons} />;
};
