import type { AlertEventView } from '@/api/generated/api.types';
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
import { useTaskSheet } from '@/lib/atoms';
import { useLayoutContext, type ThemeMode } from '@/lib/context/layout-context';
import { toUserTheme } from '@/lib/theme-preferences';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { useQueryClient } from '@tanstack/react-query';
import { jwtDecode } from 'jwt-decode';
import { Bell, LogOut, Moon, Sun, User } from 'lucide-react';
import { useMemo } from 'react';
import { useNavigate } from 'react-router';
import { BreadcrumbTrail } from './breadcrumb';

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
      <div className="ml-auto flex items-center gap-4">
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

function AlertBell() {
  const navigate = useNavigate();
  const { open: openAlertSheet } = useTaskSheet('Alert');
  const formatDateTime = useProfileDateTimeFormatter();
  const { unresolvedAlertCount, liveAlertEvents } = useAppContext();
  const latestEvents = useMemo(
    () =>
      Object.values(liveAlertEvents)
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
      <DropdownMenuContent align="end" className="w-80 p-2 bg-background">
        <DropdownMenuLabel className="flex items-center justify-between">
          <span>Alerts</span>
          {unresolvedAlertCount > 0 ? (
            <span className="text-xs font-normal text-muted-foreground">{unresolvedAlertCount} unresolved</span>
          ) : null}
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        {latestEvents.length > 0 ? (
          <div className="max-h-80 overflow-y-auto">
            {latestEvents.map((event) => (
              <AlertEventPreview
                key={event.id}
                event={event}
                formatDateTime={formatDateTime}
                onOpen={() => {
                  openAlertSheet({ kind: 'alertEvent', payload: event });
                  navigate('/alerts');
                }}
              />
            ))}
          </div>
        ) : (
          <div className="px-2 py-6 text-center text-sm text-muted-foreground">No live alerts</div>
        )}
        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={() => navigate('/alerts')} className="justify-center text-xs font-medium">
          View all alerts
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
  return (
    <DropdownMenuItem onSelect={onOpen} className="block h-auto cursor-pointer rounded-sm px-2 py-2 hover:bg-card">
      <div className="flex min-w-0 items-center justify-between gap-2">
        <span className="truncate text-xs font-semibold">{event.resourceName || event.resourceType}</span>
        <span className="shrink-0 text-[10px] text-muted-foreground">{formatDateTime(event.createdAt)}</span>
      </div>
      <p className="mt-1 line-clamp-2 text-xs text-muted-foreground">{event.message}</p>
    </DropdownMenuItem>
  );
}

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
