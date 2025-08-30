import { useApiClientContext } from '@/api/ApiClientContext';
import { use400ErrorToast } from '@/hooks/use400ErrorToast';
import { useQuery } from '@tanstack/react-query';

export const useGETInspect = (platformId: string | null, imageId: string | null) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['useGETInspectImage', platformId, imageId],
    queryFn: ({ signal }) => apiClient?.api.imagesInspect(platformId!, imageId!, { signal }),
    enabled: !!platformId && !!imageId,
  });

  use400ErrorToast(error);

  return { data, error, isLoading, isSuccess };
};
