import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGETNetworks = (platformId: string | undefined) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['useGETNetworks'],
    queryFn: ({ signal }) => apiClient?.api.networksList(platformId!, {}, { signal }),
    enabled: !!platformId,
  });

  return { data, error, isLoading, isSuccess };
};
