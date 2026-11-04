import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { useMutate } from '@/lib/hooks';

export function useCheckInstanceUpdates() {
  const queryClient = useQueryClient();
  return useMutate('checkInstanceUpdates', {
    onSuccess: async (response) => {
      // The response is authoritative even while realtime is reconnecting.
      // Other browsers and background checks use ApplicationInfoChanged.
      await queryClient.cancelQueries({ queryKey: ['getInstanceUpdates'] });
      queryClient.setQueryData(['getInstanceUpdates', {}], response);
      const status = response.data;
      if (status.checkError) {
        toast.error('Update check failed', { description: status.checkError });
      } else if (status.available) {
        toast.success(`Citadel ${status.available.version} is available`, {
          description: 'Review the release notes before upgrading.',
        });
      } else {
        toast.success('Check complete', { description: 'Release check complete. Review instance and Agent status.' });
      }
    },
  });
}
