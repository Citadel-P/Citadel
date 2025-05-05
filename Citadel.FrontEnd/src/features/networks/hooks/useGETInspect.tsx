import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGETInspect = (platformId: string | null, networkId: string | null) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['useGETInspect', platformId, networkId],
    queryFn: ({ signal }) => apiClient?.api.networksInspect(platformId!, networkId!, { signal }),
    enabled: !!platformId && !!networkId,
  });

  return { data, error, isLoading, isSuccess };
};
