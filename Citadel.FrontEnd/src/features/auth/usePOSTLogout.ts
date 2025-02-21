import { useApiClientContext } from '@/api/ApiClientProvider';
import { useMutation } from '@tanstack/react-query';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';

export const usePOSTLogout = () => {
  const { apiClient } = useApiClientContext();
  const { mutate, isPending, isSuccess, error, data } = useMutation({
    mutationFn: apiClient.api.authenticationLogout,
  });
  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, data, isSuccess, validationErrors };
};
