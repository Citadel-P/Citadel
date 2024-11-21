import { StreamLogsRequest } from '@/api/_generated';
import { useApiClientContext } from '@/api/ApiClientProvider';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';
import { useMutation } from '@tanstack/react-query';

interface IParams {
  requestParams: StreamLogsRequest;
}

export const usePOSTContainerLogs = () => {
  const { apiClient } = useApiClientContext();
  const { mutate, isPending, isSuccess, error, data } = useMutation({
    mutationFn: ({ requestParams }: IParams) => apiClient.api.containersStreamLogs(requestParams),
  });

  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, validationErrors, isSuccess, data };
};
