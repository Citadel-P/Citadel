import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGetRegistry = (registryId: string | undefined) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['registryId' + registryId],
    queryFn: ({ signal }) => apiClient!.api.registriesGetById(registryId!, { signal }),
    enabled: !!registryId,
  });

  return { data, error, isLoading, isSuccess };
};
