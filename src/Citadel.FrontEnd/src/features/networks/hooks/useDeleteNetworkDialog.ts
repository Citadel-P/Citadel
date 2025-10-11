import { use400ErrorToast } from '@/hooks/use400ErrorToast';
import { useCallback, useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { DeleteNetworksInput, DockerNetworkResult } from '@/api/generated/api.types';
import { useDialogState } from '@/hooks/useDialogState';
import { useMutate } from '@/lib/hooks';

export const useDeleteNetworkDialog = () => {
  const client = useQueryClient();
  const {
    mutate: deleteNetworks,
    isSuccess: deleteIsSuccess,
    isPending: deleteIsPending,
    error,
  } = useMutate('deleteNetworks');
  const { dialogData, setDialogData } = useDialogState<DockerNetworkResult>();

  use400ErrorToast(error, 'The selected network(s) could not be deleted (status code: 400).', on400ErrorHandled);

  // Handle successful network deletion
  useEffect(() => {
    if (deleteIsSuccess) {
      client.invalidateQueries({ queryKey: ['listNetworks'] });
      setDialogData({ open: false });
      toast.success('The selected network(s) has been successfully deleted');
    }
  }, [deleteIsSuccess, client, setDialogData]);

  function on400ErrorHandled() {
    setDialogData({ open: false });
  }

  // Handle network deletion request
  const requestDelete = useCallback(
    (request: DeleteNetworksInput) => {
      deleteNetworks(request);
    },
    [deleteNetworks],
  );

  return { setDialogData, dialogData, requestDelete, deleteIsPending, deleteIsSuccess };
};
