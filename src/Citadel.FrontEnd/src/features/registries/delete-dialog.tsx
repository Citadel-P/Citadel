import { useRegistriesContext } from './RegistriesContext';
import { ConfirmDeleteDialog } from '@/components/custom/confirm-delete-dialog';

export const DeleteDialog = () => {
  const { requestDelete, dialogData, setDialogData, deleteIsPending } = useRegistriesContext();
  const ids = dialogData.currentSelection?.map((r) => r.id!) ?? [];

  return (
    <ConfirmDeleteDialog
      open={dialogData.open}
      count={ids.length}
      type="Registry"
      isPending={deleteIsPending}
      onOpenChange={(open) => setDialogData({ open })}
      onConfirm={() => requestDelete({ids})}
    />
  );
};
