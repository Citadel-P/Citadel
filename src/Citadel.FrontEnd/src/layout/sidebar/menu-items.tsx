import { Layers, ChevronsLeftRightEllipsis, Settings, Activity } from 'lucide-react';
import { JSX } from 'react';
import DockerIcon from '@/assets/docker.min.svg';
import { AccessComponents } from '@/features/access';
import { AlertEventComponents } from '@/features/alerters/alert-events';
import { AlertRuleComponents } from '@/features/alerters/alert-rules';
import { DeploymentComponents } from '@/features/deployments';
import { GitRepoComponents } from '@/features/git-repos';
import { PlatformComponents } from '@/features/platforms';
import { RegistryComponents } from '@/features/registries';
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
        icon: renderIcon(PlatformComponents.Icon),
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
        icon: renderIcon(DeploymentComponents.Icon),
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
    group: 'Providers',
    separator: false,
    items: [
      {
        icon: renderIcon(RegistryComponents.Icon),
        label: 'Registries',
        route: '/registries',
      },
      {
        icon: renderIcon(GitRepoComponents.Icon),
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
        icon: renderIcon(AlertEventComponents.Icon),
        label: 'Alerts',
        route: '/alerts',
      },
      {
        icon: <Activity width={14} height={14} />,
        label: 'Activities',
        route: '/activities',
      },
      {
        icon: <Settings className="w-4 h-4" />,
        label: 'Settings',
        route: '/settings',
        children: [
          {
            icon: renderIcon(AlertRuleComponents.Icon),
            label: 'Alert Rules',
            route: '/alert-rules',
          },
          {
            icon: <ChevronsLeftRightEllipsis className="w-3.5 h-3.5" />,
            label: 'Variables',
            route: '/variables',
          },
          {
            icon: renderIcon(AccessComponents.Icon),
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
