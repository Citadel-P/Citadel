import { useState } from 'react';
import { useAppContext } from '@/AppContext';
import { DeleteImagesRequest, ImageView } from '@/api/generated/api.types';
import { ConfirmDeleteDialog } from '@/components/custom/confirm-delete-dialog';
import { SwitchList } from '@/components/custom/switch-section';
import { IDialogData } from '@/lib/hooks';

export const DeleteDialog = ({
  dialogData,
  setDialogData,
  requestDelete,
  deleteIsPending,
}: {
  dialogData: IDialogData<ImageView>;
  setDialogData: (data: IDialogData<ImageView>) => void;
  requestDelete: (request: DeleteImagesRequest) => void;
  deleteIsPending: boolean;
}) => {
  const { currentPlatform } = useAppContext();
  const [force, setForce] = useState(false);
  const [noPrune, setNoPrune] = useState(false);

  const imageIds = dialogData.currentSelection?.map((c) => c.dockerImageId!) ?? [];
  const handleDelete = () => {
    requestDelete({ platformId: currentPlatform?.id ?? '', ids: imageIds, force, noPrune });
  };

  return (
    <ConfirmDeleteDialog
      open={dialogData.open}
      onOpenChange={(open) => setDialogData({ open })}
      count={imageIds.length}
      type="Image"
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
