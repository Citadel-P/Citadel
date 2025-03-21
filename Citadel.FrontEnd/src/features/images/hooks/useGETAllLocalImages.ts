import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGETInternalImages = (platformId: string | undefined) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['getAllLocalImages'],
    queryFn: ({ signal }) => apiClient?.api.imagesGetAllLocalImages(platformId!, { signal }),
    enabled: !!platformId,
  });

  return { data, error, isLoading, isSuccess };
};
