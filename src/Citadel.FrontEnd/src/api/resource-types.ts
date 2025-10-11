import type { Api as GeneratedApi } from '@/api/generated/api.types';
import { resources } from '@/api/generated/resources';
import type { ResourceName } from '@/api/generated/resources';

/** Type of the 'api' object on an instance of the generated Api class. */
type ApiMethods = GeneratedApi<any>['api'];

/** Keep only resource names that actually exist on the generated client's api map */
export type KnownResourceName = Extract<ResourceName, keyof ApiMethods>;

/** Function type for a given known resource */
export type ResourceFn<K extends KnownResourceName> = ApiMethods[K];

/** Positional parameter tuple for the resource function */
export type ResourceParams<K extends KnownResourceName> = Parameters<ResourceFn<K>>;

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
/**
 * Zip two tuples: names (string literals) and types (tuple of types) -> object.
 * Example:
 *   Zip<['id','query','params'], [string, QueryType | undefined, RequestParams?]>
 *   -> { id: string; query: QueryType | undefined; params: RequestParams | undefined }
 */
type Zip<Names extends readonly string[], Types extends readonly any[]> = {
  [I in keyof Names as Names[I] extends string ? Names[I] : never]: I extends keyof Types ? Types[I] : unknown;
};

/**
 * Given a resource key K, build a named-args object type from:
 *  - the generated resources[K].params (a readonly string tuple), and
 *  - the positional parameter tuple ResourceParams<K>
 */
export type ArgsFromParams<K extends KnownResourceName> = Zip<
  // Resources.ts is generated `as const` so the type of resources[K].params is a readonly tuple of literal strings
  (typeof resources)[K]['params'] extends readonly string[] ? (typeof resources)[K]['params'] : readonly string[],
  ResourceParams<K>
>;
