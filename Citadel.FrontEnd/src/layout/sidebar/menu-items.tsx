import { Layers, Users, Settings, Bell } from 'lucide-react';

interface IMenuItem {
  group: string;
  separator?: boolean;
  selected?: boolean;
  active?: boolean;
  items: Array<ISubMenuItem>;
}

interface ISubMenuItem {
  icon?: any;
  label?: string;
  route?: string | null;
  expanded?: boolean;
  active?: boolean;
  isPlatform?: boolean;
  children?: Array<ISubMenuItem>;
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

/*const DockerPlatformMenu = function (platform: any): ISubMenuItem {
  return {
    icon: 'assets/icons/docker.min.svg',
    label: platform.name,
    route: `/platforms/${platform.id}`,
    isPlatform: true,
    children: [
      { label: 'Containers', route: `/platforms/${platform.id}/containers` },
      { label: 'Images', route: `/platforms/${platform.id}/images` },
      { label: 'Networks', route: `/platforms/${platform.id}/networks` },
      { label: 'Volumes', route: `/platforms/${platform.id}/volumes` },
    ],
  };
};*/

export { MenuItems, type IMenuItem, type ISubMenuItem };
