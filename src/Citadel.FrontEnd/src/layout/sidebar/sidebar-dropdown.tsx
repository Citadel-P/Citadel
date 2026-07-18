import { User, LogOut, Sun, Moon } from 'lucide-react';
import { useLayoutContext } from '@/lib/context/layout-context';
import { JSX } from 'react/jsx-runtime';
import { useAuthContext } from '@/features/auth/auth-context';
import { useMutate, useRead } from '@/lib/hooks';
import { useNavigate } from 'react-router';
import { getInitials } from '@/features/profile/utils';
import { jwtDecode } from 'jwt-decode';
import { useMemo } from 'react';
import type { ThemeMode } from '@/lib/context/layout-context';
import { toUserTheme } from '@/lib/theme-preferences';
import { useQueryClient } from '@tanstack/react-query';

interface ProfileMenu {
  title: string;
  link: string;
  key?: string;
  icon: JSX.Element;
}
const profileMenu: ProfileMenu[] = [
  {
    title: 'My Profile',
    link: '/profile',
    icon: <User width={20} />,
  },
  {
    title: 'Log out',
    key: 'logout',
    link: '/auth',
    icon: <LogOut width={20} />,
  },
];

const themeColors = [
  {
    name: 'base',
    code: '#e11d48',
  },
  {
    name: 'yellow',
    code: '#f59e0b',
  },
  {
    name: 'green',
    code: '#22c55e',
  },
  {
    name: 'blue',
    code: '#3b82f6',
  },
  {
    name: 'orange',
    code: '#ea580c',
  },
  {
    name: 'red',
    code: '#cc0022',
  },
  {
    name: 'violet',
    code: '#6d28d9',
  },
];

const themeModes = [
  {
    name: 'light',
    icon: <Sun width={18} />,
  },
  {
    name: 'dark',
    icon: <Moon width={18} />,
  },
];

export const SidebarDropDown = () => {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { logout, accessToken } = useAuthContext();
  const { toggleThemeColor, setThemeMode, theme } = useLayoutContext();
  const { data: cachedProfileData } = useRead('getCurrentProfile', undefined, { enabled: false });
  const patchPreferences = useMutate('patchProfilePreferences');
  const tokenProfile = useMemo(() => getTokenProfile(accessToken), [accessToken]);
  const profile = cachedProfileData?.data ?? tokenProfile;

  const handleMenuClick = (item: ProfileMenu) => {
    if (item.key == 'logout') {
      logout();
      return;
    }
    navigate(item.link);
  };

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
    <div className="absolute bottom-0 z-10 mt-2 w-60 origin-bottom-left transform rounded-md bg-background py-4 drop-shadow-md shadow-custom ring-1 ring-transparent ring-opacity-5 transition focus:outline-hidden">
      <div className="flext-row flex items-center px-4 pb-4">
        <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-primary/10 text-sm font-semibold text-primary">
          {getInitials(profile?.displayName)}
        </div>
        <div className="overflow-hidden px-2 text-sm font-semibold text-foreground">
          {profile?.displayName ?? 'Profile'}
          <p className="truncate text-ellipsis text-xs font-semibold text-muted-foreground">{profile?.email ?? ''}</p>
        </div>
      </div>

      <div className="border-b border-dashed border-border"></div>

      <ul className="my-2 mx-4 flex flex-col">
        {profileMenu.map((menu, index) => (
          <li key={index} className="">
            <div
              role="presentation"
              className="flex grow cursor-pointer items-center gap-2 rounded-md px-3 py-2 text-xs font-semibold text-muted-foreground hover:bg-card hover:text-primary"
              onClick={() => handleMenuClick(menu)}>
              <span className="h-6 w-6 text-muted-foreground/50">{menu.icon}</span>
              {menu.title}
            </div>
          </li>
        ))}
      </ul>
      <hr className="border-dashed border-border" />
      <div className="mx-4 my-2">
        <span className="text-xs font-semibold text-foreground">Color</span>
        <div className="mt-2 grid grid-cols-2 gap-2">
          {themeColors.map((item, index) => (
            <button
              key={index}
              onClick={() => toggleThemeColor!(item.name)}
              className={`${item.name === theme!.color ? 'border-muted-foreground bg-card' : ''} focus-visible:ring-ring inline-flex h-8 items-center justify-start whitespace-nowrap rounded-md border border-border bg-background px-3 text-xs font-medium text-muted-foreground shadow-xs transition-colors focus-visible:outline-hidden focus-visible:ring-1 disabled:pointer-events-none disabled:opacity-50 hover:bg-card hover:text-foreground`}>
              <span
                style={{ backgroundColor: item.code }}
                className="mr-1 flex h-5 w-5 shrink-0 -translate-x-1 items-center justify-center rounded-full bg-rose-500"></span>
              <p className="capitalize">{item.name}</p>
            </button>
          ))}
        </div>
      </div>

      <div className="mx-4 my-2">
        <span className="text-xs font-semibold text-foreground">Mode</span>
        <div className="mt-2 grid grid-cols-2 gap-2">
          {themeModes.map((item, index) => (
            <button
              key={index}
              onClick={() => handleThemeModeClick(item.name as ThemeMode)}
              className={`${item.name === theme!.mode ? 'border-muted-foreground bg-card' : ''} focus-visible:ring-ring inline-flex h-8 items-center justify-start whitespace-nowrap rounded-md border border-border bg-background px-3 text-xs font-medium text-muted-foreground shadow-xs transition-colors focus-visible:outline-hidden focus-visible:ring-1 disabled:pointer-events-none disabled:opacity-50 hover:bg-card hover:text-foreground`}>
              <span className="h-6 w-7 text-muted-foreground/50">{item.icon}</span>
              <p className="capitalize">{item.name}</p>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
};

function getTokenProfile(accessToken: string | undefined) {
  if (!accessToken) return undefined;

  try {
    return jwtDecode<{ name?: string; email?: string }>(accessToken);
  } catch {
    return undefined;
  }
}
