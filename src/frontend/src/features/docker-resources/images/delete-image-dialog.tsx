import { useState } from 'react';
import { useNavigate } from 'react-router';
import { useQueryClient } from '@tanstack/react-query';
import { Trash } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { DropdownActionButton } from '@/components/custom/dropdown-with-dialog';
import { ActionButton } from '@/components/custom/action-with-dialog';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { useAppContext } from '@/lib/context/app-context';
import { useRead, useMutate } from '@/lib/hooks';
import { notifyRequestError } from '@/lib/request-error';
import { deletionUnavailableReason, type ImageDeleteTarget } from './delete-image-action';
import { ImageUsage } from './image-usage';
import type { DropdownActionComponent } from '@/pages/types';

export const DeleteImageDropdown: DropdownActionComponent<ImageDeleteTarget> = ({ resource, onAction }) => (
  <DropdownActionButton
    title="Delete"
    icon={<Trash className="h-4 w-4" />}
    separatorBefore
    variant="destructive"
    disabled={deletionUnavailableReason(resource) !== undefined}
    onClick={() => onAction?.('deleteImage')}
  />
);

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
  const queryClient = useQueryClient();
  const [confirmation, setConfirmation] = useState('');
  const [needsReview, setNeedsReview] = useState(false);
  const [platformId] = useState(currentPlatform?.id ?? '');
  const [ids] = useState(() => [...new Set(resources.map((resource) => resource.dockerImageId))].sort());
  const formatDateTime = useProfileDateTimeFormatter();
  const selectionChanged =
    platformId !== (currentPlatform?.id ?? '') ||
    JSON.stringify(ids) !== JSON.stringify([...new Set(resources.map((resource) => resource.dockerImageId))].sort());
  const expected = detail ? resources[0].name : 'Delete';
  const preview = useRead(
    'previewImageDeletion',
    { data: { platformId, ids } },
    { retry: false, refetchOnWindowFocus: false, staleTime: 0 },
  );
  const deletion = useMutate('deleteImages');
  const reviews = preview.data?.data;
  const reason = selectionChanged
    ? 'The selection or Platform changed. Close this review and select the images again.'
    : deletionUnavailableReason(resources);
  const canDelete =
    !reason &&
    !preview.isFetching &&
    !preview.isError &&
    !deletion.isPending &&
    !needsReview &&
    reviews?.length === ids.length &&
    reviews.every((item) => item.canDelete) &&
    confirmation === expected;
  async function submit() {
    if (!canDelete || !reviews) return;
    try {
      await deletion.mutateAsync({
        data: { platformId, ids, force: false, noPrune: true, reviews: reviews.map((item) => item.review) },
      });
      await Promise.all(
        ['listImages', 'getImageUsage', 'inspectImage'].map((key) =>
          queryClient.invalidateQueries({ queryKey: [key] }),
        ),
      );
      onClose();
      if (detail) navigate(`/platforms/${platformId}/images`);
    } catch (error) {
      setNeedsReview(true);
      notifyRequestError(error);
      // A batch can remove earlier images before a later Docker conflict.
      await Promise.all(
        ['listImages', 'getImageUsage'].map((key) => queryClient.invalidateQueries({ queryKey: [key] })),
      );
    }
  }
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !deletion.isPending) onClose();
      }}>
      <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Review image deletion</DialogTitle>
          <DialogDescription>
            Remove local image content from this host. Containers, including stopped containers, and saved Local image
            dependencies prevent deletion.
          </DialogDescription>
        </DialogHeader>
        {preview.isFetching && <p role="status">Checking live image usage…</p>}
        {(preview.isError || needsReview) && (
          <p role="alert">
            The review could not be completed or is no longer valid. Refresh the review before trying again.
          </p>
        )}
        {reason && <p role="alert">{reason}</p>}
        {reviews?.map((item) => (
          <div key={item.review.dockerImageId} className="space-y-2 rounded-sm border p-3 text-sm">
            <p className="break-all font-mono">{item.review.dockerImageId}</p>
            <p>Tags: {item.review.tags.join(', ') || 'Untagged'}</p>
            <p className="text-muted-foreground">Reviewed {formatDateTime(item.observedAt)}</p>
            {item.blockers.map((blocker) => (
              <p key={blocker} role="alert" className="text-danger">
                {blocker}
              </p>
            ))}
            {item.warnings.map((warning) => (
              <p key={warning} className="text-muted-foreground">
                {warning}
              </p>
            ))}
            <ImageUsage
              key={`${item.review.dockerImageId}-${item.observedAt}`}
              imageId={item.review.dockerImageId}
              initialUsage={item.usage}
            />
          </div>
        ))}
        <Button
          variant="outline"
          disabled={preview.isFetching || deletion.isPending}
          onClick={async () => {
            setConfirmation('');
            const result = await preview.refetch();
            setNeedsReview(result.isError);
          }}>
          Refresh review
        </Button>
        <label className="text-sm">
          Type <strong>{expected}</strong> to confirm
          <Input
            aria-label="Deletion confirmation"
            value={confirmation}
            onChange={(event) => setConfirmation(event.target.value)}
          />
        </label>
        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={deletion.isPending}>
            Cancel
          </Button>
          <Button variant="destructive" disabled={!canDelete} onClick={() => void submit()}>
            Delete
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
