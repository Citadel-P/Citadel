import { useApiClientContext } from '@/api/ApiClientProvider';
import { useQuery } from '@tanstack/react-query';

export const useGETContainerInspect = (containerId: string | undefined) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: [`${containerId}/inspect`],
    queryFn: ({ signal }) => apiClient?.api.containersInspect(containerId!, { signal }),
    enabled: !!containerId,
  });

  return { data, error, isLoading, isSuccess };
};
