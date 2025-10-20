import { useState } from 'react';
import { DeleteImagesRequest, ImageView } from '@/api/generated/api.types';
import { ConfirmDeleteDialog } from '@/components/custom/confirm-delete-dialog';
import { SwitchList } from '@/components/custom/switch-section';
import { useDeleteDialog } from '@/lib/hooks';
import { useAppContext } from '@/AppContext';

export const DeleteDialog = () => {
  const type = 'Image';
  const { open, targets, closeDialog, requestDelete, deleteIsPending } = useDeleteDialog<ImageView>({ type });
  const { currentPlatform } = useAppContext();
  const [force, setForce] = useState(false);
  const [noPrune, setNoPrune] = useState(false);

  const imageIds = targets?.map((c) => c.dockerImageId!) ?? [];
  const handleDelete = () => {
    requestDelete({ platformId: currentPlatform?.id ?? '', ids: imageIds, force, noPrune } as DeleteImagesRequest);
  };

  return (
    <ConfirmDeleteDialog
      open={open}
      onOpenChange={closeDialog}
      count={imageIds.length}
      type={type}
      isPending={deleteIsPending}
      onConfirm={handleDelete}>
      <SwitchList
        switches={[
          {
            id: 'force',
            label: 'Force',
            description: 'Remove the image(s) even if it is being used by stopped containers or has other tags.',
            checked: force,
            onToggle: () => setForce((v) => !v),
          },
          {
            id: 'noprune',
            label: 'No Prune',
            description: 'Do not delete untagged parent images.',
            checked: noPrune,
            onToggle: () => setNoPrune((v) => !v),
          },
        ]}
      />
    </ConfirmDeleteDialog>
  );
};
