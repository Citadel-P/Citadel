import { Api, RequestParams } from './generated/api.types';
import { resources } from './generated/resources';
import type { KnownResourceName, ResourceArgs, ResourceResponse } from './types';

export function invokeResource<K extends KnownResourceName>(
  client: Api<unknown>,
  resource: K,
  args: NoInfer<ResourceArgs<K>>,
  requestParams?: RequestParams,
): Promise<ResourceResponse<K>> {
  const named = args as Record<string, unknown>;
  const positional = resources[resource].params.map((name) =>
    name === 'params' ? { ...(named.params as RequestParams), ...requestParams } : named[name],
  );
  // The generated metadata supplies the exact order; callers remain checked
  // against the corresponding generated method's parameter types.
  const fn = client.api[resource] as (...args: unknown[]) => Promise<ResourceResponse<K>>;
  return fn(...positional);
}

/** Transport options (including credentials and signals) never identify data. */
export function resourceQueryKey<K extends KnownResourceName>(resource: K, args?: Partial<ResourceArgs<K>>) {
  const { params: _params, ...identity } = (args ?? {}) as Record<string, unknown>;
  return [resource, identity] as const;
}
