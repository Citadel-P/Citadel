import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGETPlatform = (platformId: string | undefined) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['platformId' + platformId],
    queryFn: ({ signal }) => apiClient!.api.platformsGetById(platformId!, { signal }),
    enabled: !!platformId,
  });

  return { data, error, isLoading, isSuccess };
};
