import {
  Layers,
  Users,
  Cable,
  Bell,
  Rocket,
  Server,
  GitBranch,
  ChevronsLeftRightEllipsis,
  Settings,
  Megaphone,
  TriangleAlert,
} from 'lucide-react';
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
    group: 'Infrastructure',
    separator: false,
    items: [
      {
        icon: <Server width={14} height={14} />,
        label: 'Platforms',
        route: '/',
      },
    ],
  },
  {
    group: 'Orchestration',
    separator: false,
    items: [
      {
        icon: <Rocket width={14} height={14} />,
        label: 'Deployments',
        route: '/deployments',
      },
      {
        icon: <Layers width={14} height={14} />,
        label: 'Stacks',
        route: '/stacks',
      },
    ],
  },
  {
    group: 'System',
    separator: false,
    items: [
      {
        icon: <TriangleAlert width={14} height={14} />,
        label: 'Alerts',
        route: '/alerts',
      },
      {
        icon: <Bell width={14} height={14} />,
        label: 'Notifications',
        route: '/notifications',
      },
      {
        icon: <Settings className="w-4 h-4" />,
        label: 'Settings',
        route: '/settings',
        children: [
          {
            icon: <Cable className="w-3.5 h-3.5" />,
            label: 'Registries',
            route: '/registries',
          },
          {
            icon: <GitBranch className="w-3.5 h-3.5" />,
            label: 'Git Providers',
            route: '/git-providers',
          },
          {
            icon: <ChevronsLeftRightEllipsis className="w-3.5 h-3.5" />,
            label: 'Variables',
            route: '/variables',
          },
          {
            icon: <Megaphone className="w-3.5 h-3.5" />,
            label: 'Alerters',
            route: '/alerters',
          },
          {
            icon: <Users className="w-3.5 h-3.5" />,
            label: 'Users',
            route: '/users',
          },
        ],
      },
    ],
  },
];

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
