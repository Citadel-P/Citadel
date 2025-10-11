import { DeleteImagesRequest, ImageView } from '@/api/generated/api.types';
import { useCallback, useEffect } from 'react';
import { toast } from 'sonner';
import { useDialogState } from '@/hooks/useDialogState';
import { use400ErrorToast } from '@/hooks/use400ErrorToast';
import { useQueryClient } from '@tanstack/react-query';
import { useMutate } from '@/lib/hooks';

export const useDeleteImageDialog = () => {
  const client = useQueryClient();
  const { dialogData, setDialogData } = useDialogState<ImageView>();
  const { mutate, isSuccess: deleteIsSuccess, isPending: deleteIsPending, data, error } = useMutate('deleteImages');

  use400ErrorToast(error, 'The selected image(s) could not be deleted (status code: 400).', on400ErrorHandled);

  // Handle successful image deletion
  useEffect(() => {
    if (deleteIsSuccess) {
      client.invalidateQueries({ queryKey: ['getAllLocalImages'] });
      setDialogData({ open: false });

      const message =
        data?.data?.items && data?.data?.items.length > 1
          ? 'The selected images have been successfully deleted'
          : 'The selected image has been successfully deleted';

      toast.success(message);
    }
  }, [deleteIsSuccess, client, data, setDialogData]);

  function on400ErrorHandled() {
    setDialogData({ open: false });
  }

  const requestDelete = useCallback(
    (data: DeleteImagesRequest) => {
      mutate(data);
    },
    [mutate],
  );

  return { dialogData, setDialogData, deleteIsPending, deleteIsSuccess, requestDelete };
};
