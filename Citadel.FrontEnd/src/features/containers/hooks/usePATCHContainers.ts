import { RequestParams } from '@/api/_generated';
import { useApiClientContext } from '@/api/ApiClientProvider';
import { useMutation } from '@tanstack/react-query';

export type actionType = 'start' | 'stop' | 'pause' | 'restart' | 'delete';
interface IArgs {
  action: actionType;
  containersId: string[];
  params?: RequestParams;
}
export const usePATCHContainers = () => {
  const { apiClient } = useApiClientContext();
  const { mutate, isPending, isSuccess, data } = useMutation({
    mutationFn: ({ action, containersId: data, params }: IArgs) => {
      switch (action) {
        case 'start':
          return apiClient.api.containersStartContainers(data, params);
        case 'stop':
          return apiClient.api.containersStopContainers(data, params);
        case 'pause':
          return apiClient.api.containersPauseContainers(data, params);
        case 'restart':
          return apiClient.api.containersRestartContainers(data, params);
        case 'delete':
          return apiClient.api.containersDeleteContainers(data, params);
      }
    },
  });

  return { mutate, isPending, isSuccess, data };
};
