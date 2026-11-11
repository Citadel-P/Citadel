import { useId } from 'react';
import { MfaPolicy } from '@/api/generated/api.types';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { mfaPolicyDescription, mfaPolicyOptions } from '@/lib/mfa-policy';

export function MfaPolicyControl({
  value,
  minimum,
  disabled,
  onChange,
}: {
  value: MfaPolicy;
  minimum: MfaPolicy;
  disabled?: boolean;
  onChange: (value: MfaPolicy) => void;
}) {
  const id = useId();
  return (
    <div className="w-full space-y-5">
      <div className="space-y-2">
        <label htmlFor={`${id}-mfa`} className="text-sm font-medium">
          Two-factor authentication policy
        </label>
        <Select
          value={value}
          onValueChange={(value) => onChange(value as MfaPolicy)}
          disabled={disabled || minimum === MfaPolicy.RequiredForAllUsers}>
          <SelectTrigger id={`${id}-mfa`} className="w-full max-w-100" aria-describedby={`${id}-mfa-description`}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {mfaPolicyOptions(minimum).map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p id={`${id}-mfa-description`} className="text-xs leading-relaxed text-muted-foreground">
          {mfaPolicyDescription(value)}
        </p>
        {minimum !== MfaPolicy.Optional && (
          <p className="text-xs leading-relaxed text-muted-foreground">
            The minimum policy is set by server configuration.
          </p>
        )}
      </div>
    </div>
  );
}
