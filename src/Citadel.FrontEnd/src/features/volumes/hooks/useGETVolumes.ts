import { useApiClientContext } from '@/api/ApiClientContext';
import { useQuery } from '@tanstack/react-query';

export const useGETVolumes = (platformId: string | undefined) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['useGETVolumes'],
    queryFn: ({ signal }) => apiClient?.api.volumesList(platformId!, {}, { signal }),
    enabled: !!platformId,
  });

  return { data, error, isLoading, isSuccess };
};
