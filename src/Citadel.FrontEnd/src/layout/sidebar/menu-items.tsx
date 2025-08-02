import { Layers, Users, Settings, Bell } from 'lucide-react';
import { JSX } from 'react';
import DockerIcon from '@/assets/docker.min.svg';
interface IMenuItem {
  group: string;
  separator?: boolean;
  items: ISubMenuItem[];
}

interface ISubMenuItem {
  icon?: JSX.Element;
  label: string;
  route?: string | null;
  expanded?: boolean;
  active?: boolean;
  isPlatform?: boolean;
  children?: ISubMenuItem[];
}

const MenuItems: IMenuItem[] = [
  {
    group: 'Base',
    separator: false,
    items: [
      {
        icon: <Layers width={17} height={17} />,
        label: 'Platforms',
        route: '/',
      },
    ],
  },
  {
    group: 'Config',
    separator: false,
    items: [
      {
        icon: <Users width={17} height={17} />,
        label: 'Users',
        route: '/users',
        children: [
          { label: 'Teams', route: '/teams' },
          { label: 'Roles', route: '/roles' },
        ],
      },
      {
        icon: <Settings width={17} height={17} />,
        label: 'Registries',
        route: '/registries',
      },
      {
        icon: <Bell width={17} height={17} />,
        label: 'Notifications',
        route: '/notifications',
      },
    ],
  },
];

/**
 * Generates a dynamic menu for Docker platforms.
 * @param platform - The platform object containing `id` and `name`.
 * @returns A submenu item for the platform.
 */
const DockerPlatformMenu = (platform: { id: string; name: string }): ISubMenuItem => ({
  icon: <DockerIcon />,
  label: platform.name,
  route: `/platforms/${platform.id}`,
  isPlatform: true,
  children: [
    { label: 'Containers', route: `/platforms/${platform.id}/containers` },
    { label: 'Images', route: `/platforms/${platform.id}/images` },
    { label: 'Networks', route: `/platforms/${platform.id}/networks` },
    { label: 'Volumes', route: `/platforms/${platform.id}/volumes` },
  ],
});

export { MenuItems, type IMenuItem, type ISubMenuItem, DockerPlatformMenu };
