import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGETContainerInspect = (containerId: string | undefined) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['containerId' + containerId + '/inspect'],
    queryFn: ({ signal }) => apiClient?.api.containersInspect(containerId!, { signal }),
    enabled: !!containerId,
  });

  return { data, error, isLoading, isSuccess };
};
