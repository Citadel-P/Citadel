import {
  AlertResourceType,
  AlertResourceScope,
  AlertQuietHour,
  AlertType,
  AlertSeverity,
  DayOfWeek,
  DeploymentView,
  AlertRuleStatus,
  AlertRuleInput,
  PatchAlertRuleInput,
  LookupResourceType,
  LicenseCapability,
} from '@/api/generated/api.types';
import {
  FormShell,
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldSlider,
  ItemSelector,
} from '@/components/custom/form-builder';
import { useState, useMemo, useCallback } from 'react';
import { useMutate, useSaveResource } from '@/lib/hooks';
import { useParams } from 'react-router';
import { MultiResourceSelectorField, SelectField } from '@/components/custom/common';
import { getDefaultTimezone, TimezoneSelectField } from '@/components/custom/timezone-select';
import { ResourceType } from '@/api/types';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Pencil, Plus, Trash2 } from 'lucide-react';
import { DataTable } from '@/components/ui/data-table';
import { ColumnDef } from '@tanstack/react-table';
import { useQueryClient } from '@tanstack/react-query';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';
import { AlertMessage } from '@/components/custom/alert-message';
import { LicensedFeatureDescription } from '@/components/custom/license-feature-indicator';

type AlertRuleFormResource = AlertRuleFormInput & {
  isSystem?: boolean;
};

type AlertRuleFormInput = AlertRuleInput | PatchAlertRuleInput;

export const AlertRuleForm = ({
  mode,
  resource,
  disabled,
}: {
  mode: 'add' | 'edit';
  resource?: AlertRuleFormResource;
  disabled?: boolean;
}) => {
  const id = useParams().id;
  const queryClient = useQueryClient();
  const [update, setUpdate] = useState<Partial<AlertRuleFormInput>>({});
  const { hasCapability } = useLicenseEntitlements();
  const hasAdvancedAlerting = hasCapability(LicenseCapability.AdvancedAlerting);
  const formDisabled = disabled || (mode === 'add' && !hasAdvancedAlerting);
  const advancedFieldsDisabled = disabled || !hasAdvancedAlerting;
  const isSystemRule = resource?.isSystem === true;

  const { mutateAsync: createAlertRule } = useMutate('createAlertRule');
  const { mutateAsync: updateAlertRule } = useMutate('updateAlertRule');

  const { save: handleSave, isPending } = useSaveResource<AlertRuleFormInput, any>({
    mode,
    basePath: 'alert-rules',
    entityName: 'Alert rule',
    onCreate: (payload) => createAlertRule({ data: payload as AlertRuleInput }),
    onUpdate: (payload) => updateAlertRule({ id: id!, data: payload as PatchAlertRuleInput }),
    onRefresh: () => {
      localStorage.removeItem(`AlertRule:${id ?? 'new'}`);
      queryClient.invalidateQueries({ queryKey: ['getAlertRuleConfig', { id }] });
    },
  });

  const original = useMemo(() => (resource ?? {}) as AlertRuleFormInput, [resource]);

  const merged = useMemo(
    () => ({
      ...original,
      ...update,
      limitedTo: update.limitedTo ?? original.limitedTo ?? [],
    }),
    [original, update],
  );
  const showThresholdFields =
    merged.type === AlertType.PlatformCpuHigh ||
    merged.type === AlertType.PlatformRamHigh ||
    merged.type === AlertType.PlatformDiskHigh;

  const noCooldownTypes: AlertType[] = [
    AlertType.UnmanagedContainerCreated,
    AlertType.StackAutoUpdated,
    AlertType.StackAutoDeployFailed,
    AlertType.StackServiceAutoUpdated,
    AlertType.StackServiceAutoDeployFailed,
    AlertType.StackDriftAutoReconciled,
    AlertType.DeploymentAutoUpdated,
    AlertType.DeploymentAutoDeployFailed,
    AlertType.SwarmServiceOperationFailed,
    AlertType.AutomationActionRunFailed,
    AlertType.BuildRunFailed,
    AlertType.LicenseEnteredGracePeriod,
    AlertType.LicenseExpired,
  ];
  const showCooldown = !!merged.type && !noCooldownTypes.includes(merged.type);

  const resourceFromAlertType = useCallback((): ResourceType => {
    if (!merged.type) return 'Platform';
    if (merged.type.startsWith('AutomationAction')) return 'AutomationAction';
    if (merged.type.startsWith('Webhook')) return 'Webhook';
    if (merged.type.startsWith('License')) return 'License';
    if (merged.type.startsWith('Build')) return 'Build';
    if (merged.type.startsWith('Deployment')) return 'Deployment';
    if (merged.type.startsWith('SwarmService')) return 'SwarmService';
    if (merged.type.includes('Stack')) return 'Stack';
    return 'Platform';
  }, [merged.type]);
  const showScope = !['Webhook', 'License'].includes(resourceFromAlertType());

  const schema = useMemo(
    () => ({
      '': defineSection<AlertRuleFormInput>({
        title: '',
        items: [
          defineGroupField<AlertRuleFormInput>({
            id: 'type',
            label: 'Alert Type',
            items: [
              defineField({
                key: 'type',
                label: 'Alert Type',
                description: 'The type of event that triggers this alert.',
                required: true,
                disabled: mode === 'edit',
                render: (value, set) => (
                  <ItemSelector
                    collection={AlertType}
                    value={value}
                    disabled={mode === 'edit'}
                    onChange={(v: AlertType) =>
                      set({
                        type: v,
                        threshold: undefined,
                        requiredMatches: undefined,
                        cooldownSeconds: undefined,
                      })
                    }
                  />
                ),
              }),
              ...(mode === 'add'
                ? [
                    defineField<AlertRuleFormInput, 'name'>({
                      key: 'name',
                      label: 'Name',
                      required: true,
                      description: 'A human-readable label for the alert rule.',
                      validate: (v) => (!v ? 'Name is required' : null),
                      render: (val, set) => (
                        <FieldInput
                          value={val}
                          onChange={(v) => set({ name: v })}
                          placeholder="e.g. CPU > 90% – Platform"
                        />
                      ),
                    }),
                  ]
                : []),
            ],
          }),
          defineField({
            label: 'Status',
            key: 'status',
            description: 'Choose the current state of this rule.',
            render: (value, set) => (
              <ItemSelector
                collection={AlertRuleStatus}
                value={value}
                disabled={disabled || (!hasAdvancedAlerting && !isSystemRule && value === AlertRuleStatus.Disabled)}
                onChange={(v: AlertRuleStatus) => set({ status: v })}
              />
            ),
          }),
          defineGroupField<AlertRuleFormInput>({
            id: 'details',
            label: 'Details',
            items: [
              defineField({
                key: 'severity',
                label: 'Severity',
                description: 'How critical this alert is.',
                required: true,
                render: (val, set) => (
                  <ItemSelector
                    collection={AlertSeverity}
                    value={val}
                    disabled={advancedFieldsDisabled}
                    onChange={(v: AlertSeverity) => set({ severity: v })}
                  />
                ),
              }),

              ...(showCooldown
                ? [
                    defineField<AlertRuleFormInput, 'cooldownSeconds'>({
                      key: 'cooldownSeconds',
                      label: 'Cooldown (seconds)',
                      description: 'Minimum seconds between repeated alerts of this type.',
                      render: (val, set) => (
                        <FieldInput
                          type="number"
                          value={val ?? ''}
                          placeholder="e.g. 300"
                          disabled={advancedFieldsDisabled}
                          onChange={(v) => {
                            const n = Number(v);
                            if (v === '' || v === undefined) set({ cooldownSeconds: v });
                            else set({ cooldownSeconds: Math.max(0, Math.min(604800, n)) });
                          }}
                        />
                      ),
                    }),
                  ]
                : []),
              ...(showThresholdFields
                ? [
                    defineField<AlertRuleFormInput, 'threshold'>({
                      key: 'threshold',
                      label: 'Threshold',
                      required: true,
                      description: 'The percentage that must be exceeded to trigger the alert.',
                      validate: (v) => (!v || v.length === 0 ? 'must be greater than 0' : null),
                      render: (val, set) => (
                        <FieldSlider
                          value={typeof val === 'number' ? val : Number(val) || 0}
                          min={0}
                          max={100}
                          step={1}
                          unit="%"
                          disabled={advancedFieldsDisabled}
                          onChange={(v) => set({ threshold: v })}
                        />
                      ),
                    }),
                    defineField<AlertRuleFormInput, 'requiredMatches'>({
                      key: 'requiredMatches',
                      label: 'Required Matches',
                      required: true,
                      description: 'Number of consecutive matches before the alert fires.',
                      validate: (v) => (!v || v.length === 0 ? 'must be greater than 0' : null),
                      render: (val, set) => (
                        <FieldSlider
                          value={typeof val === 'number' ? val : Number(val) || 0}
                          min={0}
                          max={100}
                          step={1}
                          unit=""
                          disabled={advancedFieldsDisabled}
                          onChange={(v) => set({ requiredMatches: v })}
                        />
                      ),
                    }),
                  ]
                : []),
            ],
          }),

          ...(showScope
            ? [
                defineGroupField<AlertRuleFormInput>({
                  id: 'scope',
                  label: 'Scope',
                  items: [
                    defineField({
                      label: 'Applies to',
                      key: 'limitedTo',
                      description: `Optionally limit this alert rule to specific ${resourceFromAlertType().toLocaleLowerCase()}s.`,
                      render: (value, set) => (
                        <MultiResourceSelectorField
                          targetType={LookupResourceType[resourceFromAlertType() as keyof typeof LookupResourceType]}
                          sourceType={LookupResourceType.Alert}
                          sourceResourceId={mode == 'add' ? undefined : id}
                          selected={
                            (value as Array<string | AlertResourceScope> | undefined)?.map((item) =>
                              typeof item === 'string' ? item : item.resourceId,
                            ) ?? []
                          }
                          disabled={advancedFieldsDisabled}
                          onSelect={(v: DeploymentView[] | undefined) =>
                            set(() => ({
                              limitedTo:
                                v?.map((d) => ({
                                  resourceType: resourceFromAlertType() as AlertResourceType,
                                  resourceId: d.id,
                                })) ?? [],
                            }))
                          }
                          placeholder={`Select resources`}
                        />
                      ),
                    }),
                  ],
                }),
              ]
            : []),
          defineGroupField<AlertRuleFormInput>({
            id: 'channels',
            label: 'Channels',
            items: [
              defineField({
                label: 'Notification Channels',
                key: 'channelIds',
                description: 'Select which notification channels will receive this alert.',
                render: (value, set) => (
                  <MultiResourceSelectorField
                    targetType={LookupResourceType.AlertChannel}
                    sourceType={LookupResourceType.Alert}
                    sourceResourceId={mode == 'add' ? undefined : id}
                    selected={(value as string[]) ?? []}
                    disableUnselectedOptions={!hasAdvancedAlerting && !isSystemRule}
                    onSelect={(v: any[] | undefined) =>
                      set(() => ({
                        channelIds: v?.map((c) => c.id) ?? [],
                      }))
                    }
                    placeholder="Select channels"
                  />
                ),
              }),
            ],
          }),
          defineGroupField<AlertRuleFormInput>({
            id: 'quietHours',
            label: 'Quiet Hours',
            items: [
              defineField({
                label: 'Quiet Hours',
                key: 'quietHours',
                description: `Suppress alerts during scheduled maintenance windows.`,
                render: (value, set) => (
                  <QuietHoursField
                    quietHours={value ?? []}
                    disabled={advancedFieldsDisabled}
                    onChange={(next) => set({ quietHours: next })}
                  />
                ),
              }),
            ],
          }),
        ],
      }),
    }),
    [
      advancedFieldsDisabled,
      disabled,
      hasAdvancedAlerting,
      id,
      isSystemRule,
      mode,
      resourceFromAlertType,
      showCooldown,
      showScope,
      showThresholdFields,
    ],
  );

  return (
    <div className="flex flex-col gap-3">
      {!hasAdvancedAlerting ? (
        <AlertMessage type="info" title="Advanced alerting">
          <LicensedFeatureDescription requiredLicense="Team">
            Create custom alert rules and configure advanced conditions. Existing rules can still be disabled or
            deleted.
          </LicensedFeatureDescription>
        </AlertMessage>
      ) : null}
      <FormShell
        mode={mode}
        schema={schema}
        original={original}
        update={update}
        setUpdate={setUpdate}
        onSave={handleSave}
        pending={isPending}
        disabled={formDisabled}
        draftKey={`AlertRule:${id ?? 'new'}`}
        draftVersion={1}
      />
    </div>
  );
};

type QuietHoursFieldProps = {
  quietHours: AlertQuietHour[];
  onChange: (value: AlertQuietHour[]) => void;
};

const SCHEDULE_TYPE_OPTIONS = [
  { value: 'Daily', label: 'Daily' },
  { value: 'Weekly', label: 'Weekly' },
];

const DAY_OF_WEEK_OPTIONS = Object.values(DayOfWeek).map((day) => ({
  value: day,
  label: day,
}));

const toMinutes = (time: string) => {
  const match = time.match(/^([01]\d|2[0-3]):([0-5]\d)$/);
  if (!match) return null;
  return Number(match[1]) * 60 + Number(match[2]);
};

const createDailyDraft = (): AlertQuietHour => ({
  $type: 'Daily',
  name: '',
  startTime: '00:00',
  endTime: '01:00',
  timezone: getDefaultTimezone(),
  description: null,
});

const QuietHoursField = ({ quietHours, disabled, onChange }: QuietHoursFieldProps & { disabled?: boolean }) => {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [formState, setFormState] = useState<AlertQuietHour>(createDailyDraft);
  const [editIndex, setEditIndex] = useState<number | null>(null);

  const changeScheduleType = useCallback((scheduleType: AlertQuietHour['$type']) => {
    setFormState((prev) =>
      scheduleType === 'Weekly'
        ? {
            $type: 'Weekly',
            dayOfWeek: prev.$type === 'Weekly' ? prev.dayOfWeek : DayOfWeek.Monday,
            name: prev.name,
            startTime: prev.startTime,
            endTime: prev.endTime,
            timezone: prev.timezone,
            description: prev.description,
          }
        : {
            $type: 'Daily',
            name: prev.name,
            startTime: prev.startTime,
            endTime: prev.endTime,
            timezone: prev.timezone,
            description: prev.description,
          },
    );
  }, []);

  const resetForm = useCallback(() => {
    setFormState(createDailyDraft());
  }, []);

  const handleDialogClose = useCallback(() => {
    setDialogOpen(false);
    setEditIndex(null);
    resetForm();
  }, [resetForm]);

  const handleDialogOpen = () => {
    resetForm();
    setEditIndex(null);
    setDialogOpen(true);
  };

  const handleSave = () => {
    const trimmedName = formState.name.trim();
    const trimmedTimezone = formState.timezone.trim();
    const startMinutes = toMinutes(formState.startTime);
    const endMinutes = toMinutes(formState.endTime);
    if (!trimmedName || !trimmedTimezone || startMinutes === null || endMinutes === null || startMinutes >= endMinutes)
      return;

    const nextEntry: AlertQuietHour =
      formState.$type === 'Weekly'
        ? {
            $type: 'Weekly',
            name: trimmedName,
            dayOfWeek: formState.dayOfWeek,
            startTime: formState.startTime,
            endTime: formState.endTime,
            timezone: trimmedTimezone,
            description: formState.description?.trim() || null,
          }
        : {
            $type: 'Daily',
            name: trimmedName,
            startTime: formState.startTime,
            endTime: formState.endTime,
            timezone: trimmedTimezone,
            description: formState.description?.trim() || null,
          };

    if (editIndex === null) {
      onChange([...quietHours, nextEntry]);
    } else {
      const next = [...quietHours];
      next[editIndex] = nextEntry;
      onChange(next);
    }
    handleDialogClose();
  };

  const handleDialogOpenChange = (open: boolean) => {
    if (!open) {
      setDialogOpen(false);
      resetForm();
    }
  };

  const handleRemove = (index: number) => {
    const next = quietHours.filter((_, idx) => idx !== index);
    onChange(next);
  };

  const handleEdit = (entry: AlertQuietHour, index: number) => {
    const normalizeTime = (t: string) => t.substring(0, 5);
    setFormState(
      entry.$type === 'Weekly'
        ? {
            $type: 'Weekly',
            dayOfWeek: entry.dayOfWeek,
            name: entry.name,
            startTime: normalizeTime(entry.startTime),
            endTime: normalizeTime(entry.endTime),
            timezone: entry.timezone,
            description: entry.description,
          }
        : {
            $type: 'Daily',
            name: entry.name,
            startTime: normalizeTime(entry.startTime),
            endTime: normalizeTime(entry.endTime),
            timezone: entry.timezone,
            description: entry.description,
          },
    );
    setEditIndex(index);
    setDialogOpen(true);
  };

  const timeValidationError = useMemo(() => {
    const startMinutes = toMinutes(formState.startTime);
    const endMinutes = toMinutes(formState.endTime);
    if (startMinutes === null || endMinutes === null) return 'Start time and end time must be valid.';
    if (startMinutes === endMinutes) return 'Start time must be different from end time.';
    if (startMinutes > endMinutes) return 'Start time must be earlier than end time.';
    return null;
  }, [formState]);

  const canSave = Boolean(formState.name.trim() && formState.timezone.trim() && !timeValidationError);

  return (
    <div className="space-y-3">
      <Button
        variant="outline"
        className="w-full max-w-100 flex flex-row gap-2"
        disabled={disabled}
        onClick={handleDialogOpen}>
        <Plus className="h-3 w-3" /> Add Window
      </Button>
      {quietHours.length > 0 && (
        <QuietHoursTable quietHours={quietHours} disabled={disabled} onEdit={handleEdit} onDelete={handleRemove} />
      )}

      <Dialog open={dialogOpen} onOpenChange={handleDialogOpenChange}>
        <DialogContent className="sm:max-w-170">
          <DialogHeader>
            <DialogTitle>{editIndex === null ? 'Add Maintenance Window' : 'Edit Maintenance Window'}</DialogTitle>
            <DialogDescription>
              {editIndex === null
                ? 'Configure a quiet hour window for this alert rule.'
                : 'Update the selected quiet hour window for this alert rule.'}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label>Window Name</Label>
              <Input
                value={formState.name}
                placeholder="e.g. Backup db"
                onChange={(event) => setFormState((prev) => ({ ...prev, name: event.target.value }))}
              />
            </div>
            <div className="space-y-2">
              <Label>Schedule Type</Label>
              <SelectField
                value={formState.$type === 'Weekly' ? 'Weekly' : 'Daily'}
                onChange={(value) => changeScheduleType(value as AlertQuietHour['$type'])}
                options={SCHEDULE_TYPE_OPTIONS}
                placeholder="Schedule Type"
                allLabel="Schedule Type"
                selectableLabel={false}
              />
            </div>
            {formState.$type === 'Weekly' && (
              <div className="space-y-2">
                <Label>Day of Week</Label>
                <SelectField
                  value={formState.dayOfWeek}
                  onChange={(value) =>
                    setFormState((prev) =>
                      prev.$type === 'Weekly' ? { ...prev, dayOfWeek: value as DayOfWeek } : prev,
                    )
                  }
                  options={DAY_OF_WEEK_OPTIONS}
                  placeholder="Day of Week"
                  allLabel="Day of Week"
                  selectableLabel={false}
                />
              </div>
            )}
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label>Start Time</Label>
                <Input
                  type="time"
                  value={formState.startTime}
                  onChange={(event) => setFormState((prev) => ({ ...prev, startTime: event.target.value }))}
                />
              </div>
              <div className="space-y-2">
                <Label>End Time</Label>
                <Input
                  type="time"
                  value={formState.endTime}
                  onChange={(event) => setFormState((prev) => ({ ...prev, endTime: event.target.value }))}
                />
              </div>
            </div>
            {timeValidationError && <p className="text-sm text-destructive">{timeValidationError}</p>}
            <div className="space-y-2">
              <Label>Timezone</Label>
              <TimezoneSelectField
                value={formState.timezone}
                onChange={(value) => setFormState((prev) => ({ ...prev, timezone: value }))}
              />
            </div>
            <div className="space-y-2">
              <Label>Description</Label>
              <Textarea
                value={formState.description ?? ''}
                className="focus-visible:ring-0"
                onChange={(event) => setFormState((prev) => ({ ...prev, description: event.target.value || null }))}
                rows={3}
              />
            </div>
          </div>
          <DialogFooter className="justify-end space-x-2">
            <Button variant="outline" onClick={handleDialogClose}>
              Cancel
            </Button>
            <Button onClick={handleSave} disabled={!canSave}>
              {editIndex === null ? 'Save' : 'Update'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};

type QuietHourRow = {
  id: string;
  index: number;
  quietHour: AlertQuietHour;
};

const QuietHoursTable = ({
  quietHours,
  disabled,
  onEdit,
  onDelete,
}: {
  quietHours: AlertQuietHour[];
  disabled?: boolean;
  onEdit: (entry: AlertQuietHour, index: number) => void;
  onDelete: (index: number) => void;
}) => {
  const rows = useMemo<QuietHourRow[]>(
    () =>
      quietHours.map((entry, index) => ({
        id: `${index}-${entry.name}-${entry.$type}-${entry.startTime}-${entry.endTime}-${entry.timezone}`,
        index,
        quietHour: entry,
      })),
    [quietHours],
  );

  const columns = useMemo<ColumnDef<QuietHourRow>[]>(
    () => [
      {
        header: 'Name',
        cell: ({ row }) => row.original.quietHour.name,
      },
      {
        header: 'Schedule',
        cell: ({ row }) => {
          const entry = row.original.quietHour;
          const scheduleLabel =
            entry.$type;
          return entry.$type === 'Weekly' && entry.dayOfWeek ? `${scheduleLabel} - ${entry.dayOfWeek}` : scheduleLabel;
        },
      },
      {
        header: 'Start',
        cell: ({ row }) => row.original.quietHour.startTime?.substring(0, 5),
      },
      {
        header: 'End',
        cell: ({ row }) => row.original.quietHour.endTime?.substring(0, 5),
      },
      {
        header: 'Timezone',
        cell: ({ row }) => row.original.quietHour.timezone,
      },
      {
        header: 'Description',
        cell: ({ row }) => {
          const description = row.original.quietHour.description ?? '-';
          return (
            <div className="max-w-62.5">
              <span className="block truncate" title={description}>
                {description}
              </span>
            </div>
          );
        },
      },
      {
        id: 'actions',
        header: () => <div className="text-center">Actions</div>,
        cell: ({ row }) => {
          const index = row.original.index;
          return (
            <div className="flex items-center justify-end gap-1">
              <Button
                variant="ghost"
                size="icon"
                disabled={disabled}
                onClick={() => onEdit(row.original.quietHour, index)}>
                <Pencil className="size-4 text-muted-foreground" />
              </Button>
              <Button variant="ghost" size="icon" disabled={disabled} onClick={() => onDelete(index)}>
                <Trash2 className="size-4 text-muted-foreground" />
              </Button>
            </div>
          );
        },
      },
    ],
    [disabled, onDelete, onEdit],
  );

  return <DataTable columns={columns} data={rows} isLoading={false} getRowId={(row) => row.id} />;
};
