import { useApiClientContext } from '@/api/ApiClientProvider';
import { useQuery } from '@tanstack/react-query';

export const useGETRegistries = () => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['getAllRegistries'],
    queryFn: ({ signal }) => apiClient!.api.registriesGetAll({ signal }),
  });

  return { data, error, isLoading, isSuccess };
};
