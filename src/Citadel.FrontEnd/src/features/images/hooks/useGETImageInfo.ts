import { useApiClientContext } from '@/api/ApiClientContext';
import { useQuery } from '@tanstack/react-query';

export const useGETImageInfo = (platformId: string | undefined, imageId: string | undefined) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['useGETInspectImage', platformId, imageId],
    queryFn: ({ signal }) => apiClient?.api.imagesGetImageInfo(platformId!, imageId!, { signal }),
    enabled: !!platformId && !!imageId,
  });

  return { data, error, isLoading, isSuccess };
};
