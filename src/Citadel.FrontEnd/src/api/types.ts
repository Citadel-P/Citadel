import { Api, ContainerStateStatus, ContainerStatView, PlatformStatView } from './generated/api.types';
import { ResourceName, resources } from '@/api/generated/resources';
import { useApiClientContext } from '@/api/api-client-context';

export type DockerResourceType = 'Network' | 'Volume' | 'Image' | 'Container';
export type ResourceType = DockerResourceType | 'Registry' | 'Platform' | 'Deployment';
export const PluralResourceMap = {
  Network: 'Networks',
  Volume: 'Volumes',
  Image: 'Images',
  Container: 'Containers',
  Registry: 'Registries',
  Platform: 'Platforms',
  Deployment: 'Deployments',
} as const satisfies Record<ResourceType, string>;

export type AnyFn = (...args: any[]) => Promise<any>;

/** Type of the 'api' object on an instance of the generated Api class. */
type ApiMethods = Api<any>['api'];

/** Keep only resource names that actually exist on the generated client's api map */
export type KnownResourceName = Extract<ResourceName, keyof ApiMethods>;

/** Function type for a given known resource */
type ResourceFn<K extends KnownResourceName> = ApiMethods[K];

/** Positional parameter tuple for the resource function */
type ResourceParams<K extends KnownResourceName> = Parameters<ResourceFn<K>>;

/** Resolved response type for the resource function */
export type ResourceResponse<K extends KnownResourceName> = Awaited<ReturnType<ResourceFn<K>>>;

type ResourceArgs<K extends KnownResourceName> = Zip<
  (typeof resources)[K]['params'] extends readonly string[] ? (typeof resources)[K]['params'] : readonly string[],
  ResourceParams<K>
>;

type RequiredArgs<K extends KnownResourceName> = {
  [P in keyof ResourceArgs<K> as undefined extends ResourceArgs<K>[P] ? never : P]: ResourceArgs<K>[P];
};

type OptionalArgs<K extends KnownResourceName> = {
  [P in keyof ResourceArgs<K> as undefined extends ResourceArgs<K>[P] ? P : never]?: ResourceArgs<K>[P];
};

export type UseReadArgs<T extends KnownResourceName> = keyof RequiredArgs<T> extends never
  ? (Partial<ArgsFromParams<T>> & { query?: QueryFromResource<T>; params?: RequestParams }) | undefined
  : RequiredArgs<T> &
      Partial<OptionalArgs<T>> & {
        query?: QueryFromResource<T>;
        params?: RequestParams;
      };

type QueryFromResource<K extends KnownResourceName> = (typeof resources)[K]['queryParams'] extends readonly string[]
  ? { [P in (typeof resources)[K]['queryParams'][number]]?: any }
  : never;

type RequestParams = {
  headers?: Record<string, string>;
  query?: Record<string, any>;
  [key: string]: any;
};

type Zip<Names extends readonly string[], Types extends readonly any[]> = {
  [I in keyof Names as Names[I] extends string ? Names[I] : never]: I extends keyof Types ? Types[I] : unknown;
};

type ArgsFromParams<K extends KnownResourceName> = Zip<
  (typeof resources)[K]['params'] extends readonly string[] ? (typeof resources)[K]['params'] : readonly string[],
  ResourceParams<K>
>;

// Infer API client method types
type ApiClientType = ReturnType<typeof useApiClientContext>['apiClient'];
export type ApiFnMap = ApiClientType['api'];
export type ApiFn<TResource extends keyof ApiFnMap> = ApiFnMap[TResource];

type ApiFnParams<T extends keyof ApiFnMap> = Parameters<ApiFn<T>>;
export type PrimaryArg<T extends keyof ApiFnMap> = ApiFnParams<T>[0];

export type MutateVariables<TResource extends keyof typeof resources> = {
  [K in (typeof resources)[TResource]['requiredParams'][number]]: string;
} & {
  [K in Exclude<
    (typeof resources)[TResource]['params'][number],
    (typeof resources)[TResource]['requiredParams'][number]
  >]?: any;
};

type NamedArgsForResource<K extends KnownResourceName> = keyof RequiredArgs<K> extends never
  ? Partial<ArgsFromParams<K>>
  : RequiredArgs<K> & Partial<OptionalArgs<K>>;

export type UseMutateVariables<TResource extends KnownResourceName> =
  | PrimaryArg<TResource>
  | NamedArgsForResource<TResource>;

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
  memTotal: number;
  stat: PlatformStatView;
}

export interface DockerContainerView {
  id: string;
  name: string;
  state: ContainerStateStatus;
  created: number | null;
  stack: string | null;
  containerStat: ContainerStatView;
  containerPort: null | [];
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
