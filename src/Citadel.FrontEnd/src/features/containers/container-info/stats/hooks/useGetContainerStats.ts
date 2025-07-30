import { useApiClientContext } from '@/api/ApiClientContext';
import { useQuery } from '@tanstack/react-query';

export const useGetContainerStats = (containerId: string | undefined) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: [`${containerId}/stats`],
    queryFn: ({ signal }) => apiClient!.api.containersGetStats(containerId!, { signal }),
    enabled: !!containerId,
  });

  return { data, error, isLoading, isSuccess };
};
