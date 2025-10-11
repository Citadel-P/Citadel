import { useQueryClient } from '@tanstack/react-query';
import { DeleteVolumesInput, DockerVolumeResult } from '@/api/generated/api.types';
import { useDialogState } from '@/hooks/useDialogState';
import { use400ErrorToast } from '@/hooks/use400ErrorToast';
import { useCallback, useEffect } from 'react';
import { toast } from 'sonner';
import { useMutate } from '@/lib/hooks';

export const useDeleteVolumeDialog = () => {
  const client = useQueryClient();
  const {
    mutate: deleteVolumes,
    isSuccess: deleteIsSuccess,
    isPending: deleteIsPending,
    error,
  } = useMutate('deleteVolumes');
  const { dialogData, setDialogData } = useDialogState<DockerVolumeResult>();

  use400ErrorToast(error, 'The selected volume(s) could not be deleted (status code: 400).', on400ErrorHandled);

  useEffect(() => {
    if (deleteIsSuccess) {
      client.invalidateQueries({ queryKey: ['listVolumes'] });
      setDialogData({ open: false });
      toast.success('The selected volume(s) has been successfully deleted');
    }
  }, [deleteIsSuccess, client, setDialogData]);

  function on400ErrorHandled() {
    setDialogData({ open: false });
  }

  const requestDelete = useCallback(
    (request: DeleteVolumesInput) => {
      deleteVolumes(request);
    },
    [deleteVolumes],
  );

  return { setDialogData, dialogData, requestDelete, deleteIsPending, deleteIsSuccess };
};
