import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGETContainer = (containerId: string | undefined) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient!);
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['getcontainer' + containerId],
    queryFn: ({ signal }) => apiClient.api.containersGetById(containerId!, { signal }),
    enabled: !!containerId,
  });

  return { data, error, isLoading, isSuccess };
};
