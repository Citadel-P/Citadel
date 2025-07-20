import { useApiClientContext } from '@/api/ApiClientProvider';
import { useQuery } from '@tanstack/react-query';

export function useGETAccessToken() {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess, refetch } = useQuery({
    queryKey: ['getAccessToken'],
    queryFn: ({ signal }) => apiClient?.api.authenticationRefreshToken({ signal }),
  });

  return { data, error, isLoading, isSuccess, refetch };
}
