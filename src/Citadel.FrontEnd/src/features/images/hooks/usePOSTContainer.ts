import { useApiClientContext } from '@/api/ApiClientContext';
import { useMutation } from '@tanstack/react-query';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';

export const usePOSTContainer = () => {
  const { apiClient } = useApiClientContext();
  const { mutate, isPending, isSuccess, error, data, reset } = useMutation({
    mutationFn: apiClient?.api.createContainer,
  });
  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, isSuccess, data, validationErrors, reset };
};
