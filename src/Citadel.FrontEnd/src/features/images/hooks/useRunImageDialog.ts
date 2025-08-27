import { ImageView } from '@/api/_generated';
import { useDialogState } from '@/hooks/useDialogState';

export const useRunImageDialog = () => {
  const { dialogData: runDialogData, setDialogData: setRunDialogData } = useDialogState<ImageView>();

  return { runDialogData, setRunDialogData };
};
