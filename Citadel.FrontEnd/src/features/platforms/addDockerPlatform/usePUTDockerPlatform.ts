import { useApiClientContext } from '@/api/ApiClientProvider';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';
import { useMutation } from '@tanstack/react-query';

export const usePUTDockerPlatform = () => {
  const { apiClient } = useApiClientContext();
  const { mutate, isPending, isSuccess, error, data } = useMutation({ mutationFn: apiClient!.api.platformsPut });
  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, validationErrors, isSuccess, data };
};
