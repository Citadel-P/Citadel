import {
  AlertRuleInput,
  AlertResourceType,
  AlertRuleQuietHour,
  AlertRuleQuietHourDailyQuietHour,
  AlertRuleQuietHourWeeklyQuietHour,
  AlertType,
  AlertSeverity,
  DayOfWeek,
  ScheduleType,
  DeploymentView,
} from '@/api/generated/api.types';
import {
  FormShell,
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldSlider,
  FieldSwitch,
  ItemSelector,
} from '@/components/custom/form-builder';
import { useState, useMemo, useCallback, useEffect } from 'react';
import { useMutate } from '@/lib/hooks';
import { toast } from 'sonner';
import { useParams, useNavigate } from 'react-router';
import { MultiResourceSelectorField, SelectField } from '@/components/custom/common';
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
import { ContentCard } from '@/components/custom/content-card';
import { DataTable } from '@/components/ui/data-table';
import { ColumnDef } from '@tanstack/react-table';

type LimitedToEntry = {
  resourceType: AlertResourceType;
  resourceId: string;
};

export const AlertRuleForm = ({ mode, resource }: { mode: 'add' | 'edit'; resource?: AlertRuleInput }) => {
  const id = useParams().id;
  const navigate = useNavigate();
  const [update, setUpdate] = useState<Partial<AlertRuleInput>>({});
  const [isPending, setIsPending] = useState(false);

  const { mutateAsync: createAlertRule } = useMutate('createAlertRule');
  const { mutateAsync: updateAlertRule } = useMutate('updateAlertRule');

  const original = useMemo(() => {
    return {
      isEnabled: true,
      requiredMatches: 3,
      ...(resource ?? {}),
    } as AlertRuleInput;
  }, [resource]);

  const handleSave = async (payload: AlertRuleInput) => {
    setIsPending(true);
    try {
      if (mode === 'edit') await updateAlertRule({ id, data: payload });
      else await createAlertRule({ data: payload });

      toast.success(`Alert rule "${payload.type}" saved successfully`);
      navigate('/alerters');
    } finally {
      setIsPending(false);
    }
  };

  const merged = useMemo(() => ({ ...original, ...update }), [original, update]);
  const showThresholdFields = merged.type === AlertType.PlatformCpuHigh || merged.type === AlertType.PlatformRamHigh;

  const noCooldownTypes: AlertType[] = [
    AlertType.UnmanagedContainerCreated,
    AlertType.StackImageUpdateAvailable,
    AlertType.StackAutoUpdated,
    AlertType.StackAutoDeployFailed,
    AlertType.DeploymentImageUpdateAvailable,
    AlertType.DeploymentAutoUpdated,
    AlertType.DeploymentAutoDeployFailed,
  ];
  const showCooldown = !!merged.type && !noCooldownTypes.includes(merged.type);

  const resourceFromAlertType = useCallback((): ResourceType => {
    if (!merged.type) return 'Platform';
    if (merged.type.startsWith('Deployment')) return 'Deployment';
    if (merged.type.includes('Stack')) return 'Stack';
    return 'Platform';
  }, [merged.type]);

  useEffect(() => {
    update.limitedTo = original.limitedTo ?? [];
  }, [resourceFromAlertType, original, update]);

  const schema = useMemo(
    () => ({
      '': defineSection<AlertRuleInput>({
        title: '',
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
          defineField({
            label: 'Enabled',
            key: 'isEnabled',
            description: 'Enable this rule immediately.',
            render: (value, set) => (
              <FieldSwitch
                checked={value ?? false}
                id="isEnabled"
                onChange={(value) =>
                  set(() => ({
                    isEnabled: value,
                  }))
                }
              />
            ),
          }),

          defineGroupField<AlertRuleInput>({
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
                    onChange={(v: AlertSeverity) => set({ severity: v })}
                  />
                ),
              }),

              ...(showCooldown
                ? [
                    defineField<AlertRuleInput, 'cooldownSeconds'>({
                      key: 'cooldownSeconds',
                      label: 'Cooldown (seconds)',
                      description: 'Minimum seconds between repeated alerts of this type.',
                      render: (val, set) => (
                        <FieldInput
                          type="number"
                          value={val ?? ''}
                          placeholder="e.g. 300"
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
                    defineField<AlertRuleInput, 'threshold'>({
                      key: 'threshold',
                      label: 'Threshold',
                      required: true,
                      description: 'The percentage that must be exceeded to trigger the alert.',
                      render: (val, set) => (
                        <FieldSlider
                          value={typeof val === 'number' ? val : Number(val) || 0}
                          min={0}
                          max={100}
                          step={1}
                          unit="%"
                          onChange={(v) => set({ threshold: v })}
                        />
                      ),
                    }),
                    defineField<AlertRuleInput, 'requiredMatches'>({
                      key: 'requiredMatches',
                      label: 'Required Matches',
                      required: true,
                      description: 'Number of consecutive matches before the alert fires.',
                      render: (val, set) => (
                        <FieldSlider
                          value={typeof val === 'number' ? val : Number(val) || 3}
                          min={3}
                          max={100}
                          step={1}
                          unit=""
                          onChange={(v) => set({ requiredMatches: v })}
                        />
                      ),
                    }),
                  ]
                : []),
            ],
          }),

          defineGroupField<AlertRuleInput>({
            id: 'scope',
            label: 'Scope',
            items: [
              defineField({
                label: 'Applies to',
                key: 'limitedTo',
                description: `Optionally limit this alert rule to specific ${resourceFromAlertType().toLocaleLowerCase()}s.`,
                render: (value, set) => (
                  <MultiResourceSelectorField
                    type={resourceFromAlertType()}
                    selected={
                      (value as Array<string | LimitedToEntry> | undefined)?.map((item) =>
                        typeof item === 'string' ? item : item.resourceId,
                      ) ?? []
                    }
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

          defineGroupField<AlertRuleInput>({
            id: 'quietHours',
            label: 'Quiet Hours',
            items: [
              defineField({
                label: 'Quiet Hours',
                key: 'quietHours',
                description: `Suppress alerts during scheduled maintenance windows.`,
                render: (value, set) => (
                  <QuietHoursField quietHours={value ?? []} onChange={(next) => set({ quietHours: next })} />
                ),
              }),
            ],
          }),
        ],
      }),
    }),
    [mode, showThresholdFields, showCooldown, resourceFromAlertType],
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
      draftKey={`alert-rule:${id ?? 'new'}`}
      draftVersion={1}
    />
  );
};

type QuietHoursFieldProps = {
  quietHours: AlertRuleQuietHour[];
  onChange: (value: AlertRuleQuietHour[]) => void;
};

type QuietHourDraft =
  | (AlertRuleQuietHourDailyQuietHour & { $type: 'Daily' })
  | (AlertRuleQuietHourWeeklyQuietHour & { $type: 'Weekly' });

type TimezoneOption = {
  value: string;
  label: string;
};

type TimezoneSeed = TimezoneOption & {
  offsetMinutes: number;
};

const TIMEZONE_SEED: TimezoneSeed[] = [
  { value: 'UTC', label: 'Coordinated Universal Time (UTC +/-00:00)', offsetMinutes: 0 },
  { value: 'Etc/GMT', label: 'Greenwich Mean Time (UTC +/-00:00)', offsetMinutes: 0 },
  { value: 'Europe/Paris', label: 'Central European Time (UTC +01:00)', offsetMinutes: 60 },
  { value: 'Africa/Lagos', label: 'West Africa Time (UTC +01:00)', offsetMinutes: 60 },
  { value: 'Europe/Athens', label: 'Eastern European Time (UTC +02:00)', offsetMinutes: 120 },
  { value: 'Africa/Harare', label: 'Central Africa Time (UTC +02:00)', offsetMinutes: 120 },
  { value: 'Africa/Johannesburg', label: 'South Africa Standard Time (UTC +02:00)', offsetMinutes: 120 },
  { value: 'Europe/Moscow', label: 'Moscow Time (UTC +03:00)', offsetMinutes: 180 },
  { value: 'Asia/Dubai', label: 'Gulf Standard Time (UTC +04:00)', offsetMinutes: 240 },
  { value: 'Asia/Karachi', label: 'Pakistan Time (UTC +05:00)', offsetMinutes: 300 },
  { value: 'Asia/Kolkata', label: 'India Standard Time (UTC +05:30)', offsetMinutes: 330 },
  { value: 'Asia/Bangkok', label: 'Indochina Time (UTC +07:00)', offsetMinutes: 420 },
  { value: 'Asia/Shanghai', label: 'China Standard Time (UTC +08:00)', offsetMinutes: 480 },
  { value: 'Australia/Perth', label: 'Australian Western Standard Time (UTC +08:00)', offsetMinutes: 480 },
  { value: 'Asia/Tokyo', label: 'Japan Standard Time (UTC +09:00)', offsetMinutes: 540 },
  { value: 'Asia/Seoul', label: 'Korea Standard Time (UTC +09:00)', offsetMinutes: 540 },
  { value: 'Australia/Adelaide', label: 'Australian Central Standard Time (UTC +09:30)', offsetMinutes: 570 },
  { value: 'Australia/Sydney', label: 'Australian Eastern Standard Time (UTC +10:00)', offsetMinutes: 600 },
  { value: 'Pacific/Auckland', label: 'New Zealand Standard Time (UTC +12:00)', offsetMinutes: 720 },
  { value: 'America/New_York', label: 'Eastern Standard Time (UTC -05:00)', offsetMinutes: -300 },
  { value: 'America/Chicago', label: 'Central Standard Time (UTC -06:00)', offsetMinutes: -360 },
  { value: 'America/Denver', label: 'Mountain Standard Time (UTC -07:00)', offsetMinutes: -420 },
  { value: 'America/Los_Angeles', label: 'Pacific Standard Time (UTC -08:00)', offsetMinutes: -480 },
  { value: 'America/Anchorage', label: 'Alaska Standard Time (UTC -09:00)', offsetMinutes: -540 },
  { value: 'Pacific/Honolulu', label: 'Hawaii Standard Time (UTC -10:00)', offsetMinutes: -600 },
];

const TIMEZONE_OPTIONS = TIMEZONE_SEED.slice()
  .sort((a, b) => {
    const aKey = a.offsetMinutes >= 0 ? a.offsetMinutes : 10000 + Math.abs(a.offsetMinutes);
    const bKey = b.offsetMinutes >= 0 ? b.offsetMinutes : 10000 + Math.abs(b.offsetMinutes);
    return aKey - bKey || a.label.localeCompare(b.label);
  })
  .map(({ offsetMinutes: _, ...timezone }) => timezone);

const SCHEDULE_TYPE_OPTIONS = [
  { value: ScheduleType.Daily, label: 'Daily' },
  { value: ScheduleType.Weekly, label: 'Weekly' },
];

const DAY_OF_WEEK_OPTIONS = Object.values(DayOfWeek).map((day) => ({
  value: day,
  label: day,
}));

const getDefaultTimezone = () => {
  return TIMEZONE_OPTIONS[0]?.value ?? 'UTC';
};

const toMinutes = (time: string) => {
  const match = time.match(/^([01]\d|2[0-3]):([0-5]\d)$/);
  if (!match) return null;
  return Number(match[1]) * 60 + Number(match[2]);
};

const createDailyDraft = (): QuietHourDraft => ({
  $type: 'Daily',
  scheduleType: ScheduleType.Daily,
  name: '',
  startTime: '00:00',
  endTime: '01:00',
  timezone: getDefaultTimezone(),
  description: null,
});

const QuietHoursField = ({ quietHours, onChange }: QuietHoursFieldProps) => {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [formState, setFormState] = useState<QuietHourDraft>(createDailyDraft);
  const [editIndex, setEditIndex] = useState<number | null>(null);

  const changeScheduleType = useCallback((scheduleType: ScheduleType) => {
    setFormState((prev) =>
      scheduleType === ScheduleType.Weekly
        ? {
            $type: 'Weekly',
            scheduleType: ScheduleType.Weekly,
            dayOfWeek: prev.$type === 'Weekly' ? prev.dayOfWeek : DayOfWeek.Monday,
            name: prev.name,
            startTime: prev.startTime,
            endTime: prev.endTime,
            timezone: prev.timezone,
            description: prev.description,
          }
        : {
            $type: 'Daily',
            scheduleType: ScheduleType.Daily,
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

    const nextEntry: AlertRuleQuietHour =
      formState.$type === 'Weekly'
        ? ({
            $type: 'Weekly',
            name: trimmedName,
            scheduleType: ScheduleType.Weekly,
            dayOfWeek: formState.dayOfWeek,
            startTime: formState.startTime,
            endTime: formState.endTime,
            timezone: trimmedTimezone,
            description: formState.description?.trim() || null,
          } as AlertRuleQuietHour)
        : ({
            $type: 'Daily',
            name: trimmedName,
            scheduleType: ScheduleType.Daily,
            startTime: formState.startTime,
            endTime: formState.endTime,
            timezone: trimmedTimezone,
            description: formState.description?.trim() || null,
          } as AlertRuleQuietHour);

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

  const handleEdit = (entry: AlertRuleQuietHour, index: number) => {
    setFormState(
      entry.$type === 'Weekly'
        ? {
            $type: 'Weekly',
            scheduleType: ScheduleType.Weekly,
            dayOfWeek: entry.dayOfWeek,
            name: entry.name,
            startTime: entry.startTime,
            endTime: entry.endTime,
            timezone: entry.timezone,
            description: entry.description,
          }
        : {
            $type: 'Daily',
            scheduleType: ScheduleType.Daily,
            name: entry.name,
            startTime: entry.startTime,
            endTime: entry.endTime,
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
  }, [formState.startTime, formState.endTime]);

  const canSave = Boolean(formState.name.trim() && formState.timezone.trim() && !timeValidationError);

  return (
    <div className="space-y-3">
      <Button variant="outline" className="w-full max-w-[400px] flex flex-row gap-2" onClick={handleDialogOpen}>
        <Plus className="h-3 w-3" /> Add Window
      </Button>
      {quietHours.length > 0 && (
        <ContentCard>
          <QuietHoursTable quietHours={quietHours} onEdit={handleEdit} onDelete={handleRemove} />
        </ContentCard>
      )}

      <Dialog open={dialogOpen} onOpenChange={handleDialogOpenChange}>
        <DialogContent className="sm:max-w-[680px]">
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
                value={formState.$type === 'Weekly' ? ScheduleType.Weekly : ScheduleType.Daily}
                onChange={(value) => changeScheduleType(value as ScheduleType)}
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
              <SelectField
                value={formState.timezone}
                onChange={(value) => setFormState((prev) => ({ ...prev, timezone: value }))}
                options={TIMEZONE_OPTIONS}
                placeholder="Select timezone"
                allLabel="Timezone"
                selectableLabel={false}
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
  quietHour: AlertRuleQuietHour;
};

const QuietHoursTable = ({
  quietHours,
  onEdit,
  onDelete,
}: {
  quietHours: AlertRuleQuietHour[];
  onEdit: (entry: AlertRuleQuietHour, index: number) => void;
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
            entry.scheduleType ?? (entry.$type === 'Weekly' ? ScheduleType.Weekly : ScheduleType.Daily);
          return entry.$type === 'Weekly' && entry.dayOfWeek ? `${scheduleLabel} - ${entry.dayOfWeek}` : scheduleLabel;
        },
      },
      {
        header: 'Start',
        cell: ({ row }) => row.original.quietHour.startTime,
      },
      {
        header: 'End',
        cell: ({ row }) => row.original.quietHour.endTime,
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
            <div className="max-w-[250px]">
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
              <Button variant="ghost" size="icon" onClick={() => onEdit(row.original.quietHour, index)}>
                <Pencil className="size-4 text-muted-foreground" />
              </Button>
              <Button variant="ghost" size="icon" onClick={() => onDelete(index)}>
                <Trash2 className="size-4 text-muted-foreground" />
              </Button>
            </div>
          );
        },
      },
    ],
    [onDelete, onEdit],
  );

  return <DataTable columns={columns} data={rows} isLoading={false} getRowId={(row) => row.id} />;
};
