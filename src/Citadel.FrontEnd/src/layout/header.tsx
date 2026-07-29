import { AlertEventStatus, AlertResourceType, AlertSeverity, type AlertEventView } from '@/api/generated/api.types';
import type { ResourceType } from '@/api/types';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Separator } from '@/components/ui/separator';
import { SidebarTrigger } from '@/components/ui/sidebar';
import { useAuthContext } from '@/features/auth/auth-context';
import { getInitials } from '@/features/profile/utils';
import { useRead, useMutate } from '@/lib/hooks';
import { useAppContext } from '@/lib/context/app-context';
import { useLayoutContext, type ThemeMode } from '@/lib/context/layout-context';
import { toUserTheme } from '@/lib/theme-preferences';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { useQueryClient } from '@tanstack/react-query';
import { jwtDecode } from 'jwt-decode';
import { ArrowRight, Bell, BellOff, LogOut, Moon, Sun, User } from 'lucide-react';
import { useMemo } from 'react';
import { useNavigate } from 'react-router';
import { BreadcrumbTrail } from './breadcrumb';
import { GlobalSearch } from '@/features/search/global-search';
import { useOpenAlertEventSheet } from '@/features/alerters/alert-events/alert-task-sheet';
import { CitadelIcons } from '@/lib/icons';
import { fromNow } from '@/lib/dayjs.helper';
import { cn, formatActivityEvent } from '@/lib/utils';
import { LiveConnectionIndicator } from './live-connection-indicator';

const themeColors = [
  { name: 'base', code: '#e11d48' },
  { name: 'yellow', code: '#f59e0b' },
  { name: 'green', code: '#22c55e' },
  { name: 'blue', code: '#3b82f6' },
  { name: 'orange', code: '#ea580c' },
  { name: 'red', code: '#cc0022' },
  { name: 'violet', code: '#6d28d9' },
];

const themeModes = [
  { name: 'light' as const, icon: Sun },
  { name: 'dark' as const, icon: Moon },
];

export function Header() {
  return (
    <header className="flex h-16 shrink-0 items-center gap-2 border-b px-4 shadow-none backdrop-blur transition-[width,height] supports-backdrop-filter:bg-background/90 bg-background/90 ">
      <SidebarTrigger />
      <Separator orientation="vertical" className="mr-2 h-4" />
      <div className="flex min-w-0 flex-1 items-center">
        <BreadcrumbTrail compact className="min-w-0" />
      </div>
      <div className="flex shrink-0 items-center px-2 empty:hidden sm:px-4">
        <LiveConnectionIndicator />
      </div>
      <div className="ml-auto flex items-center gap-4">
        <GlobalSearch />
        <AlertBell />
        <HeaderAccountMenu />
      </div>
    </header>
  );
}

function HeaderAccountMenu() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { logout, accessToken } = useAuthContext();
  const { toggleThemeColor, setThemeMode, theme } = useLayoutContext();
  const { data: cachedProfileData } = useRead('getCurrentProfile', undefined, { enabled: false });
  const patchPreferences = useMutate('patchProfilePreferences');
  const tokenProfile = useMemo(() => getTokenProfile(accessToken), [accessToken]);
  const profile = cachedProfileData?.data ?? tokenProfile;

  const handleThemeModeClick = (mode: ThemeMode) => {
    setThemeMode(mode);

    if (mode === theme.mode) return;

    patchPreferences.mutate(
      { data: { theme: toUserTheme(mode) } as any },
      {
        onSuccess: () => {
          queryClient.invalidateQueries({ queryKey: ['getProfilePreferences'] });
        },
      },
    );
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon-sm" aria-label="Open account menu">
          <span className="flex size-7 items-center justify-center rounded-md bg-primary/10 text-xs font-semibold text-primary">
            {profile?.displayName ? getInitials(profile.displayName) : <User className="size-4" />}
          </span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-72 p-2 bg-background">
        <DropdownMenuLabel className="px-2 py-2">
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex size-9 shrink-0 items-center justify-center rounded-md bg-primary/10 text-xs font-semibold text-primary">
              {profile?.displayName ? getInitials(profile.displayName) : <User className="size-4" />}
            </div>
            <div className="min-w-0">
              <div className="truncate text-sm font-semibold">{profile?.displayName ?? 'Profile'}</div>
              <div className="truncate text-xs font-normal text-muted-foreground">{profile?.email ?? ''}</div>
            </div>
          </div>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={() => navigate('/profile')}>
          <User className="size-4" />
          My Profile
        </DropdownMenuItem>
        <DropdownMenuItem variant="destructive" onClick={logout}>
          <LogOut className="size-4" />
          Log out
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuLabel className="text-xs text-muted-foreground">Color</DropdownMenuLabel>
        <div className="grid grid-cols-2 gap-2 p-2">
          {themeColors.map((item) => (
            <button
              key={item.name}
              type="button"
              onClick={() => toggleThemeColor(item.name)}
              className={`focus-visible:ring-ring inline-flex h-8 items-center justify-start rounded-md border px-3 text-xs font-medium transition-colors focus-visible:outline-hidden focus-visible:ring-1 hover:bg-card ${
                item.name === theme.color
                  ? 'border-muted-foreground bg-card text-foreground'
                  : 'border-border text-muted-foreground'
              }`}>
              <span style={{ backgroundColor: item.code }} className="mr-2 size-4 shrink-0 rounded-full" />
              <span className="capitalize">{item.name}</span>
            </button>
          ))}
        </div>
        <DropdownMenuLabel className="text-xs text-muted-foreground">Mode</DropdownMenuLabel>
        <div className="grid grid-cols-2 gap-2 p-2 pt-1">
          {themeModes.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.name}
                type="button"
                onClick={() => handleThemeModeClick(item.name)}
                className={`focus-visible:ring-ring inline-flex h-8 items-center justify-start rounded-md border px-3 text-xs font-medium transition-colors focus-visible:outline-hidden focus-visible:ring-1 hover:bg-card ${
                  item.name === theme.mode
                    ? 'border-muted-foreground bg-card text-foreground'
                    : 'border-border text-muted-foreground'
                }`}>
                <Icon className="mr-2 size-4" />
                <span className="capitalize">{item.name}</span>
              </button>
            );
          })}
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export function AlertBell() {
  const navigate = useNavigate();
  const openAlertSheet = useOpenAlertEventSheet();
  const formatDateTime = useProfileDateTimeFormatter();
  const { unresolvedAlertCount, liveAlertEvents } = useAppContext();
  const latestEvents = useMemo(
    () =>
      Object.values(liveAlertEvents)
        .filter((event) => event.status !== AlertEventStatus.Resolved)
        .sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime())
        .slice(0, 5),
    [liveAlertEvents],
  );
  const badge = unresolvedAlertCount > 99 ? '99+' : unresolvedAlertCount.toString();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon-sm" aria-label="Open alert notifications" className="relative">
          <Bell className="size-4" />
          {unresolvedAlertCount > 0 ? (
            <span className="absolute -right-1 -top-1 min-w-4 rounded-full bg-destructive px-1 text-[10px] font-semibold leading-4 text-destructive-foreground">
              {badge}
            </span>
          ) : null}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-96 max-w-[calc(100vw-1rem)] overflow-hidden bg-background p-0">
        <DropdownMenuLabel className="flex items-center justify-between gap-3 px-4 py-3">
          <span className="min-w-0">
            <span className="block text-sm font-semibold">Alerts</span>
            <span className="block text-xs font-normal text-muted-foreground">Recent unresolved events</span>
          </span>
          {unresolvedAlertCount > 0 ? (
            <span className="shrink-0 rounded-full border bg-muted/50 px-2 py-0.5 text-[11px] font-medium tabular-nums text-muted-foreground">
              {unresolvedAlertCount} unresolved
            </span>
          ) : null}
        </DropdownMenuLabel>
        <DropdownMenuSeparator className="m-0" />
        {latestEvents.length > 0 ? (
          <div className="max-h-96 divide-y divide-border/60 overflow-y-auto">
            {latestEvents.map((event) => (
              <AlertEventPreview
                key={event.id}
                event={event}
                formatDateTime={formatDateTime}
                onOpen={() => openAlertSheet(event.id)}
              />
            ))}
          </div>
        ) : (
          <div className="flex flex-col items-center px-6 py-8 text-center">
            <span className="mb-3 flex size-9 items-center justify-center rounded-full border bg-muted/40 text-muted-foreground">
              <BellOff className="size-4" />
            </span>
            <span className="text-sm font-medium">No unresolved alerts</span>
            <span className="mt-1 text-xs text-muted-foreground">New operational alerts will appear here.</span>
          </div>
        )}
        <DropdownMenuSeparator className="m-0" />
        <DropdownMenuItem
          onClick={() => navigate('/alerts')}
          className="h-10 justify-between rounded-none px-4 text-xs font-medium">
          <span>View all alerts</span>
          <ArrowRight className="size-3.5" />
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function AlertEventPreview({
  event,
  formatDateTime,
  onOpen,
}: {
  event: AlertEventView;
  formatDateTime: (value: unknown) => string;
  onOpen: () => void;
}) {
  const Icon = CitadelIcons[event.resourceType as ResourceType] ?? CitadelIcons.Alert;
  const resourceIconClassName = alertResourceIconStyles[event.resourceType] ?? 'bg-muted/60 text-muted-foreground';
  const severityStyle = alertSeverityStyles[event.severity] ?? alertSeverityStyles[AlertSeverity.Info];

  return (
    <DropdownMenuItem
      onSelect={onOpen}
      className="h-auto cursor-pointer items-center gap-3 rounded-none px-4 py-3 focus:bg-accent/60">
      <span className={cn('flex size-7 shrink-0 items-center justify-center rounded-full', resourceIconClassName)}>
        <Icon className="size-3.5" />
      </span>

      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate text-[13px] font-medium text-foreground">{formatActivityEvent(event.type)}</span>
          <span
            className="ml-auto shrink-0 text-[11px] font-normal text-muted-foreground"
            title={formatDateTime(event.createdAt)}>
            {fromNow(event.createdAt)}
          </span>
        </div>

        <p className="mt-1 line-clamp-2 text-xs font-normal leading-4 text-muted-foreground">{event.message}</p>

        <div className="mt-2 flex min-w-0 items-center gap-1.5 text-[11px] font-normal">
          <span className={cn('inline-flex shrink-0 items-center gap-1.5 font-medium', severityStyle.text)}>
            <span className={cn('size-1.5 rounded-full', severityStyle.dot)} />
            {event.severity}
          </span>
          <span className="size-0.5 shrink-0 rounded-full bg-border" />
          <span className="truncate text-muted-foreground">
            {event.resourceName || formatActivityEvent(event.resourceType)}
          </span>
          {event.status === AlertEventStatus.Acknowledged ? (
            <>
              <span className="size-0.5 shrink-0 rounded-full bg-border" />
              <span className="shrink-0 text-muted-foreground">Acknowledged</span>
            </>
          ) : null}
        </div>
      </div>
    </DropdownMenuItem>
  );
}

const alertResourceIconStyles: Record<AlertResourceType, string> = {
  [AlertResourceType.Platform]: 'bg-sky-500/10 text-sky-600 dark:text-sky-400',
  [AlertResourceType.Deployment]: 'bg-blue-500/10 text-blue-600 dark:text-blue-400',
  [AlertResourceType.Stack]: 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400',
  [AlertResourceType.GitRepository]: 'bg-orange-500/10 text-orange-600 dark:text-orange-400',
  [AlertResourceType.Webhook]: 'bg-pink-500/10 text-pink-600 dark:text-pink-400',
  [AlertResourceType.AutomationAction]: 'bg-amber-500/10 text-amber-600 dark:text-amber-400',
  [AlertResourceType.Build]: 'bg-cyan-500/10 text-cyan-600 dark:text-cyan-400',
  [AlertResourceType.License]: 'bg-violet-500/10 text-violet-600 dark:text-violet-400',
};

const alertSeverityStyles: Record<AlertSeverity, { dot: string; text: string }> = {
  [AlertSeverity.Info]: {
    dot: 'bg-blue-500',
    text: 'text-blue-600 dark:text-blue-400',
  },
  [AlertSeverity.Warning]: {
    dot: 'bg-amber-500',
    text: 'text-amber-600 dark:text-amber-400',
  },
  [AlertSeverity.Critical]: {
    dot: 'bg-red-500',
    text: 'text-red-600 dark:text-red-400',
  },
};

function getTokenProfile(accessToken: string | undefined) {
  if (!accessToken) return undefined;

  try {
    const decoded = jwtDecode<{ displayName?: string; name?: string; email?: string }>(accessToken);
    return {
      displayName: decoded.displayName ?? decoded.name,
      email: decoded.email,
    };
  } catch {
    return undefined;
  }
}
