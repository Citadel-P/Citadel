import { KeyRound, Network, Settings } from 'lucide-react';
import { JSX } from 'react';
import DockerIcon from '@/assets/docker.min.svg?react';
import { CitadelIcons } from '@/lib/icons';
import { PlatformType } from '@/api/generated/api.types';
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
  disabled?: boolean;
  disabledReason?: string;
  access?: 'administrator' | 'alertRules' | 'bindings' | 'tags';
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
      {
        icon: renderIcon(CitadelIcons.BackupPolicy),
        label: 'Backups',
        route: '/backup-policies',
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
      {
        icon: renderIcon(CitadelIcons.SwarmService),
        label: 'Swarm Services',
        route: '/swarm-services',
      },
      {
        icon: renderIcon(CitadelIcons.AutomationAction),
        label: 'Automation',
        route: '/automation',
      },
    ],
  },
  {
    group: 'Builds',
    separator: false,
    items: [
      {
        icon: renderIcon(CitadelIcons.Build),
        label: 'Build Projects',
        route: '/builds',
      },
      {
        icon: renderIcon(CitadelIcons.BuildAgentPool),
        label: 'Build Pools',
        route: '/build-pools',
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
            access: 'alertRules',
          },
          {
            icon: renderIcon(CitadelIcons.Binding),
            label: 'Bindings',
            route: '/bindings',
            access: 'bindings',
          },
          {
            icon: renderIcon(CitadelIcons.Tag),
            label: 'Tags',
            route: '/tags',
            access: 'tags',
          },
          {
            icon: renderIcon(CitadelIcons.OidcProvider),
            label: 'OIDC Providers',
            route: '/oidc-providers',
            access: 'administrator',
          },
          {
            icon: renderIcon(CitadelIcons.Access),
            label: 'Access',
            route: '/access',
            access: 'administrator',
          },
          {
            icon: <KeyRound className="w-3.5 h-3.5" />,
            label: 'License',
            route: '/license',
            access: 'administrator',
          },
        ],
      },
    ],
  },
];

const DockerPlatformMenu = (platform: { id: string; name: string }): ISubMenuItem => ({
  icon: <DockerIcon />,
  label: platform.name,
  route: `/platforms/edit/${platform.id}`,
  isPlatform: true,
  children: [
    { label: 'Containers', route: `/platforms/${platform.id}/containers` },
    { label: 'Images', route: `/platforms/${platform.id}/images` },
    { label: 'Networks', route: `/platforms/${platform.id}/networks` },
    { label: 'Volumes', route: `/platforms/${platform.id}/volumes` },
  ],
});

const SwarmPlatformMenu = (platform: { id: string; name: string }): ISubMenuItem => ({
  icon: <Network className="h-3.5 w-3.5" aria-label="Docker Swarm" />,
  label: platform.name,
  route: `/platforms/edit/${platform.id}`,
  isPlatform: true,
  children: [
    { label: 'Nodes', route: `/platforms/${platform.id}/nodes` },
    { label: 'Services', route: `/platforms/${platform.id}/services` },
    { label: 'Tasks', route: `/platforms/${platform.id}/tasks` },
    { label: 'Secrets', route: `/platforms/${platform.id}/secrets` },
    { label: 'Configs', route: `/platforms/${platform.id}/configs` },
    { label: 'Containers', route: `/platforms/${platform.id}/containers` },
    { label: 'Volumes', route: `/platforms/${platform.id}/volumes` },
    { label: 'Networks', route: `/platforms/${platform.id}/networks` },
    { label: 'Images', route: `/platforms/${platform.id}/images` },
  ],
});

const PlatformMenu = (platform: { id: string; name: string; type: PlatformType }): ISubMenuItem =>
  platform.type === PlatformType.DockerSwarm ? SwarmPlatformMenu(platform) : DockerPlatformMenu(platform);

export { MenuItems, type IMenuItem, type ISubMenuItem, DockerPlatformMenu, SwarmPlatformMenu, PlatformMenu };
