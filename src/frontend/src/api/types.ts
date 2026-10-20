import { Api, PlatformStatView } from './generated/api.types';
import { ResourceName, resources } from '@/api/generated/resources';

export type DockerResourceType = 'Network' | 'Volume' | 'Image' | 'Container';
export type SwarmResourceType = 'Node' | 'Service' | 'Task' | 'Secret' | 'Config';

export const SwarmResourcePathMap = {
  nodes: 'Node',
  services: 'Service',
  tasks: 'Task',
  secrets: 'Secret',
  configs: 'Config',
} as const satisfies Record<string, SwarmResourceType>;
export type ResourceType =
  | DockerResourceType
  | SwarmResourceType
  | 'Registry'
  | 'Platform'
  | 'Deployment'
  | 'SwarmService'
  | 'Activity'
  | 'AlertRule'
  | 'AlertChannel'
  | 'Alert'
  | 'License'
  | 'Stack'
  | 'GitRepository'
  | 'GitAccount'
  | 'Webhook'
  | 'Automation'
  | 'AutomationAction'
  | 'Build'
  | 'BuildAgentPool'
  | 'BackupRepository'
  | 'BackupPolicy'
  | 'Access'
  | 'User'
  | 'Team'
  | 'Role'
  | 'ServiceAccount'
  | 'Binding'
  | 'Tag'
  | 'OidcProvider';
export const PluralResourceMap = {
  Network: 'Networks',
  Volume: 'Volumes',
  Image: 'Images',
  Container: 'Containers',
  Node: 'Nodes',
  Service: 'Services',
  Task: 'Tasks',
  Secret: 'Secrets',
  Config: 'Configs',
  Registry: 'Registries',
  Platform: 'Platforms',
  Deployment: 'Deployments',
  SwarmService: 'SwarmServices',
  Activity: 'Activities',
  AlertRule: 'AlertRules',
  Alert: 'Alerts',
  License: 'License',
  Stack: 'Stacks',
  AlertChannel: 'AlertChannels',
  GitRepository: 'GitRepositories',
  GitAccount: 'GitAccounts',
  Webhook: 'Webhooks',
  Automation: 'Automation',
  AutomationAction: 'Actions',
  Build: 'Builds',
  BuildAgentPool: 'BuildPools',
  BackupRepository: 'BackupRepositories',
  BackupPolicy: 'BackupPolicies',
  Access: 'Access',
  User: 'Users',
  Team: 'Teams',
  Role: 'Roles',
  ServiceAccount: 'service-accounts',
  Binding: 'Bindings',
  Tag: 'Tags',
  OidcProvider: 'OidcProviders',
} as const satisfies Record<ResourceType, string>;

/** Generic data hooks intentionally expose only the client's api namespace. */
export type ApiFnMap = Api<unknown>['api'];
export type KnownResourceName = Extract<ResourceName, keyof ApiFnMap>;
export type ApiFn<K extends KnownResourceName> = ApiFnMap[K];
export type ResourceResponse<K extends KnownResourceName> = Awaited<ReturnType<ApiFn<K>>>;

type NamedParameters<K extends KnownResourceName> = {
  [I in Exclude<
    keyof (typeof resources)[K]['params'],
    keyof (readonly unknown[])
  > as (typeof resources)[K]['params'][I] extends string
    ? (typeof resources)[K]['params'][I]
    : never]: I extends keyof Parameters<ApiFn<K>> ? Parameters<ApiFn<K>>[I] : never;
};

/** One named argument object, ordered only at the generated-client boundary. */
type NamedArgs<K extends KnownResourceName> = {
  [P in keyof NamedParameters<K> as undefined extends NamedParameters<K>[P] ? never : P]: NamedParameters<K>[P];
} & {
  [P in keyof NamedParameters<K> as undefined extends NamedParameters<K>[P] ? P : never]?: NamedParameters<K>[P];
};
export type ResourceArgs<K extends KnownResourceName> = K extends KnownResourceName ? NamedArgs<K> : never;
// Reads may await route/context arguments; useRead gates dispatch until required values exist.
export type UseReadArgs<K extends KnownResourceName> = Partial<ResourceArgs<K>>;
export type UseMutateVariables<K extends KnownResourceName> = ResourceArgs<K>;

export interface Cancellable {
  signal: AbortSignal;
}

export interface PlatformStatsBatchView {
  platformId: string;
  networkCount: number;
  volumeCount: number;
  containerCount: number;
  containersRunning: number;
  containersPaused: number;
  containersStopped: number;
  imageCount: number;
  imageUsedBytes: number | null;
  memTotal: number;
  volumeUsedBytes: number | null;
  stat: PlatformStatView;
}

export interface DeleteDialogConfig<TItem, TRequest> {
  type: ResourceType;
  getIds: (items: TItem[]) => string[];
  buildRequest: (ids: string[], toggles: Record<string, boolean>) => TRequest;
  toggles?: {
    key: string;
    label: string;
    description: string;
    default?: boolean;
  }[];
}

export const ReversePluralResourceMap = Object.fromEntries(
  Object.entries(PluralResourceMap).map(([k, v]) => [v, k]),
) as {
  [V in (typeof PluralResourceMap)[keyof typeof PluralResourceMap]]: keyof typeof PluralResourceMap;
};

export type TargetResource = 'Deployment' | 'Container' | 'Stack';
