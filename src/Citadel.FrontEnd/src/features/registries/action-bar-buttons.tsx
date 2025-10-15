import { useNavigate } from 'react-router';
import { RegistryView } from '@/api/generated/api.types';
import { Pencil, Trash } from 'lucide-react';
import { IDialogData } from '@/lib/hooks';
import { ActionButtonConfig, ActionButtons } from '@/components/custom/action-bar';

export const ActionBarButtons = ({
  selectedRows,
  setDialogData,
}: {
  selectedRows: RegistryView[] | undefined;
  setDialogData: (data: IDialogData<RegistryView>) => void;
}) => {
  const navigate = useNavigate();

  const canEdit = selectedRows?.length === 1;
  const canDelete = selectedRows ? selectedRows.length > 0 : false;

  const buttons: ActionButtonConfig[] = [
    {
      id: 'edit',
      icon: Pencil,
      label: 'Edit',
      onClick: () => navigate('/registries/edit/' + selectedRows?.at(0)?.id),
      disabled: !canEdit,
      ariaLabel: 'Edit selected registries',
    },
    {
      id: 'delete',
      icon: Trash,
      label: 'Delete',
      onClick: () => setDialogData({ open: true, currentSelection: selectedRows }),
      disabled: !canDelete,
      ariaLabel: 'Delete selected registries',
      variant: 'danger' as const,
    },
  ];

  return <ActionButtons buttons={buttons} />;
};
