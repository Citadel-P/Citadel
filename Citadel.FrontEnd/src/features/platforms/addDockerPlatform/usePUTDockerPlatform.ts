import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';
import { useMutation } from '@tanstack/react-query';

export const usePUTDockerPlatform = () => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient!);
  const { mutate, isPending, isSuccess, error, data } = useMutation({ mutationFn: apiClient.api.platformsPut });
  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, validationErrors, isSuccess, data };
};
