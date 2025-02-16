import { useApiClientContext } from '@/api/ApiClientProvider';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';
import { useMutation } from '@tanstack/react-query';

export const useGETAccessToken = () => {
  //const { setAccessToken } = useAuthContext();
  const { apiClient } = useApiClientContext();
  const { mutate, isPending, error, isSuccess, data } = useMutation({
    mutationFn: apiClient.api.authenticationRefreshToken,
  });
  const validationErrors = useGetValidationErrors(error);

  if (isSuccess && data?.data.accessToken) {
    //setAccessToken(data?.data.accessToken);
  }

  return { mutate, isPending, validationErrors };
};
