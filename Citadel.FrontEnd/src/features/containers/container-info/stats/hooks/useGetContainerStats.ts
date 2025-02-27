import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGetContainerStats = (containerId: string | undefined) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['containerId' + containerId + '/stats'],
    queryFn: ({ signal }) => apiClient!.api.containersGetStats(containerId!, { signal }),
    enabled: !!containerId,
  });

  return { data, error, isLoading, isSuccess };
};
