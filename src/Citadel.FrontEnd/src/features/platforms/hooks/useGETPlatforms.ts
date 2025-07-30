import { useApiClientContext } from '@/api/ApiClientContext';
import { useQuery } from '@tanstack/react-query';

export const useGETPlatforms = () => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess, refetch } = useQuery({
    queryKey: ['useGETPlatforms'],
    queryFn: ({ signal }) => apiClient!.api.platformsList({ signal }),
  });

  return { data, error, isLoading, isSuccess, refetch };
};
