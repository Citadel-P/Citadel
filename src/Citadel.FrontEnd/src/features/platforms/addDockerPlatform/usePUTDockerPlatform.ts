import { useApiClientContext } from '@/api/api-client-context';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';
import { useMutation } from '@tanstack/react-query';

export const usePUTDockerPlatform = () => {
  const { apiClient } = useApiClientContext();
  const { mutate, isPending, isSuccess, error, data } = useMutation({ mutationFn: apiClient!.api.updatePlatform });
  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, validationErrors, isSuccess, data };
};
