import { Settings } from 'lucide-react';
import { JSX } from 'react';
import DockerIcon from '@/assets/docker.min.svg';
import { CitadelIcons } from '@/lib/icons';
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

const renderIcon = (Icon?: React.ComponentType<{ className?: string }>, className = 'w-3.5 h-3.5') => {
  if (!Icon) return <></>;
  return <Icon className={className} />;
};

const MenuItems: IMenuItem[] = [
  {
    group: 'Infrastructure',
    separator: false,
    items: [
      {
        icon: renderIcon(CitadelIcons.Platform),
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
        icon: renderIcon(CitadelIcons.Deployment),
        label: 'Deployments',
        route: '/deployments',
      },
      {
        icon: renderIcon(CitadelIcons.Stack),
        label: 'Stacks',
        route: '/stacks',
      },
    ],
  },
  {
    group: 'Providers',
    separator: false,
    items: [
      {
        icon: renderIcon(CitadelIcons.Registry),
        label: 'Registries',
        route: '/registries',
      },
      {
        icon: renderIcon(CitadelIcons.GitRepository),
        label: 'Repositories',
        route: '/git-repos',
      },
    ],
  },
  {
    group: 'System',
    separator: false,
    items: [
      {
        icon: renderIcon(CitadelIcons.Alert),
        label: 'Alerts',
        route: '/alerts',
      },
      {
        icon: renderIcon(CitadelIcons.Activity),
        label: 'Activities',
        route: '/activities',
      },
      {
        icon: <Settings className="w-4 h-4" />,
        label: 'Settings',
        route: '/settings',
        children: [
          {
            icon: renderIcon(CitadelIcons.AlertRule),
            label: 'Alert Rules',
            route: '/alert-rules',
          },
          {
            icon: renderIcon(CitadelIcons.Variable),
            label: 'Variables',
            route: '/variables',
          },
          {
            icon: renderIcon(CitadelIcons.Access),
            label: 'Access',
            route: '/access',
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
