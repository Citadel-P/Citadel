import { usePlatformsContext } from './PlatformsContext';
import { ConfirmDeleteDialog } from '@/components/custom/confirm-delete-dialog';

export const DeleteDialog = () => {
  const { dialogData, setDialogData, deleteIsPending, requestDelete } = usePlatformsContext();
  const currentPlatform = dialogData.currentSelection?.at(0);

  return (
    <ConfirmDeleteDialog
      open={dialogData.open}
      count={dialogData.currentSelection?.length ?? 0}
      type="Platform"
      isPending={deleteIsPending}
      onOpenChange={(open) => setDialogData({ open })}
      onConfirm={() => requestDelete({ ids: [currentPlatform?.id ?? ''] })}
    />
  );
};
