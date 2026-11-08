import { useMemo, useState } from 'react';
import type { InstanceConfigurationView } from '@/api/generated/api.types';
import { MfaPolicy } from '@/api/generated/api.types';
import {
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldSwitch,
  FormShell,
} from '@/components/custom/form-builder';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useRead } from '@/lib/hooks';
import { MfaPolicyControl } from './mfa-policy';
import { useUpdateInstallationSettings } from '@/hooks/use-installation-settings';
import { instanceSections } from './fields';

type InstanceFormValue = InstanceConfigurationView & { mfaPolicy: MfaPolicy };

export function InstanceSettingsForm() {
  const query = useRead('getInstallationSettings');
  const settings = query.data?.data;
  const save = useUpdateInstallationSettings();
  const [update, setUpdate] = useState<Partial<InstanceFormValue>>({});
  const original = useMemo(
    () => (settings ? { ...settings.effectiveConfiguration, mfaPolicy: settings.mfaPolicy } : undefined),
    [settings],
  );
  const schema = useMemo(
    () => ({
      '': defineSection<InstanceFormValue>({
        items: instanceSections.map((section) =>
          defineGroupField<InstanceFormValue>({
            id: section.label.toLowerCase(),
            label: section.label,
            items: [
              ...(section.label === 'Security'
                ? [
                    defineField<InstanceFormValue>({
                      key: 'mfaPolicy',
                      label: 'MFA requirement',
                      labelSuffix: <Badge variant="outline">Saves on selection</Badge>,
                      description: `Applies at the next local-password sign-in; existing sessions stay signed in. OIDC MFA belongs to the provider. Agents and service accounts are excluded. Relaxing the policy keeps enrolled authenticators. Deployment minimum (CITADEL_MFA_POLICY): ${settings?.minimumMfaPolicy ?? MfaPolicy.Optional}.`,
                      ignoreFormDisabled: true,
                      render: () => <MfaPolicyControl />,
                    }),
                  ]
                : []),
              ...section.fields.map((field) => {
                const override = settings?.deploymentOverrides[field.key];
                const managed = override !== null && override !== undefined;
                const deploymentValue = field.key === 'requireDirectAgentHttps' ? !override : override;
                const saved = settings?.configuration[field.key];
                const effective = settings?.effectiveConfiguration[field.key];
                const running = settings?.runningConfiguration[field.key];
                const pending = effective !== running;
                const current = { ...original, ...update };
                return defineField<InstanceFormValue>({
                  key: field.key,
                  label: field.label,
                  labelSuffix: (
                    <span className="flex gap-2">
                      {field.advanced && <Badge variant="outline">Advanced</Badge>}
                      {pending && <Badge variant="outline">Restart pending</Badge>}
                    </span>
                  ),
                  description: (
                    <div className="space-y-1">
                      <p>{field.description}</p>
                      <p>Applies after a Core restart.</p>
                      {managed && (
                        <p>
                          Deployment managed by <code>{field.environment}</code>: {String(deploymentValue)}. Remove the
                          override and restart Core to manage this here.
                        </p>
                      )}
                      {(pending || managed) && (
                        <p>
                          Saved choice: {String(saved === '' ? 'Automatic' : saved)}. Currently in use:{' '}
                          {String(running === '' ? 'Automatic' : running)}.
                        </p>
                      )}
                    </div>
                  ),
                  validate: (value) => {
                    if (
                      field.minimum !== undefined &&
                      (!Number.isInteger(value) || value < field.minimum || value > field.maximum!)
                    )
                      return `Enter a whole number from ${field.minimum} to ${field.maximum}.`;
                    if (
                      field.key === 'serviceAccountDefaultTokenLifetimeDays' &&
                      value > current.serviceAccountMaximumTokenLifetimeDays!
                    )
                      return 'Default lifetime must not exceed the maximum.';
                    if (
                      field.key === 'serviceAccountMaximumTokenLifetimeDays' &&
                      value < current.serviceAccountDefaultTokenLifetimeDays!
                    )
                      return 'Maximum lifetime must be at least the default.';
                    if (field.key === 'scriptsDefaultTimeoutSeconds' && value > current.scriptsMaximumTimeoutSeconds!)
                      return 'Default timeout must not exceed the maximum.';
                    if (field.key === 'scriptsMaximumTimeoutSeconds' && value < current.scriptsDefaultTimeoutSeconds!)
                      return 'Maximum timeout must be at least the default.';
                    if (field.key === 'agentImageOverride' && (value.length > 512 || /\s/.test(value)))
                      return 'Enter an image reference without whitespace, or leave empty for Automatic.';
                    return null;
                  },
                  render: (value, set) => {
                    const change = (value: unknown) => set({ [field.key]: value } as Partial<InstanceFormValue>);
                    const disabled = save.isPending || managed;
                    if (typeof value === 'boolean')
                      return (
                        <FieldSwitch
                          id={field.key}
                          checked={value}
                          disabled={disabled}
                          onChange={change}
                          aria-label={field.label}
                        />
                      );
                    if (field.choices)
                      return (
                        <Select value={String(value)} disabled={disabled} onValueChange={change}>
                          <SelectTrigger aria-label={field.label}>
                            <SelectValue placeholder={String(value)} />
                          </SelectTrigger>
                          <SelectContent>
                            {(managed && !field.choices.includes(String(value))
                              ? [String(value), ...field.choices]
                              : field.choices
                            ).map((choice) => (
                              <SelectItem key={choice} value={choice}>
                                {choice}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      );
                    return (
                      <FieldInput
                        id={field.key}
                        value={value}
                        disabled={disabled}
                        type={field.minimum === undefined ? 'text' : 'number'}
                        min={field.minimum}
                        max={field.maximum}
                        step={1}
                        placeholder={field.key === 'agentImageOverride' ? 'Automatic' : undefined}
                        onChange={change}
                      />
                    );
                  },
                });
              }),
            ],
          }),
        ),
      }),
    }),
    [settings, original, update, save.isPending],
  );

  if (!settings || !original)
    return (
      <div role="status" className="space-y-3">
        {query.isError ? (
          <>
            <p>Could not load instance settings.</p>
            <Button onClick={() => void query.refetch()}>Retry</Button>
          </>
        ) : (
          'Loading instance settings…'
        )}
      </div>
    );

  return (
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">
        MFA applies on save. All other settings are saved for the next Core start. Saving does not restart Core or
        Agents.
      </p>
      {settings.restartRequired && (
        <p role="status" className="text-sm font-medium">
          Restart required: saved settings differ from the values currently in use.
        </p>
      )}
      {save.isError && (
        <p role="alert" className="text-sm text-destructive">
          Could not save instance settings. {save.error.message}
        </p>
      )}
      <FormShell
        schema={schema}
        original={original}
        update={update}
        setUpdate={setUpdate}
        pending={save.isPending}
        onSave={async (value) => {
          const { mfaPolicy, ...configuration } = update;
          await save.mutateAsync({
            data: {
              ...(mfaPolicy !== undefined ? { mfaPolicy: value.mfaPolicy } : {}),
              ...(Object.keys(configuration).length ? { configuration } : {}),
            },
          });
          setUpdate({});
        }}
      />
    </div>
  );
}
