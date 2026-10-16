import { ResourceControlState, type ImageView } from '@/api/generated/api.types';

export type ImageDeleteTarget = Pick<
  ImageView,
  'name' | 'dockerImageId' | 'dockerNodeId' | 'capabilities' | 'isInUse' | 'isStale'
> &
  Partial<Pick<ImageView, 'controlState'>>;

export function deletionUnavailableReason(resources: ImageDeleteTarget | ImageDeleteTarget[]): string | undefined {
  const selected = Array.isArray(resources) ? resources : [resources];
  if (!selected.length) return 'Select an image to delete.';
  if (selected.some((image) => image.capabilities?.canExecute !== true)) {
    return 'You do not have permission to delete this image.';
  }
  if (selected.some((image) => image.dockerNodeId)) {
    return 'Deleting node-local images is not supported.';
  }
  if (selected.some((image) => image.isStale !== false || image.isInUse == null)) {
    return 'Refresh image inventory before deleting.';
  }
  if (selected.some((image) => image.isInUse)) {
    return 'A container references this image. Stopped containers also prevent deletion.';
  }
  if (selected.some((image) => image.controlState === ResourceControlState.Processing)) {
    return 'An image operation is already in progress.';
  }
  return undefined;
}
