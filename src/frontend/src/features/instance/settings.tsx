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
import { MoveUpRight } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
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
            description: section.description,
            items: [
              ...(section.label === 'Security'
                ? [
                    defineField<InstanceFormValue>({
                      key: 'mfaPolicy',
                      label: 'MFA requirement',
                      description: (
                        <div className="space-y-1 text-sm text-muted-foreground">
                          <p>
                            Require MFA for local-password sign-in. Existing sessions are unaffected.{' '}
                            <EnvironmentReference variable="CITADEL_MFA_POLICY" />
                          </p>
                          <p>Deployment minimum: {settings?.minimumMfaPolicy ?? MfaPolicy.Optional}.</p>
                        </div>
                      ),
                      render: (value, set) => (
                        <MfaPolicyControl
                          value={value}
                          minimum={settings?.minimumMfaPolicy ?? MfaPolicy.Optional}
                          disabled={save.isPending}
                          onChange={(mfaPolicy) => set({ mfaPolicy })}
                        />
                      ),
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
                    <span className="flex flex-wrap gap-2">
                      {managed && <Badge variant="outline">Managed by environment</Badge>}
                      {pending && <Badge variant="outline">Restart pending</Badge>}
                    </span>
                  ),
                  description: (
                    <div className="space-y-1 text-sm text-muted-foreground">
                      <p>
                        {field.description} <EnvironmentReference variable={field.environment} />
                      </p>
                      {managed && (
                        <p>
                          Environment value: {String(deploymentValue)}. Remove{' '}
                          <code className="break-all">{field.environment}</code> from Core’s environment to edit here.
                          Recreate the Core container, or restart Core for a non-container installation.
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
                    if (managed)
                      return (
                        <output aria-label={field.label} className="block max-w-100 break-words text-sm font-medium">
                          {typeof effective === 'boolean'
                            ? effective
                              ? 'Enabled'
                              : 'Disabled'
                            : effective === ''
                              ? 'Automatic'
                              : String(effective)}
                        </output>
                      );
                    const disabled = save.isPending;
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
                          <SelectTrigger className="w-full max-w-100" aria-label={field.label}>
                            <SelectValue placeholder={String(value)} />
                          </SelectTrigger>
                          <SelectContent>
                            {field.choices.map((choice) => (
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
                        className="w-full max-w-100"
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

function EnvironmentReference({ variable }: { variable: string }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <a
          href={`https://docs.citadelplane.com/reference/environment-variables/#${variable.toLowerCase().replaceAll('_', '-')}`}
          target="_blank"
          rel="noopener noreferrer"
          aria-label={`Environment variable ${variable}: open documentation`}
          className="inline-flex size-4 align-text-bottom items-center justify-center rounded-sm text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
          <MoveUpRight className="size-3" aria-hidden="true" />
        </a>
      </TooltipTrigger>
      <TooltipContent className="max-w-[min(24rem,calc(100vw-2rem))] space-y-1">
        <code className="break-all">{variable}</code>
        <p>Open environment variable reference</p>
      </TooltipContent>
    </Tooltip>
  );
}
