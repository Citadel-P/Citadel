import { ImageView } from '@/api/_generated';

export const useAvailableActions = (images: ImageView[] | undefined) => {
  const actions: ImageActionsState = {
    canRun: images?.length === 1,
    canInspect: images?.length === 1,
    canDelete: (images?.length ?? 0) > 0,
  };
  return { actions };
};
type ImageActionsState = {
  canRun: boolean;
  canInspect: boolean;
  canDelete: boolean;
};
