import { useAppContext } from '@/lib/context/app-context';
import { useState } from 'react';
import { DeleteVolumesInput, DockerVolumeResult } from '@/api/generated/api.types';
import { ConfirmDeleteDialog } from '@/components/custom/confirm-delete-dialog';
import { SwitchList } from '@/components/custom/switch-section';
import { useDeleteDialog } from '@/lib/hooks';

export const DeleteDialog = () => {
  const type = 'Volume';
  const { open, targets, closeDialog, requestDelete, deleteIsPending } = useDeleteDialog<DockerVolumeResult>({ type });
  const { currentPlatform } = useAppContext();
  const [force, setForce] = useState(false);

  const volumeIds = targets.map((c) => c.id!);

  const handleDelete = () => {
    requestDelete({
      platformId: currentPlatform?.id ?? '',
      names: volumeIds,
      force,
    } as DeleteVolumesInput);
  };

  return (
    <ConfirmDeleteDialog
      open={open}
      onOpenChange={closeDialog}
      count={volumeIds.length}
      type={type}
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
