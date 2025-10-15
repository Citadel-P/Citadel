import { useState } from 'react';
import { IDialogData } from '@/hooks/useDialogState';
import { ContainerView, DeleteContainersRequest } from '@/api/generated/api.types';
import { DockerContainerView } from '@/api/types';
import { ConfirmDeleteDialog } from '@/components/custom/confirm-delete-dialog';
import { SwitchList } from '@/components/custom/switch-section';

export const DeleteDialog = ({
  isPending,
  requestDelete,
  setDialogData,
  dialogData,
}: {
  dialogData: IDialogData<ContainerView | DockerContainerView>;
  isPending: boolean;
  setDialogData: (data: IDialogData<ContainerView | DockerContainerView>) => void;
  requestDelete: (request: DeleteContainersRequest) => void;
}) => {
  const [volume, toggleVolume] = useState(true);
  const [force, toggleForce] = useState(true);

  const containerIds = dialogData.currentSelection?.map((c) => c.containerId!) ?? [];
  const handleDelete = () => requestDelete({ containerIds, v: volume, force });

  return (
    <ConfirmDeleteDialog
      open={dialogData.open}
      count={containerIds.length}
      type="Container"
      isPending={isPending}
      onOpenChange={(open) => setDialogData({ open })}
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
