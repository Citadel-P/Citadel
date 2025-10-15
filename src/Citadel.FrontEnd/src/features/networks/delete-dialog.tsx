import { useAppContext } from '@/AppContext';
import { DeleteNetworksInput, DockerNetworkResult } from '@/api/generated/api.types';
import { ConfirmDeleteDialog } from '@/components/custom/confirm-delete-dialog';
import { IDialogData } from '@/lib/hooks';

export const DeleteDialog = ({
  dialogData,
  setDialogData,
  requestDelete,
  deleteIsPending,
}: {
  dialogData: IDialogData<DockerNetworkResult>;
  setDialogData: (data: IDialogData<DockerNetworkResult>) => void;
  requestDelete: (request: DeleteNetworksInput) => void;
  deleteIsPending: boolean;
}) => {
  const { currentPlatform } = useAppContext();

  const networkIds = dialogData.currentSelection?.map((c) => c.id!) ?? [];

  const handleDelete = () => {
    requestDelete({ platformId: currentPlatform?.id ?? '', ids: networkIds });
  };

  return (
    <ConfirmDeleteDialog
      open={dialogData.open}
      onOpenChange={(open) => setDialogData({ open })}
      count={networkIds.length}
      type="Network"
      isPending={deleteIsPending}
      onConfirm={handleDelete}
    />
  );
};
