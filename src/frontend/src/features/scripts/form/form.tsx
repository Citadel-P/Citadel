import {
  ScriptInput,
  RunInput,
  AuthorizedScript,
  WebhookConfig,
  LicenseCapability,
  LookupResourceType,
  ResourceControlState,
  UpdateScriptInput,
} from '@/api/generated/api.types';
import {
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldSwitch,
  FieldTextArea,
  FormShell,
} from '@/components/custom/form-builder';
import { ResourceSelectorField } from '@/components/custom/common';
import { TimezoneSelectField } from '@/components/custom/timezone-select';
import { WebhookConfigField } from '@/components/custom/webhook-config-field';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { ResourceTagSelector } from '@/features/tags/components';
import { configureScriptEditor, MonacoEditor } from '@/lib/monaco';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { useTaskSheet } from '@/lib/atoms';
import { useQueryClient } from '@tanstack/react-query';
import { TestTube2 } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useParams, useSearchParams } from 'react-router';
import { useConfigurationCopyReview } from '@/components/custom/use-configuration-copy-review';
import { AlertMessage } from '@/components/custom/alert-message';
import { LicensedFeatureLabel } from '@/components/custom/license-feature-indicator';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';

type ScriptFormValue = Omit<ScriptInput, 'runAsActorId'> & {
  id?: string;
  runAsActorId: string;
  webhook: WebhookConfig | null;
  tagIds?: string[] | null;
};

const executorOptions = [
  { value: 'Core', label: 'Core', description: 'Run on the Citadel Core server.' },
  { value: 'Agent', label: 'Specific agent', description: 'Always run on the agent you select.' },
  {
    value: 'Automatic',
    label: 'Automatic',
    description: 'Let Citadel choose an eligible agent with available capacity.',
  },
] as const;

const DEFAULT_CODE = `console.log("Script run", run.id);
console.log("Arguments", args);

const result = await citadel.deployments.listDeployments();
console.log("Deployments:", result?.deployments?.length ?? 0);
`;

const emptyScript = (): ScriptFormValue => ({
  name: '',
  description: '',
  code: DEFAULT_CODE,
  defaultArgsJson: '{\n  "dryRun": true\n}',
  enabled: true,
  scheduleEnabled: false,
  scheduleCron: '',
  scheduleTimeZone: 'UTC',
  webhook: { enabled: false },
  timeoutSeconds: 300,
  alertOnFailure: false,
  runAsActorId: '',
  executor: { kind: 'Core' },
  tagIds: [],
});

export function ScriptForm({
  mode,
  resource,
  metadataChanged,
  disabled,
}: {
  mode: 'add' | 'edit';
  resource?: AuthorizedScript;
  metadataChanged?: boolean;
  disabled?: boolean;
}) {
  const { id } = useParams();
  const [searchParams] = useSearchParams();
  const duplicateFrom = mode === 'add' ? searchParams.get('duplicateFrom') : null;
  const loadedDuplicate = useRef<string | null>(null);
  const { data: duplicateData, isFetching: duplicateLoading } = useRead(
    'getScriptDuplicateDraft',
    { id: duplicateFrom ?? '' },
    { enabled: !!duplicateFrom },
  );
  const duplicate = duplicateData?.data;
  const queryClient = useQueryClient();
  const { open: openSheet } = useTaskSheet('Script');
  const createScript = useMutate('createScript');
  const updateScript = useMutate('updateScript');
  const [update, setUpdate] = useState<Partial<ScriptFormValue>>({});
  useEffect(() => {
    if (!duplicateFrom || !duplicate || loadedDuplicate.current === duplicateFrom) return;
    loadedDuplicate.current = duplicateFrom;
    setUpdate({ ...duplicate.draft, runAsActorId: '', webhook: null });
  }, [duplicateFrom, duplicate]);
  const copyReview = useConfigurationCopyReview<ScriptFormValue>({
    sourceName: duplicate?.sourceName ?? '',
    describe: (payload) => (
      <>
        <p>
          Create <strong>{payload.name}</strong> as a disabled Script, with its schedule and webhook off.
        </p>
        <p>The new Script runs as you when enabled. No run history, access grants or webhook credentials are copied.</p>
        <p>
          Review the script and arguments for resource IDs, production endpoints and credentials. These values are
          copied as written and may still target the original resources.
        </p>
      </>
    ),
  });
  const { hasCapability: hasLicenseCapability } = useLicenseEntitlements();
  const automatedOperationsEnabled = hasLicenseCapability(LicenseCapability.AutomatedOperations);

  const original = useMemo(() => toFormValue(resource), [resource]);
  const current = useMemo(() => ({ ...original, ...update }), [original, update]);
  const currentScheduleEnabled = update.scheduleEnabled ?? original.scheduleEnabled;
  const testDisabledReason =
    resource?.capabilities?.canExecute !== true
      ? 'Execute permission is required to test this Script.'
      : resource.controlState === ResourceControlState.Processing
        ? 'Wait for the current run to finish.'
        : !current.code.trim()
          ? 'Code is required to test this Script.'
          : (validateJsonObject(current.defaultArgsJson, 'Default args') ?? undefined);

  const refreshData = useCallback(() => {
    localStorage.removeItem(`script:${id ?? 'new'}`);
    queryClient.invalidateQueries({ queryKey: ['listScripts'] });
    queryClient.invalidateQueries({ queryKey: ['getScript', { id }] });
    queryClient.invalidateQueries({ queryKey: ['listScriptRuns', { id }] });
  }, [id, queryClient]);

  const { save: handleSave, isPending } = useSaveResource<ScriptFormValue, any>({
    mode,
    basePath: 'scripts',
    entityName: 'Script',
    onCreate: (payload) => createScript.mutateAsync({ data: toCreateInput(payload) } as any),
    onUpdate: (payload) => updateScript.mutateAsync({ id: id!, data: toUpdateInput(payload, update) } as any),
    onRefresh: refreshData,
  });

  const handleTestDraft = useCallback(() => {
    if (mode !== 'edit' || !id || testDisabledReason) return;

    const payload: RunInput = {
      argsJson: current.defaultArgsJson || '{}',
      ...(current.code !== original.code ? { code: current.code } : {}),
    };

    openSheet({
      kind: 'scriptRun',
      payload: {
        id,
        name: current.name?.trim() || resource?.name || 'Script',
        mode: 'test',
        ...payload,
      },
    });
  }, [
    current.name,
    current.code,
    current.defaultArgsJson,
    original.code,
    testDisabledReason,
    id,
    mode,
    openSheet,
    resource?.name,
  ]);

  const schema = useMemo(
    () => ({
      Script: defineSection<ScriptFormValue>({
        title: '',
        items: [
          defineGroupField<ScriptFormValue>({
            id: 'identity',
            label: 'Identity',
            items:
              mode === 'add'
                ? [
                    defineField({
                      key: 'name',
                      label: 'Name',
                      required: true,
                      description: 'Stable name used in Citadel activity and run history.',
                      render: (value, set) => (
                        <FieldInput
                          value={value ?? ''}
                          placeholder="restart-stale-services"
                          onChange={(name) => set({ name })}
                        />
                      ),
                    }),
                    defineField({
                      key: 'description',
                      label: 'Description',
                      description: 'Optional note shown in the script header.',
                      render: (value, set) => (
                        <FieldTextArea
                          markdown
                          value={value ?? ''}
                          placeholder="Restart stale services after maintenance"
                          onChange={(description) => set({ description })}
                        />
                      ),
                    }),
                    defineField({
                      key: 'tagIds',
                      label: 'Tags',
                      description: 'Optional tags for filtering and grouping this script.',
                      render: (value, set) => (
                        <ResourceTagSelector value={value} disabled={disabled} onChange={(tagIds) => set({ tagIds })} />
                      ),
                    }),
                  ]
                : [
                    defineField({
                      key: 'enabled',
                      label: 'Enabled',
                      description: 'Allow manual runs and configured schedule or webhook triggers.',
                      render: (value, set) => (
                        <FieldSwitch
                          checked={value ?? false}
                          id="script-enabled"
                          onChange={(enabled) => set({ enabled })}
                          disabled={disabled || !!duplicateFrom}
                        />
                      ),
                    }),
                  ],
          }),
          defineGroupField<ScriptFormValue>({
            id: 'script',
            label: 'Script',
            description: 'TypeScript executed by Deno with Citadel helpers available as globals.',
            items: [
              ...(mode === 'add'
                ? [
                    defineField<ScriptFormValue, 'enabled'>({
                      key: 'enabled',
                      label: 'Enabled',
                      description: 'Allow manual runs and configured schedule or webhook triggers.',
                      render: (value, set) => (
                        <FieldSwitch
                          checked={value ?? false}
                          id="script-enabled"
                          onChange={(enabled) => set({ enabled })}
                          disabled={disabled || !!duplicateFrom}
                        />
                      ),
                    }),
                  ]
                : []),
              defineField({
                key: 'code',
                label: 'Code',
                required: true,
                validate: (value) => (!String(value ?? '').trim() ? 'Code is required' : null),
                render: (value, set) => (
                  <div className="flex flex-col gap-2">
                    <MonacoEditor
                      value={value ?? ''}
                      language="typescript"
                      filename="script.mts"
                      minHeight={360}
                      folding
                      minimap
                      readOnly={disabled}
                      configureMonaco={configureScriptEditor}
                      onValueChange={(code) => set({ code })}
                    />
                    {mode === 'edit' && (
                      <div className="flex justify-end">
                        <TestDraftButton disabledReason={testDisabledReason} onClick={handleTestDraft} />
                      </div>
                    )}
                  </div>
                ),
              }),
              defineField({
                key: 'defaultArgsJson',
                label: 'Default Args',
                description: 'JSON object passed to args when the script runs without custom input.',
                validate: (value) => validateJsonObject(value, 'Default args'),
                render: (value, set) => (
                  <MonacoEditor
                    value={value ?? '{}'}
                    language="json"
                    filename="args.json"
                    minHeight={140}
                    folding
                    readOnly={disabled}
                    onValueChange={(defaultArgsJson) => set({ defaultArgsJson })}
                  />
                ),
              }),
            ],
          }),
        ],
      }),
      Execution: defineSection<ScriptFormValue>({
        title: 'Execution',
        items: [
          defineGroupField<ScriptFormValue>({
            id: 'runtime',
            label: 'Runtime',
            items: [
              defineField({
                key: 'executor',
                label: 'Run on',
                description: 'Choose where this script runs.',
                validate: (value) => {
                  if (value?.kind === 'Agent' && !value.agentId) return 'Select an agent';
                  return null;
                },
                render: (value, set) => (
                  <div className="flex flex-col gap-2">
                    <Select
                      value={value?.kind ?? 'Core'}
                      disabled={disabled}
                      onValueChange={(kind) =>
                        set({
                          executor:
                            kind === 'Agent'
                              ? { kind: 'Agent', agentId: '' }
                              : { kind: kind === 'Automatic' ? 'Automatic' : 'Core' },
                        })
                      }>
                      <SelectTrigger aria-label="Run on" className="w-full max-w-100">
                        <SelectValue>
                          <LicensedFeatureLabel
                            requiredLicense={
                              value?.kind === 'Automatic' && !automatedOperationsEnabled ? 'Team' : undefined
                            }>
                            {executorOptions.find((option) => option.value === (value?.kind ?? 'Core'))?.label}
                          </LicensedFeatureLabel>
                        </SelectValue>
                      </SelectTrigger>
                      <SelectContent>
                        {executorOptions.map((option) => (
                          <SelectItem
                            key={option.value}
                            value={option.value}
                            textValue={option.label}
                            disabled={option.value === 'Automatic' && !automatedOperationsEnabled}>
                            <div className="flex flex-col">
                              <LicensedFeatureLabel
                                className="font-medium"
                                requiredLicense={
                                  option.value === 'Automatic' && !automatedOperationsEnabled ? 'Team' : undefined
                                }>
                                {option.label}
                              </LicensedFeatureLabel>
                              <span className="text-xs text-muted-foreground">{option.description}</span>
                            </div>
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    {value?.kind === 'Automatic' && !automatedOperationsEnabled && (
                      <p className="text-sm text-muted-foreground">
                        Automatic requires Team. Choose Core or a specific agent to run this script.
                      </p>
                    )}
                    {value?.kind === 'Agent' && (
                      <ResourceSelectorField
                        targetType={LookupResourceType.ScriptAgent}
                        selected={value.agentId}
                        disabled={disabled}
                        placeholder="Select agent"
                        onSelect={(agent) => set({ executor: { kind: 'Agent', agentId: agent?.id ?? '' } })}
                      />
                    )}
                  </div>
                ),
              }),
              defineField({
                key: 'runAsActorId',
                label: 'Run as',
                description: 'Permissions are evaluated at run time for this identity.',
                render: (value, set) => (
                  <ResourceSelectorField
                    targetType={LookupResourceType.RunAsActor}
                    selected={value ?? undefined}
                    placeholder="Current user"
                    disabled={disabled || !!duplicateFrom}
                    onSelect={(actor) => set({ runAsActorId: actor?.id ?? '' })}
                  />
                ),
              }),
              defineField({
                key: 'timeoutSeconds',
                label: 'Timeout',
                required: true,
                description: 'Maximum run duration in seconds.',
                validate: (value) => {
                  const n = Number(value);
                  if (!Number.isFinite(n) || n < 1) return 'Timeout must be at least 1 second';
                  return null;
                },
                render: (value, set) => (
                  <FieldInput
                    type="number"
                    value={value ?? 300}
                    placeholder="300"
                    disabled={disabled}
                    onChange={(timeoutSeconds) => set({ timeoutSeconds })}
                  />
                ),
              }),
              defineField({
                key: 'alertOnFailure',
                label: 'Alert On Failure',
                description: 'Record failed and timed-out runs as alert-worthy events.',
                render: (value, set) => (
                  <FieldSwitch
                    checked={value ?? false}
                    id="script-alert-on-failure"
                    onChange={(alertOnFailure) => set({ alertOnFailure })}
                    disabled={disabled}
                  />
                ),
              }),
            ],
          }),
        ],
      }),
      Triggers: defineSection<ScriptFormValue>({
        title: 'Triggers',
        items: [
          defineGroupField<ScriptFormValue>({
            id: 'schedule',
            label: 'Schedule',
            description: 'Run this script automatically from a cron expression.',
            requiredLicense: automatedOperationsEnabled ? undefined : 'Team',
            items: [
              defineField({
                key: 'scheduleEnabled',
                label: 'Enabled',
                render: (value, set) => (
                  <FieldSwitch
                    checked={value ?? false}
                    id="script-schedule-enabled"
                    onChange={(scheduleEnabled) => set({ scheduleEnabled })}
                    disabled={disabled || !!duplicateFrom || !automatedOperationsEnabled}
                  />
                ),
              }),
              defineField({
                key: 'scheduleCron',
                label: 'Cron',
                required: currentScheduleEnabled,
                disabled: !currentScheduleEnabled,
                validate: (value) =>
                  currentScheduleEnabled && !String(value ?? '').trim()
                    ? 'Cron is required when schedule is enabled'
                    : null,
                render: (value, set) => (
                  <FieldInput
                    value={value ?? ''}
                    placeholder="*/15 * * * *"
                    disabled={disabled || !currentScheduleEnabled || !automatedOperationsEnabled}
                    onChange={(scheduleCron) => set({ scheduleCron })}
                  />
                ),
              }),
              defineField({
                key: 'scheduleTimeZone',
                label: 'Time Zone',
                required: currentScheduleEnabled,
                disabled: !currentScheduleEnabled,
                render: (value, set) => (
                  <TimezoneSelectField
                    value={value ?? 'UTC'}
                    disabled={disabled || !currentScheduleEnabled || !automatedOperationsEnabled}
                    onChange={(scheduleTimeZone) => set({ scheduleTimeZone })}
                    className="w-full min-w-0 max-w-100"
                  />
                ),
              }),
            ],
          }),
          defineGroupField<ScriptFormValue>({
            id: 'webhook',
            label: 'Webhook',
            description: 'Allow a Git webhook to queue this script through the shared listener.',
            requiredLicense: automatedOperationsEnabled ? undefined : 'Team',
            items: [
              defineField<ScriptFormValue, 'webhook'>({
                key: 'webhook',
                label: 'Enabled',
                render: (value, set) => (
                  <WebhookConfigField
                    resourceType="script"
                    resourceId={id}
                    execution="run"
                    value={value ?? { enabled: false }}
                    disabled={disabled}
                    enableDisabled={!!duplicateFrom || !automatedOperationsEnabled}
                    onChange={(webhook) => set({ webhook: webhook as WebhookConfig })}
                  />
                ),
              }),
            ],
          }),
        ],
      }),
    }),
    [
      automatedOperationsEnabled,
      currentScheduleEnabled,
      duplicateFrom,
      disabled,
      handleTestDraft,
      id,
      mode,
      testDisabledReason,
    ],
  );

  return (
    <>
      {duplicateFrom && (
        <AlertMessage type="info" title="Configuration copy">
          Review the script and target references. The Script is created disabled, with its schedule and webhook off.
        </AlertMessage>
      )}
      <FormShell
        mode={mode}
        schema={schema}
        original={original}
        update={update}
        setUpdate={setUpdate}
        onSave={(value) =>
          handleSave({
            ...value,
            // Form edits merge nested objects; discard fields from the previous placement type.
            executor:
              value.executor?.kind === 'Agent'
                ? { kind: 'Agent', agentId: value.executor.agentId }
                : { kind: value.executor?.kind ?? 'Core' },
          })
        }
        pending={isPending}
        disabled={disabled}
        saveDisabled={!!duplicateFrom && (duplicateLoading || !duplicate)}
        saveLabel={duplicateFrom ? 'Review copy' : 'Save'}
        confirmSave={duplicateFrom ? copyReview.confirmSave : undefined}
        draftKey={duplicateFrom ? undefined : `script:${id ?? 'new'}`}
        draftVersion={metadataChanged ? 2 : 1}
      />
      {copyReview.review}
    </>
  );
}

function TestDraftButton({ disabledReason, onClick }: { disabledReason?: string; onClick: () => void }) {
  const button = (
    <Button type="button" variant="outline" disabled={disabledReason !== undefined} onClick={onClick}>
      <TestTube2 className="size-3.5" />
      Test Draft
    </Button>
  );

  if (!disabledReason) return button;

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span className="inline-flex">{button}</span>
      </TooltipTrigger>
      <TooltipContent>{disabledReason}</TooltipContent>
    </Tooltip>
  );
}

function toFormValue(script?: AuthorizedScript): ScriptFormValue {
  if (!script) return emptyScript();

  return {
    id: script.id,
    name: script.name,
    description: script.description ?? '',
    code: script.code,
    defaultArgsJson: script.defaultArgsJson || '{}',
    enabled: script.enabled,
    scheduleEnabled: script.scheduleEnabled,
    scheduleCron: script.scheduleCron ?? '',
    scheduleTimeZone: script.scheduleTimeZone || 'UTC',
    webhook: script.webhook ?? { enabled: false },
    timeoutSeconds: Number(script.timeoutSeconds),
    alertOnFailure: script.alertOnFailure,
    runAsActorId: script.runAsActorId ?? '',
    executor: script.executor,
    tagIds: script.tags?.map((tag) => tag.id) ?? [],
  };
}

function toCreateInput(value: ScriptFormValue): ScriptInput {
  return {
    duplicateSourceId: value.duplicateSourceId,
    name: value.name.trim(),
    description: value.description?.trim() || null,
    code: value.code,
    defaultArgsJson: normalizeJsonObject(value.defaultArgsJson),
    enabled: value.enabled,
    scheduleEnabled: value.scheduleEnabled,
    scheduleCron: value.scheduleEnabled || value.duplicateSourceId ? value.scheduleCron?.trim() || null : null,
    scheduleTimeZone:
      value.scheduleEnabled || value.duplicateSourceId ? value.scheduleTimeZone?.trim() || 'UTC' : 'UTC',
    webhook: value.duplicateSourceId ? null : normalizeWebhook(value.webhook),
    timeoutSeconds: Number(value.timeoutSeconds) || 300,
    alertOnFailure: value.alertOnFailure,
    runAsActorId: value.runAsActorId?.trim() || null,
    executor: value.executor ?? { kind: 'Core' },
    tagIds: value.tagIds ?? [],
  };
}

function toUpdateInput(value: ScriptFormValue, update: Partial<ScriptFormValue>): UpdateScriptInput {
  const input: UpdateScriptInput = {
    code: value.code,
    defaultArgsJson: normalizeJsonObject(value.defaultArgsJson),
    enabled: value.enabled,
    scheduleEnabled: value.scheduleEnabled,
    scheduleTimeZone: value.scheduleTimeZone?.trim() || 'UTC',
    timeoutSeconds: Number(value.timeoutSeconds) || 300,
    alertOnFailure: value.alertOnFailure,
    runAsActorId: value.runAsActorId?.trim() || null,
    executor: value.executor ?? { kind: 'Core' },
  };

  if (update.scheduleCron !== undefined || update.scheduleEnabled !== undefined) {
    input.scheduleCron = value.scheduleEnabled ? value.scheduleCron?.trim() || null : null;
  }

  if (update.webhook !== undefined) {
    input.webhook = normalizeWebhook(value.webhook);
  }

  return input;
}

function normalizeWebhook(webhook: WebhookConfig | null | undefined): WebhookConfig | null {
  if (!webhook) return null;

  return {
    ...webhook,
    ...(webhook.secret === undefined ? {} : { secret: webhook.secret?.trim() || null }),
    branchFilter: webhook.branchFilter?.trim() || null,
  };
}

function validateJsonObject(value: unknown, label: string) {
  try {
    const parsed = JSON.parse(String(value || '{}'));
    if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object') {
      return `${label} must be a JSON object`;
    }
    return null;
  } catch {
    return `${label} must be valid JSON`;
  }
}

function normalizeJsonObject(value: unknown) {
  const text = String(value || '{}').trim() || '{}';
  const parsed = JSON.parse(text);
  return JSON.stringify(parsed, null, 2);
}
