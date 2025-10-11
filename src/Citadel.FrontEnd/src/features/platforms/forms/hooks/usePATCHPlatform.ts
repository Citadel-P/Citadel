import { useApiClientContext } from '@/api/ApiClientContext';
import { useMutation } from '@tanstack/react-query';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';
import { PlatformInput } from '@/api/generated/api.types';
interface Props {
  id: string | undefined;
  data: PlatformInput;
}
export const usePATCHPlatform = () => {
  const { apiClient } = useApiClientContext();
  const { mutate, isPending, isSuccess, error, data } = useMutation({
    mutationFn: ({ id, data }: Props) => {
      return apiClient.api.updatePlatform(id!, data);
    },
  });
  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, isSuccess, data, validationErrors };
};
