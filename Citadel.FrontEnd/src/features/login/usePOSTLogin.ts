import { useApiClientContext } from '@/api/ApiClientProvider';
import { useMutation } from '@tanstack/react-query';
import { useAuthContext } from './AuthProvider';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';

export const usePOSTLogin = () => {
  const { setJwtToken } = useAuthContext();
  const { apiClient } = useApiClientContext();
  const { mutate, isPending, isSuccess, error, data } = useMutation({ mutationFn: apiClient.api.authenticationLogin });
  const validationErrors = useGetValidationErrors(error);

  if (isSuccess && data?.data.jwt) {
    setJwtToken(data?.data.jwt);
    // Reload the entire app
    window.location.href = '/';
  }

  return { mutate, isPending, validationErrors };
};
