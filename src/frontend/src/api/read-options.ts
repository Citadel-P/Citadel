import { queryOptions } from '@tanstack/react-query';
import { Api } from './generated/api.types';
import { resources } from './generated/resources';
import { invokeResource, resourceQueryKey } from './dispatch';
import type { KnownResourceName, ResourceArgs, UseReadArgs } from './types';

/** Shared HTTP query contract; feature helpers supply arguments and explicit cache effects. */
export function resourceReadOptions<K extends KnownResourceName>(
  client: Api<unknown>,
  resource: K,
  args?: NoInfer<UseReadArgs<K>>,
) {
  const definition = resources[resource];
  const enabled =
    !!client &&
    definition.requiredParams.every((name) => (args as Record<string, unknown> | undefined)?.[name] != null) &&
    definition.requiredQueryParams.every(
      (name) => (args as { query?: Record<string, unknown> } | undefined)?.query?.[name] != null,
    );
  return queryOptions({
    queryKey: resourceQueryKey<K>(resource, args),
    enabled,
    queryFn: ({ signal }) => {
      if (!enabled) throw new Error(`Missing required arguments for ${resource}`);
      const request = (args as { params?: { signal?: AbortSignal | null } })?.params;
      return invokeResource<K>(client, resource, (args ?? {}) as ResourceArgs<K>, {
        signal: request?.signal ? AbortSignal.any([signal, request.signal]) : signal,
      });
    },
  });
}
