import { useApiClientContext } from '@/api/ApiClientContext';
import { useQuery } from '@tanstack/react-query';

export const useGETContainers = (platformId: string) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['containers' + platformId],
    queryFn: ({ signal }) => apiClient!.api.platformsListContainers(platformId, { signal }),
  });

  return { data, error, isLoading, isSuccess };
};
