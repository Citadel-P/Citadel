import type { ResourceResponse } from '@/api/types';
import { useMutate } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';

export function useUpdateInstallationSettings() {
  const queryClient = useQueryClient();
  return useMutate('patchInstallationSettings', {
    scope: { id: 'installation-settings' },
    onSuccess: async (response, variables) => {
      await queryClient.cancelQueries({ queryKey: ['getInstallationSettings'] });
      queryClient.setQueryData(['getInstallationSettings', {}], response);
      if (variables.data.updateChecksEnabled !== undefined) {
        await queryClient.cancelQueries({ queryKey: ['getInstanceUpdates'] });
        const status = queryClient.setQueryData<ResourceResponse<'getInstanceUpdates'>>(
          ['getInstanceUpdates', {}],
          (cached) =>
            cached?.data
              ? {
                  ...cached,
                  data: {
                    ...cached.data,
                    checksEnabled: response.data.updateChecksEnabled,
                    checksAllowed: response.data.updateChecksAllowed,
                  },
                }
              : cached,
        );
        // A cancelled initial read has no full status to patch. Restart that
        // active read instead of leaving the status panel loading indefinitely.
        if (!status) void queryClient.invalidateQueries({ queryKey: ['getInstanceUpdates'] });
        if (!response.data.updateChecksEnabled) {
          await queryClient.cancelQueries({ queryKey: ['getApplicationInfo'] });
          queryClient.setQueryData<ResourceResponse<'getApplicationInfo'>>(['getApplicationInfo', {}], (cached) =>
            cached?.data ? { ...cached, data: { ...cached.data, availableUpdate: undefined } } : cached,
          );
        }
      }
      toast.success('Instance settings saved.');
    },
  });
}
