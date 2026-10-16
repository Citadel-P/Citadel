import { DeleteImageButton } from '../delete-image-dialog';
import type { ImageDeleteTarget } from '../delete-image-action';
import { ImageReferenceButton, type ImageReferenceTarget } from '../reference-actions';

export type ImageInfoActionTarget = ImageDeleteTarget & ImageReferenceTarget;

export const ImageInfoActions = {
  check: ({ resource }: { resource: ImageInfoActionTarget }) => (
    <ImageReferenceButton resources={[resource]} kind="check" />
  ),
  pull: ({ resource }: { resource: ImageInfoActionTarget }) => (
    <ImageReferenceButton resources={[resource]} kind="pull" />
  ),
  delete: ({ resource }: { resource: ImageDeleteTarget }) => <DeleteImageButton resources={[resource]} detail />,
};
