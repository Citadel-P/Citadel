import { useApiClientContext } from '@/api/ApiClientContext';
import { useQuery } from '@tanstack/react-query';

export const useGETInspect = (platformId: string | null, imageId: string | null) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['useGETInspectImage', platformId, imageId],
    queryFn: ({ signal }) => apiClient?.api.imagesInspect(platformId!, imageId!, { signal }),
    enabled: !!platformId && !!imageId,
  });

  return { data, error, isLoading, isSuccess };
};
