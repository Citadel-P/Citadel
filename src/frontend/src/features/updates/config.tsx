import { useMemo, useState } from 'react';
import { toast } from 'sonner';
import { Clock } from 'lucide-react';
import { AgentUpdates, agentOptions } from './agents';
import { ReviewUpdates } from './review';
import { AvailableUpdateHeading, CoreUpdateAvailability, DatabaseUpdateAvailability } from './available';
import { useRead, useMutate } from '@/lib/hooks';
import { UpdateScheduleView, LicenseCapability } from '@/api/generated/api.types';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';
import { MultiSelect } from '@/components/ui/multi-select';
import { TimezoneSelectField } from '@/components/custom/timezone-select';
import {
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldSelect,
  FieldSwitch,
  FormShell,
} from '@/components/custom/form-builder';
import { useUpdateInstallationSettings } from '@/hooks/use-installation-settings';
import { Timestamp } from '@/components/custom/timestamp';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { AlertMessage } from '@/components/custom/alert-message';
import { requestErrorMessage } from '@/lib/request-error';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';

type UpdateConfig = UpdateScheduleView & {
  updateChecksEnabled: boolean;
  updateCheckIntervalHours: number;
  coreStatus: null;
  databaseStatus: null;
  agentSelection: null;
  reviewUpdates: null;
  scheduleInfo: null;
  nextCheck: null;
};
const emptySchedule: UpdateScheduleView = {
  includeCore: false,
  targets: [],
  enabled: false,
  cron: '0 3 * * *',
  timeZone: 'UTC',
};

export function UpdateCheckSettings() {
  const query = useRead('getInstallationSettings');
  const settings = query.data?.data;
  const status = useRead('getInstanceUpdates');
  const agentQuery = useRead('getAgentUpdates');
  const agents = agentQuery.data?.data;
  const schedule = agents?.schedule ?? emptySchedule;
  const { hasCapability } = useLicenseEntitlements();
  const team = hasCapability(LicenseCapability.AutomatedOperations);
  const saveSchedule = useMutate('updateUpdateSchedule', { onSuccess: () => toast.success('Update schedule saved') });
  const formatDate = useProfileDateTimeFormatter();
  const save = useUpdateInstallationSettings();
  const [update, setUpdate] = useState<Partial<UpdateConfig>>({});
  const [selectedAgents, setSelectedAgents] = useState<string[]>([]);
  const busy =
    !!status.data?.data.operation ||
    (!!agents?.operation && !agents.operation.finished) ||
    (!!agents?.scheduleRun && agents.scheduleRun.phase !== 'Finished');
  const saving = save.isPending || saveSchedule.isPending;
  const scheduleEnabled = update.enabled ?? schedule.enabled;
  const includeCore = update.includeCore ?? schedule.includeCore;
  const automaticChecks = update.updateChecksEnabled ?? settings?.updateChecksEnabled;
  const allowed = !!settings?.updateChecksAllowed;
  const scheduleDisabled = saving || !allowed || !team || !agents || !scheduleEnabled;
  const coreBlocker =
    status.data?.data.installationUnavailableReason ?? (!status.data ? 'Core status is unavailable.' : undefined);
  const nextCheck =
    status.data?.data.lastAttempt && settings
      ? Date.parse(status.data.data.lastAttempt) + settings.updateCheckIntervalHours * 3_600_000
      : undefined;
  const nextCheckLabel =
    !allowed || !settings?.updateChecksEnabled
      ? 'Disabled'
      : status.isError
        ? 'Unavailable'
        : !status.data
          ? 'Loading…'
          : nextCheck
            ? null
            : 'Due now';
  const nextRunLabel = !schedule.enabled
    ? 'Disabled'
    : !team || !allowed
      ? 'Paused'
      : (schedule.includeCore && coreBlocker) || !agents?.nextRun
        ? 'Unavailable'
        : null;
  const schema = useMemo(
    () => ({
      '': defineSection<UpdateConfig>({
        items: [
          defineGroupField<UpdateConfig>({
            id: 'available-updates',
            label: 'Available updates',
            title: <AvailableUpdateHeading status={status.data?.data} />,
            items: [
              defineField({
                key: 'coreStatus',
                label: 'Citadel Core',
                render: () => <CoreUpdateAvailability data={status.data?.data} isError={status.isError} busy={busy} />,
              }),
              defineField({
                key: 'databaseStatus',
                label: 'Database',
                render: () => <DatabaseUpdateAvailability data={status.data?.data} />,
              }),
              defineField({
                key: 'agentSelection',
                label: 'Agents',
                render: () => (
                  <AgentUpdates
                    state={agents}
                    failed={agentQuery.isError}
                    allowed={allowed}
                    busy={busy}
                    selected={selectedAgents}
                    onSelect={setSelectedAgents}
                  />
                ),
              }),
              defineField({
                key: 'reviewUpdates',
                label: '',
                render: () => (
                  <ReviewUpdates
                    status={status.data?.data}
                    agents={agents}
                    selected={selectedAgents}
                    allowed={allowed}
                    busy={busy}
                    agentsFailed={agentQuery.isError}
                    onAgentsApproved={() => setSelectedAgents([])}
                  />
                ),
              }),
            ],
          }),
          defineGroupField<UpdateConfig>({
            id: 'release-checks',
            label: 'Release checks',
            description: 'Find new releases. This does not install updates.',
            items: [
              defineField({
                key: 'updateChecksEnabled',
                label: 'Check automatically',
                description: !allowed ? 'Disabled by server configuration.' : undefined,
                render: (value, set) => (
                  <FieldSwitch
                    id="updateChecksEnabled"
                    checked={value}
                    aria-label="Check automatically"
                    disabled={saving || !allowed}
                    onChange={(updateChecksEnabled) => set({ updateChecksEnabled })}
                  />
                ),
              }),
              defineField({
                key: 'updateCheckIntervalHours',
                label: 'Frequency',
                render: (value, set) => (
                  <FieldSelect
                    value={String(value)}
                    options={[
                      { value: '6', label: 'Every 6 hours' },
                      { value: '12', label: 'Every 12 hours' },
                      { value: '24', label: 'Daily' },
                    ]}
                    disabled={saving || !allowed || !automaticChecks}
                    onChange={(hours) => set({ updateCheckIntervalHours: Number(hours) })}
                  />
                ),
              }),
              defineField({
                key: 'nextCheck',
                label: 'Next automatic check',
                render: () => (
                  <p className="text-sm text-muted-foreground">
                    {nextCheckLabel ?? (
                      <Timestamp
                        value={new Date(nextCheck!).toISOString()}
                        formatDateTime={formatDate}
                        className="text-inherit"
                      />
                    )}
                  </p>
                ),
              }),
            ],
          }),
          defineGroupField<UpdateConfig>({
            id: 'scheduled-updates',
            label: 'Scheduled updates',
            requiredLicense: 'Team',
            description: 'Install supported updates for the components you select.',
            items: [
              defineField<UpdateConfig>({
                key: 'enabled',
                label: 'Enabled',
                render: (value, set) => (
                  <FieldSwitch
                    id="update-schedule-enabled"
                    aria-label="Scheduled updates enabled"
                    checked={value}
                    disabled={saving || !agents || ((!team || !allowed) && !schedule.enabled)}
                    onChange={(enabled) => set({ enabled })}
                  />
                ),
              }),

              defineField<UpdateConfig>({
                key: 'includeCore',
                label: 'Update Citadel Core',
                disabled: scheduleDisabled,
                description: includeCore
                  ? 'Citadel Core updates first and temporarily disconnects the interface. Selected Agents follow after Core is verified.'
                  : 'Include Core in scheduled installation.',
                render: (value, set) => (
                  <div className="space-y-2">
                    <Checkbox
                      aria-label="Update Citadel Core"
                      checked={value ?? false}
                      disabled={scheduleDisabled || (!!coreBlocker && !value)}
                      onCheckedChange={(checked) => set({ includeCore: checked === true })}
                    />
                    {coreBlocker && <p className="text-sm text-muted-foreground">{coreBlocker}</p>}
                  </div>
                ),
              }),
              defineField<UpdateConfig>({
                key: 'targets',
                label: 'Scheduled Agents',
                disabled: scheduleDisabled,
                validate: (value) =>
                  scheduleEnabled && !includeCore && !value.length
                    ? 'Select Core or at least one Agent.'
                    : value.length > 100
                      ? 'Select at most 100 Agents.'
                      : null,
                render: (value, set) => (
                  <div className="max-w-100">
                    <span id="scheduled-agent-selection" className="sr-only">
                      Scheduled Agents
                    </span>
                    <MultiSelect
                      aria-labelledby="scheduled-agent-selection"
                      options={[
                        ...agentOptions(agents?.targets ?? [], true),
                        ...schedule.targets
                          .filter((id) => !agents?.targets.some((t) => t.id === id))
                          .map((id) => ({
                            value: id,
                            label: `${id} · no longer registered`,
                            disabled: true,
                            disabledReason: 'Remove this target before saving the schedule.',
                          })),
                      ]}
                      defaultValue={value}
                      onValueChange={(targets) => set({ targets })}
                      disabled={scheduleDisabled}
                      placeholder="Select scheduled Agents"
                    />
                  </div>
                ),
              }),
              defineField<UpdateConfig>({
                key: 'cron',
                label: 'Cron',
                disabled: scheduleDisabled,
                required: scheduleEnabled,
                validate: (value) =>
                  scheduleEnabled && !String(value ?? '').trim() ? 'Cron is required when schedule is enabled' : null,
                render: (value, set) => (
                  <FieldInput
                    value={value}
                    placeholder="0 3 * * *"
                    disabled={scheduleDisabled}
                    onChange={(cron) => set({ cron })}
                  />
                ),
              }),
              defineField<UpdateConfig>({
                key: 'timeZone',
                label: 'Time Zone',
                disabled: scheduleDisabled,
                required: scheduleEnabled,
                render: (value, set) => (
                  <TimezoneSelectField
                    value={value}
                    disabled={scheduleDisabled}
                    onChange={(timeZone) => set({ timeZone })}
                    className="w-full min-w-0 max-w-100"
                  />
                ),
              }),
              defineField({
                key: 'scheduleInfo',
                label: 'Next run',
                render: () => (
                  <div className="space-y-2 text-sm text-muted-foreground">
                    {!agents ? (
                      <p>{agentQuery.isError ? 'Could not load the saved schedule.' : 'Loading schedule…'}</p>
                    ) : (
                      <p className="flex items-center gap-2">
                        <Clock className="size-3.5" />
                        {nextRunLabel ?? (
                          <Timestamp value={agents.nextRun!} formatDateTime={formatDate} className="text-inherit" />
                        )}
                        {schedule.enabled && <span>· {schedule.timeZone}</span>}
                      </p>
                    )}
                    {!team && (
                      <p>Team enables scheduled installation. Manual updates and release checks remain available.</p>
                    )}
                    {agents?.scheduleRun && agents.scheduleRun.phase !== 'Finished' && (
                      <p>Scheduled update in progress. Changes apply to future occurrences.</p>
                    )}
                    {agents?.scheduleRun?.message && (
                      <AlertMessage type="warning" className="my-0">
                        {agents.scheduleRun.message}
                      </AlertMessage>
                    )}
                  </div>
                ),
              }),
            ],
          }),
        ],
      }),
    }),
    [
      selectedAgents,
      status.isError,
      busy,
      status.data,
      agentQuery.isError,
      agents,
      schedule,
      scheduleEnabled,
      includeCore,
      team,
      saving,
      scheduleDisabled,
      allowed,
      coreBlocker,
      automaticChecks,
      nextRunLabel,
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
            <Button type="button" onClick={() => void query.refetch()}>
              Retry
            </Button>
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
          ...schedule,
          coreStatus: null,
          databaseStatus: null,
          agentSelection: null,
          reviewUpdates: null,
          scheduleInfo: null,
          nextCheck: null,
          updateChecksEnabled: settings.updateChecksEnabled,
          updateCheckIntervalHours: settings.updateCheckIntervalHours,
        }}
        update={update}
        setUpdate={setUpdate}
        pending={saving}
        onSave={async (value) => {
          const core = {
            ...(update.updateChecksEnabled !== undefined && value.updateChecksEnabled !== settings.updateChecksEnabled
              ? { updateChecksEnabled: value.updateChecksEnabled }
              : {}),
            ...(update.updateCheckIntervalHours !== undefined &&
            value.updateCheckIntervalHours !== settings.updateCheckIntervalHours
              ? { updateCheckIntervalHours: value.updateCheckIntervalHours }
              : {}),
          };
          if (Object.keys(core).length) {
            await save.mutateAsync({ data: core });
            setUpdate(({ updateChecksEnabled: _enabled, updateCheckIntervalHours: _hours, ...remaining }) => remaining);
          }
          if (['targets', 'includeCore', 'enabled', 'cron', 'timeZone'].some((key) => key in update)) {
            await saveSchedule.mutateAsync({
              data: {
                targets: value.targets,
                includeCore: value.includeCore ?? false,
                enabled: value.enabled,
                cron: value.cron.trim(),
                timeZone: value.timeZone,
              },
            });
          }
        }}
      />
      {saveSchedule.isError && (
        <p role="alert" className="text-sm text-destructive">
          {requestErrorMessage(saveSchedule.error)}
        </p>
      )}
      {save.isError && (
        <p role="alert" className="text-sm text-destructive">
          Update-check setting could not be saved. {requestErrorMessage(save.error)}
        </p>
      )}
    </>
  );
}
