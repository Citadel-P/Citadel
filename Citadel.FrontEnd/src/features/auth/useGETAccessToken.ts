import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';
import { usePOSTLogout } from './usePOSTLogout';

export function useGETAccessToken(enabled = true) {
  const { mutate: logout } = usePOSTLogout();
  const apiClient = useContextSelector(ApiClientContext, (s) => s?.apiClient);
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['getAccessToken'],
    queryFn: ({ signal }) => apiClient?.api.authenticationRefreshToken({ signal }),
    enabled: enabled,
  });

  if (error && error.status === 401) {
    logout({});
  }

  return { data, error, isLoading, isSuccess };
}
