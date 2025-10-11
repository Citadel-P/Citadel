import { useApiClientContext } from '@/api/ApiClientContext';
import { useMutation } from '@tanstack/react-query';
import { use400ErrorToast } from '@/hooks/use400ErrorToast';

export const useDELETEContainers = () => {
  const { apiClient } = useApiClientContext();
  const { mutate, isPending, isSuccess, error, data } = useMutation({
    mutationFn: apiClient?.api.deleteContainers,
  });
  use400ErrorToast(error, 'The selected container(s) could not be deleted (status code: 400).');

  return { mutate, isPending, isSuccess, data };
};
