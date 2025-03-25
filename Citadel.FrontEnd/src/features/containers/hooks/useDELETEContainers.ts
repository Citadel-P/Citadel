import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useMutation } from '@tanstack/react-query';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';

export const useDELETEContainers = () => {
  const apiClient = useContextSelector(ApiClientContext, (s) => s?.apiClient);
  const { mutate, isPending, isSuccess, error, data } = useMutation({
    mutationFn: apiClient?.api.containersDeleteContainers,
  });
  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, isSuccess, data, validationErrors };
};
