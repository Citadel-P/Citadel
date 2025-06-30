import { useApiClientContext } from '@/api/ApiClientProvider';
import { useQuery } from '@tanstack/react-query';

export const useGetRegistry = (registryId: string | undefined) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['registryId' + registryId],
    queryFn: ({ signal }) => apiClient!.api.registriesGetById(registryId!, { signal }),
    enabled: !!registryId,
  });

  return { data, error, isLoading, isSuccess };
};
