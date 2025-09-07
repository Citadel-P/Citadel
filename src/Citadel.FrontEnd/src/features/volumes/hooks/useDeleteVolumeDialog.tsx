import { useQueryClient } from '@tanstack/react-query';
import { useDELETEVolumes } from './useDELETEVolumes';
import { DeleteVolumesInput, DockerVolumeResult } from '@/api/_generated';
import { useDialogState } from '@/hooks/useDialogState';
import { use400ErrorToast } from '@/hooks/use400ErrorToast';
import { useCallback, useEffect } from 'react';
import { toast } from 'sonner';

export const useDeleteVolumeDialog = () => {
  const client = useQueryClient();
  const { mutate: deleteVolumes, isSuccess: deleteIsSuccess, isPending: deleteIsPending, error } = useDELETEVolumes();
  const { dialogData, setDialogData } = useDialogState<DockerVolumeResult>();

  use400ErrorToast(error, 'The selected volume(s) could not be deleted (status code: 400).', on400ErrorHandled);

  // Handle successful volume deletion
  useEffect(() => {
    if (deleteIsSuccess) {
      client.invalidateQueries({ queryKey: ['useGETVolumes'] });
      setDialogData({ open: false });
      toast.success('The selected volume(s) has been successfully deleted');
    }
  }, [deleteIsSuccess, client, setDialogData]);

  function on400ErrorHandled() {
    setDialogData({ open: false });
  }

  // Handle volume deletion request
  const requestDelete = useCallback(
    (request: DeleteVolumesInput) => {
      deleteVolumes(request);
    },
    [deleteVolumes],
  );

  return { setDialogData, dialogData, requestDelete, deleteIsPending, deleteIsSuccess };
};
