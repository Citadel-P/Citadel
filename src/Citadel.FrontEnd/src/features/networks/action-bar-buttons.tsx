import { DockerNetworkResult } from '@/api/generated/api.types';
import { ActionButtonConfig, ActionButtons } from '@/components/custom/action-bar';
import { IDialogData } from '@/lib/hooks';
import { formatId } from '@/lib/utils';
import { SearchCode, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';

export const ActionBarButtons = ({
  selectedRows,
  setDialogData,
  showInspectButton = true,
}: {
  selectedRows: DockerNetworkResult[] | undefined;
  setDialogData: (data: IDialogData<DockerNetworkResult>) => void;
  showInspectButton?: boolean;
}) => {
  const navigate = useNavigate();
  const networkId = formatId(selectedRows?.at(0)?.id);

  const canDelete = (selectedRows?.length ?? 0) > 0 && selectedRows?.find((row) => row.inUse) === undefined;
  const canInspect = selectedRows?.length === 1;

  const buttons: ActionButtonConfig[] = [
    ...(showInspectButton
      ? [
          {
            id: 'inspect',
            icon: SearchCode,
            label: 'Inspect',
            onClick: () => navigate(`${networkId}`),
            disabled: !canInspect,
            ariaLabel: 'Inspect selected network',
          },
        ]
      : []),
    {
      id: 'delete',
      icon: Trash,
      label: 'Delete',
      onClick: () => setDialogData({ open: true, currentSelection: selectedRows }),
      disabled: !canDelete,
      ariaLabel: 'Delete selected networks',
      variant: 'danger',
    },
  ];

  return <ActionButtons buttons={buttons} />;
};
