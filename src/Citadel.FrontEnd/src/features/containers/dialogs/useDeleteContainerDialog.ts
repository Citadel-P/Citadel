import { ContainerView, DeleteContainersRequest } from '@/api/generated/api.types';
import { useCallback, useEffect } from 'react';
import { toast } from 'sonner';
import { useDialogState } from '@/hooks/useDialogState';
import { DockerContainerView } from '@/api/models';
import { useMutate } from '@/lib/hooks';
import { use400ErrorToast } from '@/hooks/use400ErrorToast';

export const useDeleteContainerDialog = () => {
  const { dialogData, setDialogData } = useDialogState<ContainerView | DockerContainerView>();
  const { mutate, isSuccess: deleteIsSuccess, isPending: deleteIsPending, error } = useMutate('deleteContainers');
  use400ErrorToast(error, 'The selected container(s) could not be deleted (status code: 400).');

  useEffect(() => {
    if (deleteIsSuccess) {
      setDialogData({ open: false });
      toast.success('The selected container(s) has been successfully deleted');
    }
  }, [deleteIsSuccess, setDialogData]);

  const requestDelete = useCallback(
    (data: DeleteContainersRequest) => {
      mutate(data);
    },
    [mutate],
  );

  return { dialogData, setDialogData, deleteIsPending, deleteIsSuccess, requestDelete };
};
