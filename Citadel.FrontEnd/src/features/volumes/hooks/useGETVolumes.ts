import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGETVolumes = (platformId: string | undefined) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['useGETVolumes'],
    queryFn: ({ signal }) => apiClient?.api.volumesList(platformId!, {}, { signal }),
    enabled: !!platformId,
  });

  return { data, error, isLoading, isSuccess };
};
