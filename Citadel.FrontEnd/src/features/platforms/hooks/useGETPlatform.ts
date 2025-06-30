import { useApiClientContext } from '@/api/ApiClientProvider';
import { useQuery } from '@tanstack/react-query';

export const useGETPlatform = (platformId: string | undefined) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['platformId' + platformId],
    queryFn: ({ signal }) => apiClient!.api.platformsGetById(platformId!, { signal }),
    enabled: !!platformId,
  });
  return { data, error, isLoading, isSuccess };
};
