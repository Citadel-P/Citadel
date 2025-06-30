import { useApiClientContext } from '@/api/ApiClientProvider';
import { useQuery } from '@tanstack/react-query';

export const useGETAllLocalImages = (platformId: string | undefined) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['getAllLocalImages'],
    queryFn: ({ signal }) => apiClient?.api.imagesGetAllLocalImages(platformId!, { signal }),
    enabled: !!platformId,
  });

  return { data, error, isLoading, isSuccess };
};
