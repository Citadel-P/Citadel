import { CurrentProfileView, UserDateTimeFormat } from '@/api/generated/api.types';
import { getBrowserTimezone } from '@/components/custom/timezone-select';
import { Badge } from '@/components/ui/badge';
import Loader from '@/components/ui/loader';
import { useRead } from '@/lib/hooks';
import { CalendarClock, Mail, ShieldCheck, UserRound, Users } from 'lucide-react';
import { ReactNode, useMemo } from 'react';
import { ProfileForm } from './profile-form';
import { formatProfileDateTime, getInitials, ProfileDateFormatter } from './utils';

export default function ProfilePage() {
  const profileQuery = useRead('getCurrentProfile');
  const preferencesQuery = useRead('getProfilePreferences');

  const profile = profileQuery.data?.data;
  const preferences = preferencesQuery.data?.data;
  const timeZone = preferences?.timeZone ?? getBrowserTimezone();
  const dateTimeFormat = preferences?.dateTimeFormat ?? UserDateTimeFormat.System;

  const formatDate = useMemo(
    () => (value: unknown) => formatProfileDateTime(value, timeZone, dateTimeFormat),
    [timeZone, dateTimeFormat],
  );

  if (profileQuery.isLoading || preferencesQuery.isLoading) {
    return (
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="rounded-sm border bg-background p-4">
          <Loader />
        </div>
      </div>
    );
  }

  if (!profile) {
    return (
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="rounded-sm border bg-background p-4 text-sm text-muted-foreground">
          Profile is not available.
        </div>
      </div>
    );
  }

  return (
    <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
      <div className="space-y-4 bg-background p-4">
        <AccountHeader profile={profile} formatDate={formatDate} />
        <ProfileForm profile={profile} preferences={preferences} formatDate={formatDate} />
      </div>
    </div>
  );
}

function AccountHeader({ profile, formatDate }: { profile: CurrentProfileView; formatDate: ProfileDateFormatter }) {
  return (
    <header className="rounded-sm border bg-background">
      <div className="flex flex-col gap-4 p-4 lg:flex-row lg:items-center lg:justify-between">
        <div className="flex min-w-0 items-center gap-3">
          <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-sm bg-primary/10 text-sm font-semibold text-primary">
            {getInitials(profile.displayName)}
          </div>
          <div className="min-w-0">
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <h1 className="min-w-0 truncate text-lg font-semibold text-foreground">{profile.displayName}</h1>
              <Badge variant="secondary" className="rounded-sm">
                {profile.authentication.label}
              </Badge>
            </div>
            <div className="mt-1 flex min-w-0 items-center gap-2 text-sm text-muted-foreground">
              <Mail className="size-3.5 shrink-0" />
              <span className="truncate">{profile.email}</span>
            </div>
          </div>
        </div>

        <div className="grid gap-3 text-sm sm:grid-cols-2 lg:min-w-[520px] lg:grid-cols-4">
          <HeaderFact
            icon={<ShieldCheck className="size-3.5" />}
            label="Roles"
            value={profile.directRoles.length.toString()}
          />
          <HeaderFact icon={<Users className="size-3.5" />} label="Teams" value={profile.teams.length.toString()} />
          <HeaderFact icon={<UserRound className="size-3.5" />} label="Method" value={profile.authentication.label} />
          <HeaderFact
            icon={<CalendarClock className="size-3.5" />}
            label="Created"
            value={formatDate(profile.createdAt)}
          />
        </div>
      </div>
    </header>
  );
}

function HeaderFact({ icon, label, value }: { icon: ReactNode; label: string; value: string }) {
  return (
    <div className="min-w-0 border-l pl-3">
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        {icon}
        <span>{label}</span>
      </div>
      <div className="mt-1 truncate font-medium text-foreground" title={value}>
        {value}
      </div>
    </div>
  );
}
