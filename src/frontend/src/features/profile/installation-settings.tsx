import { useId } from 'react';
import type { ResourceResponse } from '@/api/types';
import { MfaPolicy } from '@/api/generated/api.types';
import { FieldSwitch } from '@/components/custom/form-builder';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { useMutate, useRead } from '@/lib/hooks';
import { mfaPolicyDescription, mfaPolicyOptions } from '@/lib/mfa-policy';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';

function useUpdateInstallationSettings() {
  const queryClient = useQueryClient();
  return useMutate('patchInstallationSettings', {
    scope: { id: 'installation-settings' },
    onSuccess: async (response, variables) => {
      await queryClient.cancelQueries({ queryKey: ['getInstallationSettings'] });
      queryClient.setQueryData(['getInstallationSettings', {}], response);
      if (variables.data.updateChecksEnabled !== undefined) {
        if (!response.data.updateChecksEnabled) {
          await queryClient.cancelQueries({ queryKey: ['getApplicationInfo'] });
          queryClient.setQueryData<ResourceResponse<'getApplicationInfo'>>(['getApplicationInfo', {}], (cached) =>
            cached?.data ? { ...cached, data: { ...cached.data, availableUpdate: undefined } } : cached,
          );
        } else {
          void queryClient.invalidateQueries({ queryKey: ['getApplicationInfo'] });
        }
      }
      if (variables.data.mfaPolicy !== undefined) {
        await queryClient.invalidateQueries({ queryKey: ['getProfileMfaStatus'] });
      }
      toast.success('Instance settings saved.');
    },
  });
}

export function InstallationSettingsControls() {
  const id = useId();
  const query = useRead('getInstallationSettings');
  const settings = query.data?.data;
  const updateChecks = useUpdateInstallationSettings();
  const updateMfa = useUpdateInstallationSettings();
  if (!settings)
    return (
      <div className="space-y-2 text-sm text-muted-foreground" role="status">
        {query.isError ? (
          <>
            Could not load instance settings.{' '}
            <Button type="button" variant="outline" size="sm" onClick={() => void query.refetch()}>
              Retry
            </Button>
          </>
        ) : (
          'Loading instance settings…'
        )}
      </div>
    );
  return (
    <div className="w-full space-y-5">
      <div className="space-y-2">
        <div className="min-w-0 space-y-1">
          <label htmlFor={`${id}-updates`} className="text-sm font-medium">
            Check for Citadel updates
          </label>
          <p id={`${id}-updates-description`} className="text-xs leading-relaxed text-muted-foreground">
            {settings.updateChecksAllowed
              ? 'Checks GitHub for new releases. No telemetry or automatic installation.'
              : 'Disabled by server configuration.'}
          </p>
        </div>
        <FieldSwitch
          id={`${id}-updates`}
          checked={settings.updateChecksEnabled}
          disabled={updateChecks.isPending || !settings.updateChecksAllowed}
          aria-label="Check for Citadel updates"
          aria-describedby={`${id}-updates-description`}
          onChange={(checked) => updateChecks.mutate({ data: { updateChecksEnabled: checked } })}
        />
        {updateChecks.isError && (
          <p role="alert" className="text-sm text-destructive">
            Update-check setting could not be saved. Try again.
          </p>
        )}
      </div>
      <div className="space-y-2">
        <label htmlFor={`${id}-mfa`} className="text-sm font-medium">
          Two-factor authentication policy
        </label>
        <Select
          value={settings.mfaPolicy}
          onValueChange={(value) => updateMfa.mutate({ data: { mfaPolicy: value as MfaPolicy } })}
          disabled={updateMfa.isPending || settings.minimumMfaPolicy === MfaPolicy.RequiredForAllUsers}>
          <SelectTrigger id={`${id}-mfa`} className="w-full max-w-100" aria-describedby={`${id}-mfa-description`}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {mfaPolicyOptions(settings.minimumMfaPolicy).map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p id={`${id}-mfa-description`} className="text-xs leading-relaxed text-muted-foreground">
          {mfaPolicyDescription(settings.mfaPolicy)}
        </p>
        {settings.minimumMfaPolicy !== MfaPolicy.Optional && (
          <p className="text-xs leading-relaxed text-muted-foreground">
            The minimum policy is set by server configuration.
          </p>
        )}
        {updateMfa.isError && (
          <p role="alert" className="text-sm text-destructive">
            MFA policy could not be saved. Try again.
          </p>
        )}
      </div>
    </div>
  );
}
