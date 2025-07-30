import { useApiClientContext } from '@/api/ApiClientContext';
import { useQuery } from '@tanstack/react-query';

export const useGETInspect = (platformId: string | null, name: string | null) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['useGETInspectVolume', platformId, name],
    queryFn: ({ signal }) => apiClient?.api.volumesInspect(platformId!, name!, { signal }),
    enabled: !!platformId && !!name,
  });

  return { data, error, isLoading, isSuccess };
};
