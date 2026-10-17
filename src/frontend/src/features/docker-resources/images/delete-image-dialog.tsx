import { useState } from 'react';
import { useNavigate } from 'react-router';
import { useQueryClient } from '@tanstack/react-query';
import { Trash } from 'lucide-react';
import { DropdownActionButton } from '@/components/custom/dropdown-with-dialog';
import { ActionButton, ActionWithDialog } from '@/components/custom/action-with-dialog';
import { useAppContext } from '@/lib/context/app-context';
import { useMutate } from '@/lib/hooks';
import { requestErrorMessage } from '@/lib/request-error';
import { deletionUnavailableReason, type ImageDeleteTarget } from './delete-image-action';
import type { DropdownActionComponent } from '@/pages/types';

function useImageDeletion(platformId: string, ids: string[]) {
  const queryClient = useQueryClient();
  const deletion = useMutate('deleteImages');
  async function run() {
    try {
      await deletion.mutateAsync({ data: { platformId, ids, force: false, noPrune: true } });
      await Promise.all(
        ['listImages', 'getImageUsage', 'inspectImage'].map((key) =>
          queryClient.invalidateQueries({ queryKey: [key] }),
        ),
      );
    } catch (error) {
      // A batch can remove earlier images before a later Docker conflict.
      await Promise.all(
        ['listImages', 'getImageUsage'].map((key) => queryClient.invalidateQueries({ queryKey: [key] })),
      );
      throw error;
    }
  }
  return { deletion, run };
}

export const DeleteImageDropdown: DropdownActionComponent<ImageDeleteTarget> = ({ resource, onAction }) => {
  const { currentPlatform } = useAppContext();
  const { deletion, run } = useImageDeletion(currentPlatform?.id ?? '', [resource.dockerImageId]);
  const disabled = deletionUnavailableReason(resource) !== undefined || deletion.isPending;
  return (
    <DropdownActionButton
      title="Delete"
      icon={<Trash className="h-4 w-4" />}
      separatorBefore
      variant="destructive"
      disabled={disabled}
      onClick={() =>
        onAction?.('confirm', {
          name: resource.name || resource.dockerImageId,
          title: 'Delete',
          icon: <Trash className="h-4 w-4" />,
          variant: 'destructive',
          disabled,
          onClick: run,
        })
      }
    />
  );
};

export function DeleteImageButton({ resources, detail = false }: { resources: ImageDeleteTarget[]; detail?: boolean }) {
  const [open, setOpen] = useState(false);
  const reason = deletionUnavailableReason(resources);
  return (
    <>
      <ActionButton
        title="Delete"
        icon={<Trash className="h-4 w-4" />}
        iconPosition="left"
        variant="destructive"
        disabled={!!reason}
        disabledReason={reason}
        onClick={() => setOpen(true)}
      />
      {open && <DeleteImageDialog resources={resources} detail={detail} onClose={() => setOpen(false)} />}
    </>
  );
}

export function DeleteImageDialog({
  resources,
  detail = false,
  onClose,
}: {
  resources: ImageDeleteTarget[];
  detail?: boolean;
  onClose: () => void;
}) {
  const { currentPlatform } = useAppContext();
  const navigate = useNavigate();
  const [platformId] = useState(currentPlatform?.id ?? '');
  const [ids] = useState(() => [...new Set(resources.map((resource) => resource.dockerImageId))].sort());
  const selectionChanged =
    platformId !== (currentPlatform?.id ?? '') ||
    JSON.stringify(ids) !== JSON.stringify([...new Set(resources.map((resource) => resource.dockerImageId))].sort());
  const expected = detail ? resources[0].name || ids[0] : 'Delete';
  const { deletion, run } = useImageDeletion(platformId, ids);
  const reason = selectionChanged
    ? 'The selection or Platform changed. Close this dialog and select the images again.'
    : deletionUnavailableReason(resources);
  const canDelete = !reason && !deletion.isPending;
  async function submit() {
    if (!canDelete) return;
    await run();
    if (detail) navigate(`/platforms/${platformId}/images`);
  }
  return (
    <ActionWithDialog
      open
      renderTrigger={false}
      name={expected}
      title="Delete"
      icon={<Trash className="h-4 w-4" />}
      variant="destructive"
      disabled={!canDelete}
      onClick={submit}
      onOpenChange={(open) => {
        if (!open && !deletion.isPending) onClose();
      }}
      items={
        detail
          ? undefined
          : ids.map((id) => ({
              id,
              name: resources.find((resource) => resource.dockerImageId === id)?.name || id,
            }))
      }
      additional={
        <>
          {reason && <p role="alert">{reason}</p>}
          {deletion.isError && <p role="alert">{requestErrorMessage(deletion.error)}</p>}
        </>
      }
    />
  );
}
