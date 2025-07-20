import { useApiClientContext } from '@/api/ApiClientProvider';
import { useQuery } from '@tanstack/react-query';

export const useGETContainer = (containerId: string | undefined) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: [`${containerId}/getContainer`],
    queryFn: ({ signal }) => apiClient!.api.containersGetById(containerId!, { signal }),
    enabled: !!containerId,
  });

  return { data, error, isLoading, isSuccess };
};
