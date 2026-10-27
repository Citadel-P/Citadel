import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { Timestamp } from '@/components/custom/timestamp';
import { AppContent } from '@/components/custom/app-content';
import { CurrentProfileView, ProfileResourceInfo } from '@/api/generated/api.types';
import { OverflowCountBadge } from '@/components/custom/common';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import Loader from '@/components/ui/loader';
import { useRead } from '@/lib/hooks';
import { CalendarClock, Mail, ShieldCheck, Users } from 'lucide-react';
import { ReactNode, useId, useState } from 'react';
import { ProfileForm } from './profile-form';
import { getInitials, ProfileDateFormatter } from './utils';

export default function ProfilePage() {
  const profileQuery = useRead('getCurrentProfile');
  const preferencesQuery = useRead('getProfilePreferences');

  const profile = profileQuery.data?.data;
  const preferences = preferencesQuery.data?.data;
  const formatDate = useProfileDateTimeFormatter();

  if (profileQuery.isLoading || preferencesQuery.isLoading) {
    return (
      <AppContent>
        <Loader />
      </AppContent>
    );
  }

  if (!profile) {
    return <AppContent className="text-sm text-muted-foreground">Profile is not available.</AppContent>;
  }

  return (
    <AppContent className="space-y-(--section-gap)">
      <AccountHeader profile={profile} formatDate={formatDate} />
      <ProfileForm profile={profile} preferences={preferences} formatDate={formatDate} />
    </AppContent>
  );
}

function AccountHeader({ profile, formatDate }: { profile: CurrentProfileView; formatDate: ProfileDateFormatter }) {
  return (
    <header className="rounded-lg border bg-card shadow-xs">
      <div className="flex flex-col gap-(--section-gap) p-(--surface-padding) lg:flex-row lg:items-center lg:justify-between">
        <div className="flex min-w-0 items-center gap-3">
          <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-md bg-primary/10 text-sm font-semibold text-primary">
            {getInitials(profile.displayName)}
          </div>
          <div className="min-w-0">
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <h1 className="min-w-0 truncate text-lg font-semibold text-foreground">{profile.displayName}</h1>
              <Badge variant="secondary" className="min-h-6 leading-4 bg-accent/60 border-none">
                {profile.authentication.label}
              </Badge>
              {profile.authentication.oidcProviderName && (
                <Badge variant="secondary" className="min-h-6 max-w-full leading-4 bg-accent/60 border-none">
                  <span className="truncate">{profile.authentication.oidcProviderName}</span>
                </Badge>
              )}
            </div>
            <div className="mt-1 flex min-w-0 items-center gap-2 text-sm text-muted-foreground">
              <Mail className="size-3.5 shrink-0" />
              <span className="truncate">{profile.email}</span>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-3 text-sm sm:grid-cols-[1fr_1fr_auto] lg:min-w-100">
          <HeaderMembership
            icon={<ShieldCheck className="size-3.5" />}
            label="Roles"
            description="Roles assigned directly to you."
            items={profile.directRoles}
          />
          <HeaderMembership
            icon={<Users className="size-3.5" />}
            label="Teams"
            description="Teams you're a member of."
            items={profile.teams}
          />
          <HeaderFact
            icon={<CalendarClock className="size-3.5" />}
            className="col-span-2 sm:col-span-1"
            label="Created"
            value={<Timestamp value={profile.createdAt} formatDateTime={formatDate} />}
          />
        </div>
      </div>
    </header>
  );
}

function HeaderMembership({
  icon,
  label,
  description,
  items,
}: {
  icon: ReactNode;
  label: string;
  description: string;
  items: ProfileResourceInfo[];
}) {
  const [open, setOpen] = useState(false);
  const contentId = useId();
  const first = items[0];
  const remaining = items.length - 1;

  return (
    <div className="min-w-0 border-l pl-3">
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        {icon}
        <span>{label}</span>
      </div>
      <div className="mt-1">
        {first ? (
          <HoverCard open={open} onOpenChange={setOpen} openDelay={150} closeDelay={150}>
            <HoverCardTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-auto w-full min-w-0 justify-start gap-1.5 px-2 py-0 text-left"
                aria-label={`${label}: ${first.name}${remaining > 0 ? `, ${remaining} more` : ''}`}
                aria-expanded={open}
                aria-controls={open ? contentId : undefined}
                onClick={() => setOpen(true)}
                onTouchStart={() => setOpen(true)}
                onKeyDown={(event) => {
                  if (event.key === 'Escape') setOpen(false);
                }}>
                <span className="min-w-0 truncate">{first.name}</span>
                <OverflowCountBadge count={remaining} className="shrink-0" />
              </Button>
            </HoverCardTrigger>
            <HoverCardContent
              id={contentId}
              role="region"
              aria-label={label}
              align="start"
              className="w-72 max-w-[calc(100vw-2rem)] p-3">
              <p className="text-sm font-semibold">{label}</p>
              <p className="mt-1 text-xs text-muted-foreground">{description}</p>
              <ul className="mt-3 max-h-60 space-y-2 overflow-y-auto text-sm">
                {items.map((item) => (
                  <li key={item.id} className="flex min-w-0 items-start gap-2">
                    <span className="mt-0.5 shrink-0 text-muted-foreground">{icon}</span>
                    <span className="min-w-0 wrap-anywhere">{item.name}</span>
                  </li>
                ))}
              </ul>
            </HoverCardContent>
          </HoverCard>
        ) : (
          <span className="text-sm text-muted-foreground">--</span>
        )}
      </div>
    </div>
  );
}

function HeaderFact({
  icon,
  label,
  value,
  className = '',
}: {
  icon: ReactNode;
  label: string;
  value: ReactNode;
  className?: string;
}) {
  return (
    <div className={`min-w-0 border-l pl-3 ${className}`}>
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        {icon}
        <span>{label}</span>
      </div>
      <div className="mt-1 truncate font-medium text-foreground" title={typeof value === 'string' ? value : undefined}>
        {value}
      </div>
    </div>
  );
}
