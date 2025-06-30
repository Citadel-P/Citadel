import { useApiClientContext } from '@/api/ApiClientProvider';
import { useMutation } from '@tanstack/react-query';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';

export const usePOSTRegistry = () => {
  const { apiClient } = useApiClientContext();
  const { mutate, isPending, isSuccess, error, data } = useMutation({ mutationFn: apiClient?.api.registriesCreate });
  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, isSuccess, data, validationErrors };
};
