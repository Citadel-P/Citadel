import { useId } from 'react';
import { MfaPolicy } from '@/api/generated/api.types';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { useRead } from '@/lib/hooks';
import { mfaPolicyDescription, mfaPolicyOptions } from '@/lib/mfa-policy';
import { useUpdateInstallationSettings } from '@/hooks/use-installation-settings';

export function MfaPolicyControl() {
  const id = useId();
  const query = useRead('getInstallationSettings');
  const settings = query.data?.data;
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
