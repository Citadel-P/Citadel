import {
  AutomationActionInput,
  AutomationActionView,
  AutomationWebhookConfig,
  LookupResourceType,
  ResourceControlState,
  TestAutomationActionInput,
  UpdateAutomationActionInput,
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
import { WebhookConfigField } from '@/components/custom/webhook-config-field';
import { Button } from '@/components/ui/button';
import { TimezoneSelectField } from '@/features/alerters/alert-rules/form/form';
import { ResourceTagSelector } from '@/features/tags/components';
import { MonacoEditor } from '@/lib/monaco';
import { useMutate, useSaveResource } from '@/lib/hooks';
import { useTaskSheet } from '@/lib/atoms';
import { useQueryClient } from '@tanstack/react-query';
import { TestTube2 } from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import { useParams } from 'react-router';
import { toast } from 'sonner';
import { configureAutomationActionEditor } from './automation-action-editor-types';

type AutomationActionFormValue = Omit<AutomationActionInput, 'runAsActorId'> & {
  id?: string;
  runAsActorId: string;
  webhook: AutomationWebhookConfig | null;
  tagIds?: string[] | null;
};

const DEFAULT_CODE = `console.log("Action run", run.id);
console.log("Arguments", args);

const deployments = await citadel.deployments.listDeployments();
console.log("Deployments:", deployments?.deployments?.length ?? 0);
`;

const emptyAction = (): AutomationActionFormValue => ({
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
  tagIds: [],
});

export function AutomationActionForm({
  mode,
  resource,
  metadataChanged,
  disabled,
}: {
  mode: 'add' | 'edit';
  resource?: AutomationActionView;
  metadataChanged?: boolean;
  disabled?: boolean;
}) {
  const { id } = useParams();
  const queryClient = useQueryClient();
  const { open: openSheet } = useTaskSheet('AutomationAction');
  const createAction = useMutate('createAutomationAction');
  const updateAction = useMutate('updateAutomationAction');
  const [update, setUpdate] = useState<Partial<AutomationActionFormValue>>({});

  const original = useMemo(() => toFormValue(resource), [resource]);
  const current = useMemo(() => ({ ...original, ...update }), [original, update]);
  const currentScheduleEnabled = update.scheduleEnabled ?? original.scheduleEnabled;
  const testDisabled = disabled || resource?.controlState === ResourceControlState.Processing;

  const refreshData = useCallback(() => {
    localStorage.removeItem(`automation-action:${id ?? 'new'}`);
    queryClient.invalidateQueries({ queryKey: ['listAutomationActions'] });
    queryClient.invalidateQueries({ queryKey: ['getAutomationAction', { id }] });
    queryClient.invalidateQueries({ queryKey: ['listAutomationActionRuns', { id }] });
  }, [id, queryClient]);

  const { save: handleSave, isPending } = useSaveResource<AutomationActionFormValue, any>({
    mode,
    basePath: 'automation',
    entityName: 'Action',
    onCreate: (payload) => createAction.mutateAsync({ data: toCreateInput(payload) } as any),
    onUpdate: (payload) => updateAction.mutateAsync({ id: id!, data: toUpdateInput(payload, update) } as any),
    onRefresh: refreshData,
  });

  const handleTestDraft = useCallback(() => {
    if (mode !== 'edit' || !id) return;

    const code = current.code ?? '';
    if (!code.trim()) {
      toast.error('Code is required');
      return;
    }

    const argsError = validateJsonObject(current.defaultArgsJson, 'Default args');
    if (argsError) {
      toast.error(argsError);
      return;
    }

    const timeoutSeconds = Number(current.timeoutSeconds);
    if (!Number.isFinite(timeoutSeconds) || timeoutSeconds < 1) {
      toast.error('Timeout must be at least 1 second');
      return;
    }

    const payload: TestAutomationActionInput = {
      code,
      argsJson: null,
      defaultArgsJson: normalizeJsonObject(current.defaultArgsJson),
      timeoutSeconds,
      runAsActorId: current.runAsActorId?.trim() || null,
    };

    openSheet({
      kind: 'automationActionRun',
      payload: {
        id,
        name: current.name?.trim() || resource?.name || 'Action',
        mode: 'test',
        ...payload,
      },
    });
  }, [current, id, mode, openSheet, resource?.name]);

  const schema = useMemo(
    () => ({
      Action: defineSection<AutomationActionFormValue>({
        title: 'Action',
        items: [
          defineGroupField<AutomationActionFormValue>({
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
                      description: 'Optional note shown in the action header.',
                      render: (value, set) => (
                        <FieldTextArea
                          value={value ?? ''}
                          placeholder="Restart stale services after maintenance"
                          onChange={(description) => set({ description })}
                        />
                      ),
                    }),
                    defineField({
                      key: 'tagIds',
                      label: 'Tags',
                      description: 'Optional tags for filtering and grouping this action.',
                      render: (value, set) => (
                        <ResourceTagSelector value={value} disabled={disabled} onChange={(tagIds) => set({ tagIds })} />
                      ),
                    }),
                  ]
                : [
                    defineField({
                      key: 'enabled',
                      label: 'Enabled',
                      render: (value, set) => (
                        <FieldSwitch
                          checked={value ?? false}
                          id="automation-action-enabled"
                          onChange={(enabled) => set({ enabled })}
                          disabled={disabled}
                        />
                      ),
                    }),
                  ],
          }),
          defineGroupField<AutomationActionFormValue>({
            id: 'script',
            label: 'Script',
            description: 'TypeScript executed by Deno with Citadel helpers available as globals.',
            items: [
              ...(mode === 'add'
                ? [
                    defineField<AutomationActionFormValue, 'enabled'>({
                      key: 'enabled',
                      label: 'Enabled',
                      render: (value, set) => (
                        <FieldSwitch
                          checked={value ?? false}
                          id="automation-action-enabled"
                          onChange={(enabled) => set({ enabled })}
                          disabled={disabled}
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
                      filename="action.mts"
                      minHeight={360}
                      folding
                      minimap
                      readOnly={disabled}
                      configureMonaco={configureAutomationActionEditor}
                      onValueChange={(code) => set({ code })}
                    />
                    {mode === 'edit' && (
                      <div className="flex justify-end">
                        <Button type="button" variant="outline" disabled={testDisabled} onClick={handleTestDraft}>
                          <TestTube2 className="size-3.5" />
                          Test Draft
                        </Button>
                      </div>
                    )}
                  </div>
                ),
              }),
              defineField({
                key: 'defaultArgsJson',
                label: 'Default Args',
                description: 'JSON object passed to args when the action runs without custom input.',
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
      Execution: defineSection<AutomationActionFormValue>({
        title: 'Execution',
        items: [
          defineGroupField<AutomationActionFormValue>({
            id: 'runtime',
            label: 'Runtime',
            items: [
              defineField({
                key: 'runAsActorId',
                label: 'Run As User',
                description: 'Permissions are evaluated at run time for this user.',
                render: (value, set) => (
                  <ResourceSelectorField
                    targetType={LookupResourceType.User}
                    selected={value ?? undefined}
                    placeholder="Current user"
                    disabled={disabled}
                    onSelect={(user) => set({ runAsActorId: user?.id ?? '' })}
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
                    id="automation-action-alert-on-failure"
                    onChange={(alertOnFailure) => set({ alertOnFailure })}
                    disabled={disabled}
                  />
                ),
              }),
            ],
          }),
        ],
      }),
      Triggers: defineSection<AutomationActionFormValue>({
        title: 'Triggers',
        items: [
          defineGroupField<AutomationActionFormValue>({
            id: 'schedule',
            label: 'Schedule',
            description: 'Run this action automatically from a cron expression.',
            items: [
              defineField({
                key: 'scheduleEnabled',
                label: 'Enabled',
                render: (value, set) => (
                  <FieldSwitch
                    checked={value ?? false}
                    id="automation-action-schedule-enabled"
                    onChange={(scheduleEnabled) => set({ scheduleEnabled })}
                    disabled={disabled}
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
                    disabled={disabled || !currentScheduleEnabled}
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
                    disabled={disabled || !currentScheduleEnabled}
                    onChange={(scheduleTimeZone) => set({ scheduleTimeZone })}
                    className="w-100"
                  />
                ),
              }),
            ],
          }),
          defineGroupField<AutomationActionFormValue>({
            id: 'webhook',
            label: 'Webhook',
            description: 'Allow a Git webhook to queue this action through the shared listener.',
            items: [
              defineField<AutomationActionFormValue, 'webhook'>({
                key: 'webhook',
                label: 'Enabled',
                render: (value, set) => (
                  <WebhookConfigField
                    resourceType="automation-action"
                    resourceId={id}
                    execution="run"
                    value={value ?? { enabled: false }}
                    disabled={disabled}
                    onChange={(webhook) => set({ webhook: webhook as AutomationWebhookConfig })}
                  />
                ),
              }),
            ],
          }),
        ],
      }),
    }),
    [currentScheduleEnabled, disabled, handleTestDraft, id, mode, testDisabled],
  );

  return (
    <FormShell
      mode={mode}
      schema={schema}
      original={original}
      update={update}
      setUpdate={setUpdate}
      onSave={handleSave}
      pending={isPending}
      disabled={disabled}
      draftKey={`automation-action:${id ?? 'new'}`}
      draftVersion={metadataChanged ? 2 : 1}
    />
  );
}

function toFormValue(action?: AutomationActionView): AutomationActionFormValue {
  if (!action) return emptyAction();

  return {
    id: action.id,
    name: action.name,
    description: action.description ?? '',
    code: action.code,
    defaultArgsJson: action.defaultArgsJson || '{}',
    enabled: action.enabled,
    scheduleEnabled: action.scheduleEnabled,
    scheduleCron: action.scheduleCron ?? '',
    scheduleTimeZone: action.scheduleTimeZone || 'UTC',
    webhook: action.webhook ?? { enabled: false },
    timeoutSeconds: Number(action.timeoutSeconds),
    alertOnFailure: action.alertOnFailure,
    runAsActorId: action.runAsActorId ?? '',
    tagIds: action.tags?.map((tag) => tag.id) ?? [],
  };
}

function toCreateInput(value: AutomationActionFormValue): AutomationActionInput {
  return {
    name: value.name.trim(),
    description: value.description?.trim() || null,
    code: value.code,
    defaultArgsJson: normalizeJsonObject(value.defaultArgsJson),
    enabled: value.enabled,
    scheduleEnabled: value.scheduleEnabled,
    scheduleCron: value.scheduleEnabled ? value.scheduleCron?.trim() || null : null,
    scheduleTimeZone: value.scheduleEnabled ? value.scheduleTimeZone?.trim() || 'UTC' : 'UTC',
    webhook: normalizeWebhook(value.webhook),
    timeoutSeconds: Number(value.timeoutSeconds) || 300,
    alertOnFailure: value.alertOnFailure,
    runAsActorId: value.runAsActorId?.trim() || null,
    tagIds: value.tagIds ?? [],
  };
}

function toUpdateInput(
  value: AutomationActionFormValue,
  update: Partial<AutomationActionFormValue>,
): UpdateAutomationActionInput {
  const input: UpdateAutomationActionInput = {
    code: value.code,
    defaultArgsJson: normalizeJsonObject(value.defaultArgsJson),
    enabled: value.enabled,
    scheduleEnabled: value.scheduleEnabled,
    scheduleTimeZone: value.scheduleTimeZone?.trim() || 'UTC',
    timeoutSeconds: Number(value.timeoutSeconds) || 300,
    alertOnFailure: value.alertOnFailure,
    runAsActorId: value.runAsActorId?.trim() || null,
  };

  if (update.scheduleCron !== undefined || update.scheduleEnabled !== undefined) {
    input.scheduleCron = value.scheduleEnabled ? value.scheduleCron?.trim() || null : null;
  }

  if (update.webhook !== undefined) {
    input.webhook = normalizeWebhook(value.webhook);
  }

  return input;
}

function normalizeWebhook(webhook: AutomationWebhookConfig | null | undefined): AutomationWebhookConfig | null {
  if (!webhook) return null;

  return {
    ...webhook,
    secret: webhook.secret?.trim() || null,
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
