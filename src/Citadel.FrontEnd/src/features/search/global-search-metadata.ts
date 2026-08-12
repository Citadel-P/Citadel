import { GlobalSearchCategory, GlobalSearchResourceType } from '@/api/generated/api.types';
import type { ResourceType } from '@/api/types';
import { CitadelIcons } from '@/lib/icons';
import type { LucideIcon } from 'lucide-react';

type ResourceSearchMetadata = {
  category: GlobalSearchCategory;
  label: string;
  singularLabel: string;
  icon: LucideIcon;
  listPath: string;
  getDetailsPath: (id: string) => string;
};

type CategorySearchMetadata = {
  category: GlobalSearchCategory;
  label: string;
  icon: LucideIcon;
  listPath: string;
};

const iconFor = (type: GlobalSearchResourceType) => CitadelIcons[type as ResourceType] as LucideIcon;

export const globalSearchResourceMetadata: Record<GlobalSearchResourceType, ResourceSearchMetadata> = {
  [GlobalSearchResourceType.Platform]: {
    category: GlobalSearchCategory.Platforms,
    label: 'Platforms',
    singularLabel: 'Platform',
    icon: iconFor(GlobalSearchResourceType.Platform),
    listPath: '/platforms',
    getDetailsPath: (id) => `/platforms/edit/${id}`,
  },
  [GlobalSearchResourceType.Stack]: {
    category: GlobalSearchCategory.Stacks,
    label: 'Stacks',
    singularLabel: 'Stack',
    icon: iconFor(GlobalSearchResourceType.Stack),
    listPath: '/stacks',
    getDetailsPath: (id) => `/stacks/edit/${id}`,
  },
  [GlobalSearchResourceType.Deployment]: {
    category: GlobalSearchCategory.Deployments,
    label: 'Deployments',
    singularLabel: 'Deployment',
    icon: iconFor(GlobalSearchResourceType.Deployment),
    listPath: '/deployments',
    getDetailsPath: (id) => `/deployments/edit/${id}`,
  },
  [GlobalSearchResourceType.SwarmService]: {
    category: GlobalSearchCategory.SwarmServices,
    label: 'Swarm Services',
    singularLabel: 'Swarm Service',
    icon: iconFor(GlobalSearchResourceType.SwarmService),
    listPath: '/swarm-services',
    getDetailsPath: (id) => `/swarm-services/edit/${id}`,
  },
  [GlobalSearchResourceType.GitRepository]: {
    category: GlobalSearchCategory.Repositories,
    label: 'Repositories',
    singularLabel: 'Repository',
    icon: iconFor(GlobalSearchResourceType.GitRepository),
    listPath: '/git-repos',
    getDetailsPath: (id) => `/git-repos/edit/${id}`,
  },
  [GlobalSearchResourceType.Registry]: {
    category: GlobalSearchCategory.Registries,
    label: 'Registries',
    singularLabel: 'Registry',
    icon: iconFor(GlobalSearchResourceType.Registry),
    listPath: '/registries',
    getDetailsPath: (id) => `/registries/edit/${id}`,
  },
  [GlobalSearchResourceType.AutomationAction]: {
    category: GlobalSearchCategory.Automations,
    label: 'Automations',
    singularLabel: 'Automation',
    icon: iconFor(GlobalSearchResourceType.AutomationAction),
    listPath: '/automation',
    getDetailsPath: (id) => `/automation/edit/${id}`,
  },
  [GlobalSearchResourceType.BackupPolicy]: {
    category: GlobalSearchCategory.Backups,
    label: 'Backups',
    singularLabel: 'Backup policy',
    icon: iconFor(GlobalSearchResourceType.BackupPolicy),
    listPath: '/backup-policies',
    getDetailsPath: (id) => `/backup-policies/edit/${id}`,
  },
  [GlobalSearchResourceType.BackupRepository]: {
    category: GlobalSearchCategory.Backups,
    label: 'Backups',
    singularLabel: 'Backup repository',
    icon: iconFor(GlobalSearchResourceType.BackupRepository),
    listPath: '/backup-repositories',
    getDetailsPath: (id) => `/backup-repositories/edit/${id}`,
  },
  [GlobalSearchResourceType.Build]: {
    category: GlobalSearchCategory.Builds,
    label: 'Builds',
    singularLabel: 'Build project',
    icon: iconFor(GlobalSearchResourceType.Build),
    listPath: '/builds',
    getDetailsPath: (id) => `/builds/edit/${id}`,
  },
  [GlobalSearchResourceType.BuildAgentPool]: {
    category: GlobalSearchCategory.Builds,
    label: 'Builds',
    singularLabel: 'Build pool',
    icon: iconFor(GlobalSearchResourceType.BuildAgentPool),
    listPath: '/build-pools',
    getDetailsPath: (id) => `/build-pools/edit/${id}`,
  },
};

export const globalSearchCategories: CategorySearchMetadata[] = [
  {
    category: GlobalSearchCategory.Platforms,
    label: 'Platforms',
    icon: iconFor(GlobalSearchResourceType.Platform),
    listPath: '/platforms',
  },
  {
    category: GlobalSearchCategory.Stacks,
    label: 'Stacks',
    icon: iconFor(GlobalSearchResourceType.Stack),
    listPath: '/stacks',
  },
  {
    category: GlobalSearchCategory.Deployments,
    label: 'Deployments',
    icon: iconFor(GlobalSearchResourceType.Deployment),
    listPath: '/deployments',
  },
  {
    category: GlobalSearchCategory.SwarmServices,
    label: 'Swarm Services',
    icon: iconFor(GlobalSearchResourceType.SwarmService),
    listPath: '/swarm-services',
  },
  {
    category: GlobalSearchCategory.Repositories,
    label: 'Repositories',
    icon: iconFor(GlobalSearchResourceType.GitRepository),
    listPath: '/git-repos',
  },
  {
    category: GlobalSearchCategory.Registries,
    label: 'Registries',
    icon: iconFor(GlobalSearchResourceType.Registry),
    listPath: '/registries',
  },
  {
    category: GlobalSearchCategory.Automations,
    label: 'Automations',
    icon: iconFor(GlobalSearchResourceType.AutomationAction),
    listPath: '/automation',
  },
  {
    category: GlobalSearchCategory.Backups,
    label: 'Backups',
    icon: iconFor(GlobalSearchResourceType.BackupPolicy),
    listPath: '/backup-policies',
  },
  {
    category: GlobalSearchCategory.Builds,
    label: 'Builds',
    icon: iconFor(GlobalSearchResourceType.Build),
    listPath: '/builds',
  },
];
