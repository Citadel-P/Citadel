import { ImageView } from '@/api/generated/api.types';
import { useDialogState } from '@/hooks/useDialogState';

export const useRunImageDialog = () => {
  const { dialogData: runDialogData, setDialogData: setRunDialogData } = useDialogState<ImageView>();

  return { runDialogData, setRunDialogData };
};
