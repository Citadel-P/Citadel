import { useSetupContext } from '@/features/setup/setup-context';
import {
  CurrentProfileView,
  UserDateTimeFormat,
  UserPreferencesView,
  UserSessionSummaryView,
} from '@/api/generated/api.types';
import { SelectField } from '@/components/custom/common';
import { defineField, defineGroupField, defineSection, FieldInput, FormShell } from '@/components/custom/form-builder';
import { getBrowserTimezone, TimezoneSelectField } from '@/components/custom/timezone-select';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Skeleton } from '@/components/ui/skeleton';
import { Constants } from '@/lib/constants';
import { AppearanceCustomizer } from '@/components/custom/appearance-customizer';
import { useMutate, useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { ChevronDown, Clock, KeyRound, Laptop, MapPin, RefreshCw } from 'lucide-react';
import { FormEvent, ReactNode, useMemo, useState } from 'react';
import { toast } from 'sonner';
import { ProfileDateFormatter } from './utils';
import { ProfileMfaCommand, ProfileMfaStatusBadge } from './mfa/profile-mfa';

const DATE_TIME_FORMAT_OPTIONS = [
  { value: UserDateTimeFormat.System, label: 'System' },
  { value: UserDateTimeFormat.TwentyFourHour, label: '24-hour' },
  { value: UserDateTimeFormat.TwelveHour, label: '12-hour' },
];

type ProfileFormValue = {
  displayName: string;
  email: string;
  timeZone: string;
  dateTimeFormat: UserDateTimeFormat;
  appearance: string;
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
  const updateProfile = useMutate('updateCurrentProfile');
  const patchPreferences = useMutate('patchProfilePreferences');
  const [update, setUpdate] = useState<Partial<ProfileFormValue>>({});

  const original = useMemo<ProfileFormValue>(
    () => ({
      displayName: profile.displayName,
      email: profile.email,
      timeZone: preferences?.timeZone ?? getBrowserTimezone(),
      dateTimeFormat: preferences?.dateTimeFormat ?? UserDateTimeFormat.System,
      appearance: '',
      passwordCommands: '',
      mfaCommands: '',
      sessionCommands: '',
    }),
    [preferences, profile],
  );

  const handleSave = async (payload: ProfileFormValue) => {
    const displayName = payload.displayName.trim();
    const profileChanged = displayName !== original.displayName;
    const preferencesChanged =
      payload.timeZone !== original.timeZone || payload.dateTimeFormat !== original.dateTimeFormat;

    if (profileChanged) {
      await updateProfile.mutateAsync({ data: { displayName } });
    }

    if (preferencesChanged) {
      await patchPreferences.mutateAsync({
        data: {
          timeZone: payload.timeZone,
          dateTimeFormat: payload.dateTimeFormat,
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
            description: 'Your name and sign-in email.',
            items: [
              defineField({
                key: 'displayName',
                label: 'Display Name',
                required: true,
                description: 'Your display name must be unique.',
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
                    className="w-full max-w-100"
                  />
                ),
              }),
              defineField({
                key: 'dateTimeFormat',
                label: 'Date and Time',
                description: 'How dates and times are displayed.',
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
                key: 'appearance',
                label: 'Appearance',
                description: 'Customize your workspace. Changes are saved automatically.',
                ignoreFormDisabled: true,
                render: () => <AppearanceCustomizer showLabel />,
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
                labelSuffix: (
                  <ProfileMfaStatusBadge canUseLocalPassword={profile.authentication.canUseLocalPasswordMfa} />
                ),
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
    [formatDate, profile.authentication.canUseLocalPasswordMfa, profile.authentication.canChangePassword],
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
      }}
    />
  );
}

function ReadOnlyValue({ value }: { value?: string | null }) {
  return (
    <Input
      aria-label="Email"
      readOnly
      value={value || '-'}
      className="max-w-100 max-h-9 bg-muted/30 text-muted-foreground"
    />
  );
}

function PasswordCommand() {
  const { passwordMinimumLength, passwordMaximumLength } = useSetupContext();
  const queryClient = useQueryClient();
  const changePassword = useMutate('changeCurrentPassword');
  const [open, setOpen] = useState(false);
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');

  const validationError = useMemo(() => {
    if (!newPassword || !confirmPassword) return undefined;
    const length = [...newPassword].length;
    if (length < passwordMinimumLength) return `New password must be at least ${passwordMinimumLength} characters.`;
    if (length > passwordMaximumLength) return `New password must be no more than ${passwordMaximumLength} characters.`;
    return newPassword === confirmPassword ? undefined : 'New password and confirmation do not match.';
  }, [newPassword, confirmPassword, passwordMinimumLength, passwordMaximumLength]);

  const canSubmit = Boolean(currentPassword && newPassword && confirmPassword && !validationError);

  const handleOpenChange = (nextOpen: boolean) => {
    if (changePassword.isPending) return;
    setOpen(nextOpen);
    setCurrentPassword('');
    setNewPassword('');
    setConfirmPassword('');
    changePassword.reset();
  };

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    if (!canSubmit || changePassword.isPending) return;

    try {
      await changePassword.mutateAsync({
        data: {
          currentPassword,
          newPassword,
        },
      });
    } catch {
      // Keep the dialog open; shared HTTP handling and inline validation report the error.
      return;
    }

    setCurrentPassword('');
    setNewPassword('');
    setConfirmPassword('');
    setOpen(false);
    await queryClient.invalidateQueries({ queryKey: ['listProfileSessions'] });
    toast.success('Password changed.');
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger asChild>
        <Button type="button" variant="outline" size="sm">
          <KeyRound className="size-4" />
          Change Password
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Change password</DialogTitle>
          <DialogDescription>Changing your password signs out other Citadel sessions.</DialogDescription>
        </DialogHeader>
        <form className="grid gap-3" onSubmit={handleSubmit}>
          <div className="space-y-2">
            <label htmlFor="profile-current-password" className="text-sm font-medium">
              Current password
            </label>
            <Input
              id="profile-current-password"
              type="password"
              value={currentPassword}
              autoComplete="current-password"
              disabled={changePassword.isPending}
              onChange={(event) => setCurrentPassword(event.target.value)}
            />
          </div>
          <div className="space-y-2">
            <label htmlFor="profile-new-password" className="text-sm font-medium">
              New password
            </label>
            <Input
              id="profile-new-password"
              type="password"
              value={newPassword}
              autoComplete="new-password"
              disabled={changePassword.isPending}
              onChange={(event) => setNewPassword(event.target.value)}
            />
          </div>
          <div className="space-y-2">
            <label htmlFor="profile-confirm-password" className="text-sm font-medium">
              Confirm new password
            </label>
            <Input
              id="profile-confirm-password"
              type="password"
              value={confirmPassword}
              autoComplete="new-password"
              disabled={changePassword.isPending}
              onChange={(event) => setConfirmPassword(event.target.value)}
            />
          </div>
          {!validationError && (
            <p className="text-xs text-muted-foreground">
              Use {passwordMinimumLength} to {passwordMaximumLength} characters.
            </p>
          )}

          {(validationError || changePassword.validationErrors) && (
            <p className="text-sm text-destructive">{validationError ?? changePassword.validationErrors}</p>
          )}

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              disabled={changePassword.isPending}
              onClick={() => handleOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" disabled={!canSubmit || changePassword.isPending}>
              Change Password
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function SessionsCommand({ formatDate }: { formatDate: ProfileDateFormatter }) {
  const [open, setOpen] = useState(false);
  const queryClient = useQueryClient();
  const sessionsQuery = useRead('listProfileSessions', undefined, { staleTime: Infinity });
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
    <Collapsible
      open={open}
      onOpenChange={setOpen}
      className="max-w-full overflow-hidden rounded-lg border bg-card shadow-xs">
      <div className={`flex flex-wrap items-center bg-muted/15 ${open ? 'border-b' : ''}`}>
        <CollapsibleTrigger
          aria-label={
            sessions
              ? `Manage active sessions: ${sessions.sessions.length} session${sessions.sessions.length === 1 ? '' : 's'}`
              : 'Manage active sessions'
          }
          className="flex min-w-0 flex-1 items-center gap-2 p-3 text-left text-sm font-normal hover:bg-muted/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring">
          Manage active sessions
          {sessions && (
            <Badge
              variant="secondary"
              className="min-h-6 leading-4 bg-accent/60"
              title={`${sessions.sessions.length} active session${sessions.sessions.length === 1 ? '' : 's'}`}>
              {sessions.sessions.length}
            </Badge>
          )}
        </CollapsibleTrigger>
        {open && (
          <div className="order-3 flex w-full items-center justify-end gap-2 px-3 pb-3 sm:order-none sm:w-auto sm:py-2 sm:pr-0 sm:pl-0">
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="flex-1 sm:flex-none"
              disabled={sessionsQuery.isFetching}
              onClick={() => void sessionsQuery.refetch()}>
              <RefreshCw
                className={`size-4 ${sessionsQuery.isFetching ? 'animate-spin motion-reduce:animate-none' : ''}`}
                aria-hidden="true"
              />
              Refresh
            </Button>
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="flex-1 sm:flex-none"
              disabled={
                !sessions?.canRevokeOtherSessions || sessions.sessions.length <= 1 || revokeOtherSessions.isPending
              }
              onClick={() => setConfirmAction({ type: 'other-sessions' })}>
              Sign Out Others
            </Button>
          </div>
        )}
        <CollapsibleTrigger
          aria-label={open ? 'Collapse active sessions' : 'Expand active sessions'}
          className="shrink-0 self-start p-3 hover:bg-muted/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring sm:self-auto">
          <ChevronDown
            className={`size-4 transition-transform motion-reduce:transition-none ${open ? 'rotate-180' : ''}`}
            aria-hidden="true"
          />
        </CollapsibleTrigger>
      </div>
      <CollapsibleContent>
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
      </CollapsibleContent>

      <ConfirmSessionDialog
        action={confirmAction}
        pending={revokeSession.isPending || revokeOtherSessions.isPending}
        onOpenChange={(open) => !open && setConfirmAction(null)}
        onConfirm={handleConfirm}
      />
    </Collapsible>
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
              <Badge variant="secondary" className="shrink-0 rounded-sm bg-accent/60 font-normal">
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
