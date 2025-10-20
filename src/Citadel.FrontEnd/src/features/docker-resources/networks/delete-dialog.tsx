import { useAppContext } from '@/AppContext';
import { DeleteNetworksInput, DockerNetworkResult } from '@/api/generated/api.types';
import { ConfirmDeleteDialog } from '@/components/custom/confirm-delete-dialog';
import { useDeleteDialog } from '@/lib/hooks';

export const DeleteDialog = () => {
  const type = 'Network';
  const { open, targets, closeDialog, requestDelete, deleteIsPending } = useDeleteDialog<DockerNetworkResult>({ type });
  const { currentPlatform } = useAppContext();

  const networksId = targets.map((c) => c.id!) ?? [];

  const handleDelete = () => {
    requestDelete({ platformId: currentPlatform?.id ?? '', ids: networksId } as DeleteNetworksInput);
  };

  return (
    <ConfirmDeleteDialog
      open={open}
      onOpenChange={closeDialog}
      count={networksId.length}
      type={type}
      isPending={deleteIsPending}
      onConfirm={handleDelete}
    />
  );
};
