import { ImageView } from '@/api/generated/api.types';
import { useDialogState } from '@/lib/hooks';

export const useRunImageDialog = () => {
  const { dialogData: runDialogData, setDialogData: setRunDialogData } = useDialogState<ImageView>();

  return { runDialogData, setRunDialogData };
};
