import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useMutation } from '@tanstack/react-query';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';

export const usePOSTLogin = () => {
  const apiClient = useContextSelector(ApiClientContext, (s) => s?.apiClient);
  const { mutate, isPending, isSuccess, error, data } = useMutation({ mutationFn: apiClient?.api.authenticationLogin });
  const validationErrors = useGetValidationErrors(error);

  if (isSuccess && data?.data.accessToken) {
    // Reload the entire app
    window.location.href = '/';
  }

  return { mutate, isPending, validationErrors };
};
