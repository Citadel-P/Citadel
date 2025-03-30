import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export function useGETAccessToken(enabled = true) {
  const apiClient = useContextSelector(ApiClientContext, (s) => s?.apiClient);
  const { data, error, isLoading, isSuccess, refetch } = useQuery({
    queryKey: ['getAccessToken'],
    queryFn: ({ signal }) => apiClient?.api.authenticationRefreshToken({ signal }),
    enabled: enabled,
  });

  return { data, error, isLoading, isSuccess, refetch };
}
