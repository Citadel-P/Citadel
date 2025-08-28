import { RequestParams } from '@/api/_generated';
import { useApiClientContext } from '@/api/ApiClientContext';
import { use400ErrorToast } from '@/hooks/use400ErrorToast';
import { useMutation } from '@tanstack/react-query';

export type actionType = 'start' | 'stop' | 'pause' | 'restart';
interface IArgs {
  action: actionType;
  containersId: string[];
  params?: RequestParams;
}
export const usePATCHContainers = () => {
  const { apiClient } = useApiClientContext();
  const { mutate, isPending, isSuccess, data, error } = useMutation({
    mutationFn: ({ action, containersId: data, params }: IArgs) => {
      switch (action) {
        case 'start':
          return apiClient!.api.containersStartContainers(data, params);
        case 'stop':
          return apiClient!.api.containersStopContainers(data, params);
        case 'pause':
          return apiClient!.api.containersPauseContainers(data, params);
        case 'restart':
          return apiClient!.api.containersRestartContainers(data, params);
        default:
          throw new Error(`Unsupported action: ${action}`);
      }
    },
  });

  use400ErrorToast(error);

  return { mutate, isPending, isSuccess, data };
};
