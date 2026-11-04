import { useMemo, useState } from 'react';
import { AgentUpdates } from './agents';
import { useRead } from '@/lib/hooks';
import { defineField, defineSection, FieldSelect, FieldSwitch, FormShell } from '@/components/custom/form-builder';
import { useUpdateInstallationSettings } from '@/hooks/use-installation-settings';
import { Timestamp } from '@/components/custom/timestamp';
import { Button } from '@/components/ui/button';
import { RefreshCw } from 'lucide-react';
import { useCheckInstanceUpdates } from './queries';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';

type UpdateCheckConfig = { updateChecksEnabled: boolean; updateCheckIntervalHours: number };

export function UpdateCheckSettings() {
  const query = useRead('getInstallationSettings');
  const settings = query.data?.data;
  const status = useRead('getInstanceUpdates');
  const { mutate: checkNow, isPending: checking, isError: checkFailed } = useCheckInstanceUpdates();
  const formatDate = useProfileDateTimeFormatter();
  const save = useUpdateInstallationSettings();
  const [update, setUpdate] = useState<Partial<UpdateCheckConfig>>({});
  const automaticChecks = update.updateChecksEnabled ?? settings?.updateChecksEnabled;
  const nextCheck =
    status.data?.data.lastAttempt && settings
      ? Date.parse(status.data.data.lastAttempt) + settings.updateCheckIntervalHours * 3_600_000
      : undefined;
  const nextCheckLabel =
    !settings?.updateChecksAllowed || !settings.updateChecksEnabled
      ? 'Disabled'
      : status.isError
        ? 'Unavailable'
        : !status.data
          ? 'Loading…'
          : nextCheck
            ? null
            : 'Due now';
  const schema = useMemo(
    () => ({
      '': defineSection<UpdateCheckConfig>({
        title: '',
        items: [
          defineField({
            key: 'updateChecksEnabled',
            label: 'Automatic checks',
            description: settings?.updateChecksAllowed
              ? 'Check for new Citadel releases on a schedule. Checking does not install updates.'
              : 'Disabled by server configuration.',
            render: (value, set) => (
              <FieldSwitch
                id="updateChecksEnabled"
                checked={value}
                disabled={save.isPending || !settings?.updateChecksAllowed}
                aria-label="Automatic checks"
                onChange={(updateChecksEnabled) => set({ updateChecksEnabled })}
              />
            ),
          }),
          defineField({
            key: 'updateCheckIntervalHours',
            label: 'Check frequency',
            description: 'Choose how often Citadel checks for new releases.',
            render: (value, set) => (
              <div className="space-y-3">
                <div className="flex flex-wrap items-center gap-2">
                  <FieldSelect
                    value={String(value)}
                    options={[
                      { value: '6', label: 'Every 6 hours' },
                      { value: '12', label: 'Every 12 hours' },
                      { value: '24', label: 'Daily' },
                    ]}
                    disabled={save.isPending || !settings?.updateChecksAllowed || !automaticChecks}
                    onChange={(hours) => set({ updateCheckIntervalHours: Number(hours) })}
                  />
                  <Button
                    type="button"
                    variant="outline"
                    disabled={checking || !settings?.updateChecksAllowed}
                    onClick={() => checkNow({})}>
                    <RefreshCw className={checking ? 'size-4 animate-spin' : 'size-4'} />
                    {checking ? 'Checking…' : 'Check now'}
                  </Button>
                </div>
                <p className="text-sm text-muted-foreground" aria-live="polite">
                  Next automatic check:{' '}
                  {nextCheckLabel ?? (
                    <Timestamp
                      value={new Date(nextCheck!).toISOString()}
                      formatDateTime={formatDate}
                      className="text-sm text-inherit"
                    />
                  )}
                </p>
              </div>
            ),
          }),
        ],
      }),
    }),
    [
      settings?.updateChecksAllowed,
      save.isPending,
      automaticChecks,
      checking,
      checkNow,
      nextCheckLabel,
      nextCheck,
      formatDate,
    ],
  );
  if (!settings)
    return (
      <div role="status">
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
    <>
      <FormShell
        schema={schema}
        original={{
          updateChecksEnabled: settings.updateChecksEnabled,
          updateCheckIntervalHours: settings.updateCheckIntervalHours,
        }}
        update={update}
        setUpdate={setUpdate}
        pending={save.isPending}
        disabled={!settings.updateChecksAllowed}
        onSave={async () => {
          await save.mutateAsync({ data: update });
        }}
      />
      <AgentUpdates />
      {checkFailed && (
        <p role="alert" className="text-sm text-destructive">
          The check could not finish. Try again. Last successful release information is retained.
        </p>
      )}
      {save.isError && (
        <p role="alert" className="text-sm text-destructive">
          Update-check setting could not be saved. Try again.
        </p>
      )}
    </>
  );
}
