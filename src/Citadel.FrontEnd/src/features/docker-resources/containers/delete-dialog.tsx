import { useState } from 'react';
import { ContainerView } from '@/api/generated/api.types';
import { ConfirmDeleteDialog } from '@/components/custom/confirm-delete-dialog';
import { SwitchList } from '@/components/custom/switch-section';
import { useDeleteDialog } from '@/lib/hooks';

export const DeleteDialog = () => {
  const type = 'Container';
  const { open, targets, closeDialog, requestDelete, deleteIsPending } = useDeleteDialog<ContainerView>({ type });

  const [volume, toggleVolume] = useState(true);
  const [force, toggleForce] = useState(true);

  const containerIds = targets.map((c) => c.containerId!) ?? [];
  const handleDelete = () => requestDelete({ containerIds, v: volume, force });

  return (
    <ConfirmDeleteDialog
      open={open}
      count={containerIds.length}
      type={type}
      isPending={deleteIsPending}
      onOpenChange={closeDialog}
      onConfirm={handleDelete}>
      <SwitchList
        switches={[
          {
            id: 'volume',
            label: 'Volume',
            description: 'Remove anonymous volumes associated with the container(s).',
            checked: volume,
            onToggle: () => toggleVolume((v) => !v),
          },
          {
            id: 'force',
            label: 'Force',
            description: 'If the container(s) is running, kill it before removing it.',
            checked: force,
            onToggle: () => toggleForce((v) => !v),
          },
        ]}
      />
    </ConfirmDeleteDialog>
  );
};
