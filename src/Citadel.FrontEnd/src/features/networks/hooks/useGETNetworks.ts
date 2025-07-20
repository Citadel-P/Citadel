import { useApiClientContext } from '@/api/ApiClientProvider';
import { useQuery } from '@tanstack/react-query';

export const useGETNetworks = (platformId: string | undefined) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['useGETNetworks'],
    queryFn: ({ signal }) => apiClient?.api.networksList(platformId!, {}, { signal }),
    enabled: !!platformId,
  });

  return { data, error, isLoading, isSuccess };
};
