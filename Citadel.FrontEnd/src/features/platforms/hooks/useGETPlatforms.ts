import { useApiClientContext } from '@/api/ApiClientProvider';
import { useQuery } from '@tanstack/react-query';

export const useGETPlatforms = () => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['platforms'],
    queryFn: ({ signal }) => apiClient.api.platformsList({ signal }),
  });

  return { data, error, isLoading, isSuccess };
};
