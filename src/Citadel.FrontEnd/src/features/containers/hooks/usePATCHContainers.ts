import { RequestParams } from '@/api/generated/api.types';
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
          return apiClient!.api.startContainers(data, params);
        case 'stop':
          return apiClient!.api.stopContainers(data, params);
        case 'pause':
          return apiClient!.api.pauseContainers(data, params);
        case 'restart':
          return apiClient!.api.restartContainers(data, params);
        default:
          throw new Error(`Unsupported action: ${action}`);
      }
    },
  });

  use400ErrorToast(error);

  return { mutate, isPending, isSuccess, data };
};
