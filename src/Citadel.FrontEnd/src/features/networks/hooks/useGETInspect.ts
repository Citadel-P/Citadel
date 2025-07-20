import { useApiClientContext } from '@/api/ApiClientProvider';
import { useQuery } from '@tanstack/react-query';

export const useGETInspect = (platformId: string | null, networkId: string | null) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['useGETInspectNetwork', platformId, networkId],
    queryFn: ({ signal }) => apiClient?.api.networksInspect(platformId!, networkId!, { signal }),
    enabled: !!platformId && !!networkId,
  });

  return { data, error, isLoading, isSuccess };
};
