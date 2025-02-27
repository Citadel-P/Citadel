import { User, Settings, LogOut, Sun, Moon } from 'lucide-react';
import { useContextSelector } from 'use-context-selector';
import { LayoutContext } from '@/layout/LayoutProvider';
import { JSX } from 'react/jsx-runtime';

interface ProfileMenu {
  title: string;
  link: string;
  icon: JSX.Element;
}
const profileMenu: ProfileMenu[] = [
  {
    title: 'Your Profile',
    link: '/profile',
    icon: <User width={20} />,
  },
  {
    title: 'Settings',
    link: '/settings',
    icon: <Settings width={20} />,
  },
  {
    title: 'Log out',
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

export const PorfileDropDown = () => {
  const theme = useContextSelector(LayoutContext, (v) => v?.theme);
  const setThemeMode = useContextSelector(LayoutContext, (v) => v?.setThemeMode);
  const toggleThemeColor = useContextSelector(LayoutContext, (v) => v?.toggleThemeColor);

  const handleMenuClick = (menu: ProfileMenu) => {};
  return (
    <div className="absolute right-0 z-20 mt-2 w-60 origin-top-right transform rounded-md bg-background py-4 drop-shadow-md shadow-custom ring-1 ring-transparent ring-opacity-5 transition focus:outline-hidden">
      <div className="flext-row flex items-center px-4 pb-4">
        <div className="w-10 shrink-0">
          <img className="rounded-md" src="https://avatars.githubusercontent.com/u/993610?v=4" alt="" />
        </div>
        <div className="overflow-hidden px-2 text-sm font-semibold text-foreground">
          7amou3
          <p className="truncate text-ellipsis text-xs font-semibold text-muted-foreground">me&#64;7amou3</p>
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
              onClick={() => setThemeMode!(item.name as any)}
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
