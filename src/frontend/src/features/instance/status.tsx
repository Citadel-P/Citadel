import { useMutationState } from '@tanstack/react-query';
import type { UseMutateVariables } from '@/api/types';
import { AlertMessage } from '@/components/custom/alert-message';
import { BackToTop } from '@/components/custom/back-to-top';
import { useRead } from '@/lib/hooks';

export function InstanceStatus() {
  const settings = useRead('getInstallationSettings').data?.data;
  const saves = useMutationState({
    filters: {
      mutationKey: ['patchInstallationSettings'],
      predicate: (mutation) => {
        const variables = mutation.state.variables as UseMutateVariables<'patchInstallationSettings'> | undefined;
        return variables?.data.configuration !== undefined || variables?.data.mfaPolicy !== undefined;
      },
    },
    select: (mutation) => mutation.state,
  });
  const save = saves.at(-1);

  return (
    <>
      <BackToTop />
      {settings && (
        <div className="space-y-4">
          {!settings.restartRequired && (
            <AlertMessage type="info">
              MFA changes apply on save. Restart Core to apply other saved settings.
            </AlertMessage>
          )}
          {settings.restartRequired && (
            <AlertMessage type="warning">
              <span className='font-semibold'>Restart required</span>: saved settings differ from the values currently in use.
            </AlertMessage>
          )}
          {save?.status === 'error' && (
            <AlertMessage type="error">Could not save instance settings. {save.error?.message}</AlertMessage>
          )}
        </div>
      )}
    </>
  );
}
