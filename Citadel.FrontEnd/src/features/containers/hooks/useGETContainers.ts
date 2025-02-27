import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGETContainers = (platformId: string) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['containers' + platformId],
    queryFn: ({ signal }) => apiClient!.api.platformsListContainers(platformId, { signal }),
  });

  return { data, error, isLoading, isSuccess };
};
