import { ContainerView, DeleteContainersRequest } from '@/api/generated/api.types';
import { useCallback, useEffect } from 'react';
import { toast } from 'sonner';
import { useDELETEContainers } from '../hooks/useDELETEContainers';
import { useDialogState } from '@/hooks/useDialogState';
import { DockerContainerView } from '@/api/models';

export const useDeleteContainerDialog = () => {
  const { dialogData, setDialogData } = useDialogState<ContainerView | DockerContainerView>();
  const { mutate, isSuccess: deleteIsSuccess, isPending: deleteIsPending } = useDELETEContainers();

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
