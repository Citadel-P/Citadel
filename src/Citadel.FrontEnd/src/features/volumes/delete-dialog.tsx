import { useAppContext } from '@/AppContext';
import { useState } from 'react';
import { DeleteVolumesInput, DockerVolumeResult } from '@/api/generated/api.types';
import { ConfirmDeleteDialog } from '@/components/custom/confirm-delete-dialog';
import { SwitchList } from '@/components/custom/switch-section';
import { IDialogData } from '@/lib/hooks';

export const DeleteDialog = ({
  dialogData,
  setDialogData,
  requestDelete,
  deleteIsPending,
}: {
  dialogData: IDialogData<DockerVolumeResult>;
  setDialogData: (data: IDialogData<DockerVolumeResult>) => void;
  requestDelete: (request: DeleteVolumesInput) => void;
  deleteIsPending: boolean;
}) => {
  const { currentPlatform } = useAppContext();
  const [force, setForce] = useState(false);

  const volumesId = dialogData.currentSelection?.map((c) => c.id!) ?? [];
  const handleDelete = () => {
    requestDelete({ platformId: currentPlatform?.id ?? '', names: volumesId, force });
  };

  return (
    <ConfirmDeleteDialog
      open={dialogData.open}
      onOpenChange={(open) => setDialogData({ open })}
      count={volumesId.length}
      type="Volume"
      isPending={deleteIsPending}
      onConfirm={handleDelete}>
      <SwitchList
        switches={[
          {
            id: 'force',
            label: 'Force',
            description: 'Force the removal of the volume.',
            checked: force,
            onToggle: () => setForce((v) => !v),
          },
        ]}
      />
    </ConfirmDeleteDialog>
  );
};
