import { useApiClientContext } from '@/api/ApiClientContext';
import { useQuery } from '@tanstack/react-query';

export const useGetRegistryWithConfig = (registryId: string | undefined) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['registryConfigId' + registryId],
    queryFn: ({ signal }) => apiClient!.api.registriesGetWithConfig(registryId!, { signal }),
    enabled: !!registryId,
  });

  return { data, error, isLoading, isSuccess };
};
