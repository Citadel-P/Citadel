import { User, LogOut } from 'lucide-react';
import { JSX } from 'react/jsx-runtime';
import { useAuthContext } from '@/features/auth/auth-context';
import { useRead } from '@/lib/hooks';
import { useNavigate } from 'react-router';
import { getInitials } from '@/features/profile/utils';
import { jwtDecode } from 'jwt-decode';
import { useMemo } from 'react';

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

export const SidebarDropDown = () => {
  const navigate = useNavigate();
  const { logout, accessToken } = useAuthContext();
  const { data: cachedProfileData } = useRead('getCurrentProfile', undefined, { enabled: false });
  const tokenProfile = useMemo(() => getTokenProfile(accessToken), [accessToken]);
  const profile = cachedProfileData?.data ?? tokenProfile;

  const handleMenuClick = (item: ProfileMenu) => {
    if (item.key == 'logout') {
      logout();
      return;
    }
    navigate(item.link);
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
    </div>
  );
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
