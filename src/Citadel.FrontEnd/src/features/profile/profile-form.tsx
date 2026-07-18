import {
  CurrentProfileView,
  ProfileResourceInfoView,
  UserDateTimeFormat,
  UserPreferencesView,
  UserSessionSummaryView,
  UserTheme,
} from '@/api/generated/api.types';
import { SelectField } from '@/components/custom/common';
import {
  defineField,
  defineGroupField,
  defineSection,
  FieldChange,
  FieldInput,
  FormShell,
} from '@/components/custom/form-builder';
import { getBrowserTimezone, TimezoneSelectField } from '@/components/custom/timezone-select';
import { Badge } from '@/components/ui/badge';
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
import { Skeleton } from '@/components/ui/skeleton';
import { Constants } from '@/lib/constants';
import { useLayoutContext } from '@/lib/context/layout-context';
import { useMutate, useRead } from '@/lib/hooks';
import { toThemeMode } from '@/lib/theme-preferences';
import { useQueryClient } from '@tanstack/react-query';
import { Clock, KeyRound, Laptop, MapPin, ShieldCheck, Users } from 'lucide-react';
import { FormEvent, ReactNode, useEffect, useMemo, useState } from 'react';
import { toast } from 'sonner';
import { ProfileDateFormatter } from './utils';
import { ProfileMfaCommand } from './mfa/profile-mfa';

const DATE_TIME_FORMAT_OPTIONS = [
  { value: UserDateTimeFormat.System, label: 'System' },
  { value: UserDateTimeFormat.TwentyFourHour, label: '24-hour' },
  { value: UserDateTimeFormat.TwelveHour, label: '12-hour' },
];

const THEME_OPTIONS = [
  { value: UserTheme.System, label: 'System' },
  { value: UserTheme.Light, label: 'Light' },
  { value: UserTheme.Dark, label: 'Dark' },
];

type ProfileFormValue = {
  displayName: string;
  email: string;
  authenticationLabel: string;
  oidcProviderName: string;
  roles: ProfileResourceInfoView[];
  teams: ProfileResourceInfoView[];
  timeZone: string;
  dateTimeFormat: UserDateTimeFormat;
  theme: UserTheme;
  passwordCommands: string;
  mfaCommands: string;
  sessionCommands: string;
};

type ConfirmSessionAction = { type: 'session'; session: UserSessionSummaryView } | { type: 'other-sessions' };

export function ProfileForm({
  profile,
  preferences,
  formatDate,
}: {
  profile: CurrentProfileView;
  preferences?: UserPreferencesView;
  formatDate: ProfileDateFormatter;
}) {
  const queryClient = useQueryClient();
  const { setThemeMode } = useLayoutContext();
  const updateProfile = useMutate('updateCurrentProfile');
  const patchPreferences = useMutate('patchProfilePreferences');
  const [update, setUpdate] = useState<Partial<ProfileFormValue>>({});

  const original = useMemo<ProfileFormValue>(
    () => ({
      displayName: profile.displayName,
      email: profile.email,
      authenticationLabel: profile.authentication.label,
      oidcProviderName: profile.authentication.oidcProviderName ?? '-',
      roles: profile.directRoles,
      teams: profile.teams,
      timeZone: preferences?.timeZone ?? getBrowserTimezone(),
      dateTimeFormat: preferences?.dateTimeFormat ?? UserDateTimeFormat.System,
      theme: preferences?.theme ?? UserTheme.System,
      passwordCommands: '',
      mfaCommands: '',
      sessionCommands: '',
    }),
    [preferences, profile],
  );

  useEffect(() => {
    setThemeMode(toThemeMode(original.theme));
  }, [original.theme, setThemeMode]);

  const handleSave = async (payload: ProfileFormValue) => {
    const displayName = payload.displayName.trim();
    const profileChanged = displayName !== original.displayName;
    const preferencesChanged =
      payload.timeZone !== original.timeZone ||
      payload.dateTimeFormat !== original.dateTimeFormat ||
      payload.theme !== original.theme;

    if (profileChanged) {
      await updateProfile.mutateAsync({ data: { displayName } });
    }

    if (preferencesChanged) {
      await patchPreferences.mutateAsync({
        data: {
          timeZone: payload.timeZone,
          dateTimeFormat: payload.dateTimeFormat,
          theme: payload.theme,
        },
      });
    }

    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['getCurrentProfile'] }),
      queryClient.invalidateQueries({ queryKey: ['getProfilePreferences'] }),
    ]);

    toast.success('Profile saved.');
  };

  const schema = useMemo(
    () => ({
      '': defineSection<ProfileFormValue>({
        title: '',
        items: [
          defineGroupField<ProfileFormValue>({
            id: 'profile',
            label: 'Profile',
            title: 'Profile',
            description: 'Your account identity and access information.',
            items: [
              defineField({
                key: 'displayName',
                label: 'Display Name',
                required: true,
                description: 'Your display name must be unique and use the same naming rules as users.',
                validate: (value) =>
                  !new RegExp(Constants.validNameIdentifier).test(value ?? '') ? 'Invalid name format' : null,
                render: (value, set) => (
                  <FieldInput value={value ?? ''} onChange={(displayName) => set({ displayName })} />
                ),
              }),
              defineField({
                key: 'email',
                label: 'Email',
                description: 'Used to sign in and recover your account.',
                render: (value) => <ReadOnlyValue value={value} />,
              }),
              defineField({
                key: 'authenticationLabel',
                label: 'Authentication',
                description: 'How you currently sign in to Citadel.',
                render: (value) => (
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge variant="secondary" className="rounded-sm bg-accent/60">
                      {value}
                    </Badge>
                    {profile.authentication.oidcProviderName && (
                      <Badge variant="outline" className="rounded-sm bg-accent/60">
                        {profile.authentication.oidcProviderName}
                      </Badge>
                    )}
                  </div>
                ),
              }),
              defineField({
                key: 'roles',
                label: 'Direct Roles',
                description: 'Roles assigned directly to you.',
                render: (value) => <ResourceBadges items={value ?? []} icon={<ShieldCheck className="size-3.5" />} />,
              }),
              defineField({
                key: 'teams',
                label: 'Teams',
                description: "Teams you're a member of.",
                render: (value) => <ResourceBadges items={value ?? []} icon={<Users className="size-3.5" />} />,
              }),
            ],
          }),
          defineGroupField<ProfileFormValue>({
            id: 'preferences',
            label: 'Preferences',
            title: 'Preferences',
            description: 'How Citadel displays information for you.',
            items: [
              defineField({
                key: 'timeZone',
                label: 'Time Zone',
                required: true,
                description: 'Used when Citadel displays dates and times for you.',
                render: (value, set) => (
                  <TimezoneSelectField
                    value={value ?? getBrowserTimezone()}
                    onChange={(timeZone) => set({ timeZone })}
                    className="w-100"
                  />
                ),
              }),
              defineField({
                key: 'dateTimeFormat',
                label: 'Date and Time',
                description: 'Choose how clock values are shown in your profile and session views.',
                render: (value, set) => (
                  <SelectField
                    value={value ?? UserDateTimeFormat.System}
                    onChange={(dateTimeFormat) => set({ dateTimeFormat: dateTimeFormat as UserDateTimeFormat })}
                    options={DATE_TIME_FORMAT_OPTIONS}
                    placeholder="Date and time format"
                    allLabel="Date and time format"
                    selectableLabel={false}
                    className="max-w-100"
                  />
                ),
              }),
              defineField({
                key: 'theme',
                label: 'Theme',
                description: 'The selected theme is previewed immediately in this browser.',
                render: (value, set) => (
                  <ThemeField
                    value={value ?? UserTheme.System}
                    set={set}
                    onPreview={(theme) => setThemeMode(toThemeMode(theme))}
                  />
                ),
              }),
            ],
          }),
          defineGroupField<ProfileFormValue>({
            id: 'security',
            label: 'Security',
            title: 'Security',
            description: 'Your password and active browser sessions.',
            items: [
              defineField({
                key: 'passwordCommands',
                label: 'Password',
                ignoreFormDisabled: true,
                description: profile.authentication.canChangePassword
                  ? 'Changing your password signs out other Citadel sessions.'
                  : 'Your password is managed by your identity provider.',
                render: () =>
                  profile.authentication.canChangePassword ? (
                    <PasswordCommand />
                  ) : (
                    <p className="text-sm text-muted-foreground">
                      Password changes are not available for your account.
                    </p>
                  ),
              }),
              defineField({
                key: 'mfaCommands',
                label: 'Two-factor authentication',
                ignoreFormDisabled: true,
                description: 'Protect local-password sign-in with an authenticator app and recovery codes.',
                render: () => <ProfileMfaCommand canUseLocalPassword={profile.authentication.canUseLocalPasswordMfa} />,
              }),
              defineField({
                key: 'sessionCommands',
                label: 'Active Sessions',
                ignoreFormDisabled: true,
                description: 'Browser sessions that can still refresh your access.',
                render: () => <SessionsCommand formatDate={formatDate} />,
              }),
            ],
          }),
        ],
      }),
    }),
    [
      formatDate,
      profile.authentication.canUseLocalPasswordMfa,
      profile.authentication.canChangePassword,
      profile.authentication.oidcProviderName,
      setThemeMode,
    ],
  );

  return (
    <FormShell
      mode="edit"
      schema={schema}
      original={original}
      update={update}
      setUpdate={setUpdate}
      onSave={handleSave}
      pending={updateProfile.isPending || patchPreferences.isPending}
      draftKey="profile:self"
      draftVersion={1}
      onReset={() => {
        setUpdate({});
        setThemeMode(toThemeMode(original.theme));
      }}
    />
  );
}

function ThemeField({
  value,
  set,
  onPreview,
}: {
  value: UserTheme;
  set: FieldChange<ProfileFormValue>;
  onPreview: (theme: UserTheme) => void;
}) {
  return (
    <SelectField
      value={value}
      onChange={(theme) => {
        const next = theme as UserTheme;
        set({ theme: next });
        onPreview(next);
      }}
      options={THEME_OPTIONS}
      placeholder="Theme"
      allLabel="Theme"
      selectableLabel={false}
      className="max-w-100"
    />
  );
}

function ReadOnlyValue({ value }: { value?: string | null }) {
  return (
    <div className="flex min-h-9 max-w-100 items-center rounded-sm border bg-muted/30 px-3 py-2 text-sm text-muted-foreground">
      <span className="truncate">{value || '-'}</span>
    </div>
  );
}

function ResourceBadges({ items, icon }: { items: ProfileResourceInfoView[]; icon: ReactNode }) {
  if (!items.length) return <p className="text-sm text-muted-foreground">None assigned.</p>;

  return (
    <div className="flex min-w-0 flex-wrap gap-1.5">
      {items.map((item) => (
        <Badge key={item.id} variant="secondary" className="max-w-full rounded-sm font-semibold bg-accent/60">
          {icon}
          <span className="truncate">{item.name}</span>
        </Badge>
      ))}
    </div>
  );
}

function PasswordCommand() {
  const queryClient = useQueryClient();
  const changePassword = useMutate('changeCurrentPassword');
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');

  const validationError = useMemo(() => {
    if (!newPassword || !confirmPassword) return undefined;
    return newPassword === confirmPassword ? undefined : 'New password and confirmation do not match.';
  }, [newPassword, confirmPassword]);

  const canSubmit = Boolean(currentPassword && newPassword && confirmPassword && !validationError);

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    if (!canSubmit) return;

    await changePassword.mutateAsync({
      data: {
        currentPassword,
        newPassword,
      },
    });

    setCurrentPassword('');
    setNewPassword('');
    setConfirmPassword('');
    await queryClient.invalidateQueries({ queryKey: ['listProfileSessions'] });
    toast.success('Password changed.');
  };

  return (
    <form className="grid max-w-100 gap-3" onSubmit={handleSubmit}>
      <Input
        type="password"
        value={currentPassword}
        placeholder="Current password"
        autoComplete="current-password"
        onChange={(event) => setCurrentPassword(event.target.value)}
      />
      <Input
        type="password"
        value={newPassword}
        placeholder="New password"
        autoComplete="new-password"
        onChange={(event) => setNewPassword(event.target.value)}
      />
      <Input
        type="password"
        value={confirmPassword}
        placeholder="Confirm new password"
        autoComplete="new-password"
        onChange={(event) => setConfirmPassword(event.target.value)}
      />

      {(validationError || changePassword.validationErrors) && (
        <p className="text-sm text-destructive">{validationError ?? changePassword.validationErrors}</p>
      )}

      <div className="flex justify-end">
        <Button type="submit" disabled={!canSubmit || changePassword.isPending}>
          Change Password
        </Button>
      </div>
    </form>
  );
}

function SessionsCommand({ formatDate }: { formatDate: ProfileDateFormatter }) {
  const queryClient = useQueryClient();
  const sessionsQuery = useRead('listProfileSessions');
  const sessions = sessionsQuery.data?.data;
  const revokeSession = useMutate('revokeProfileSession');
  const revokeOtherSessions = useMutate('revokeOtherProfileSessions');
  const [confirmAction, setConfirmAction] = useState<ConfirmSessionAction | null>(null);

  const handleConfirm = async () => {
    if (!confirmAction) return;

    if (confirmAction.type === 'session') {
      await revokeSession.mutateAsync({ sessionId: confirmAction.session.id });
      toast.success('Session revoked.');
    } else {
      const response = await revokeOtherSessions.mutateAsync({});
      toast.success(`${response.data.count} session${Number(response.data.count) === 1 ? '' : 's'} revoked.`);
    }

    setConfirmAction(null);
    await queryClient.invalidateQueries({ queryKey: ['listProfileSessions'] });
  };

  return (
    <div className="max-full rounded-sm border">
      <div className="flex flex-col gap-3 border-b px-3 py-2 sm:flex-row sm:items-center sm:justify-between">
        <div className="text-sm text-muted-foreground">Manage your active browser sessions.</div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={!sessions?.canRevokeOtherSessions || revokeOtherSessions.isPending}
          onClick={() => setConfirmAction({ type: 'other-sessions' })}>
          Sign Out Others
        </Button>
      </div>

      {sessionsQuery.isLoading ? (
        <div className="p-3">
          <Skeleton className="h-24 w-full" />
        </div>
      ) : sessions?.sessions.length ? (
        <div className="divide-y">
          {sessions.sessions.map((session) => (
            <SessionRow
              key={session.id}
              session={session}
              formatDate={formatDate}
              disabled={revokeSession.isPending}
              onRevoke={() => setConfirmAction({ type: 'session', session })}
            />
          ))}
        </div>
      ) : (
        <p className="p-3 text-sm text-muted-foreground">No active sessions found.</p>
      )}

      <ConfirmSessionDialog
        action={confirmAction}
        pending={revokeSession.isPending || revokeOtherSessions.isPending}
        onOpenChange={(open) => !open && setConfirmAction(null)}
        onConfirm={handleConfirm}
      />
    </div>
  );
}

function SessionRow({
  session,
  formatDate,
  disabled,
  onRevoke,
}: {
  session: UserSessionSummaryView;
  formatDate: ProfileDateFormatter;
  disabled: boolean;
  onRevoke: () => void;
}) {
  return (
    <div className="flex flex-col gap-3 px-3 py-2 sm:flex-row sm:items-start sm:justify-between">
      <div className="flex min-w-0 gap-3">
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-sm bg-accent/60 text-muted-foreground">
          <Laptop className="size-4" />
        </div>
        <div className="min-w-0">
          <div className="flex min-w-0 flex-wrap items-center gap-2">
            <span className="min-w-0 truncate text-sm font-medium">{session.displayName}</span>
            {session.isCurrent && (
              <Badge variant="secondary" className="shrink-0 rounded-sm bg-accent/60">
                Current
              </Badge>
            )}
          </div>
          <div className="mt-2 grid gap-2 text-xs text-muted-foreground md:grid-cols-3">
            <SessionMeta icon={<MapPin className="size-3.5" />} label={session.ipAddress ?? 'Unknown'} />
            <SessionMeta icon={<Clock className="size-3.5" />} label={formatDate(session.lastSeenAt)} />
            <SessionMeta icon={<KeyRound className="size-3.5" />} label={formatDate(session.expiresAt)} />
          </div>
        </div>
      </div>
      <Button type="button" variant="outline" size="sm" disabled={session.isCurrent || disabled} onClick={onRevoke}>
        Revoke
      </Button>
    </div>
  );
}

function SessionMeta({ icon, label }: { icon: ReactNode; label: string }) {
  return (
    <div className="flex min-w-0 items-center gap-1.5">
      <span className="shrink-0">{icon}</span>
      <span className="truncate" title={label}>
        {label}
      </span>
    </div>
  );
}

function ConfirmSessionDialog({
  action,
  pending,
  onOpenChange,
  onConfirm,
}: {
  action: ConfirmSessionAction | null;
  pending: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
}) {
  const title = action?.type === 'session' ? 'Revoke Session' : 'Sign Out Other Sessions';
  const description =
    action?.type === 'session'
      ? `Revoke "${action.session.displayName}" from your account.`
      : 'Revoke every other active Citadel session for your account.';

  return (
    <Dialog open={Boolean(action)} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={pending}>
            Cancel
          </Button>
          <Button onClick={onConfirm} disabled={pending}>
            Confirm
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
