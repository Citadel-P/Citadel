import { useApiClientContext } from '@/api/ApiClientProvider';
import { useQuery } from '@tanstack/react-query';

export function useGETAccessToken() {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['getAccessToken'],
    queryFn: ({ signal }) => apiClient.api.authenticationRefreshToken({ signal }),
  });

  return { data, error, isLoading, isSuccess };
}
