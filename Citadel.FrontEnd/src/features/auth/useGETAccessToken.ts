import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export function useGETAccessToken() {
  const apiClient = useContextSelector(ApiClientContext, (s) => s?.apiClient);
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['getAccessToken'],
    queryFn: ({ signal }) => apiClient?.api.authenticationRefreshToken({ signal }),
  });

  return { data, error, isLoading, isSuccess };
}
