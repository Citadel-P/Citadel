import { useApiClientContext } from '@/api/ApiClientProvider';
import { useMutation } from '@tanstack/react-query';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';
import { PlatformInput } from '@/api/_generated';
interface Props {
  id: string | undefined;
  data: PlatformInput;
}
export const usePATCHPlatform = () => {
  const { apiClient } = useApiClientContext();
  const { mutate, isPending, isSuccess, error, data } = useMutation({
    mutationFn: ({ id, data }: Props) => {
      return apiClient.api.platformsPatch(id!, data);
    },
  });
  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, isSuccess, data, validationErrors };
};
