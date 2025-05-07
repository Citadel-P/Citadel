import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGETInspect = (platformId: string | null, name: string | null) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['useGETInspectVolume', platformId, name],
    queryFn: ({ signal }) => apiClient?.api.volumesInspect(platformId!, name!, { signal }),
    enabled: !!platformId && !!name,
  });

  return { data, error, isLoading, isSuccess };
};
