import { useQuery, useQueryClient, type QueryClient } from '@tanstack/react-query';
import { useApiClientContext } from '@/api/api-client-context';
import { resourceQueryKey } from '@/api/dispatch';
import { resourceReadOptions } from '@/api/read-options';
import type { Api } from '@/api/generated/api.types';
import type { ResourceArgs } from '@/api/types';
import { useMutate } from '@/lib/hooks';

type RegistryListArgs = ResourceArgs<'listRegistries'>;

export const registryKeys = {
  lists: ['listRegistries'] as const,
  list: (args: RegistryListArgs = {}) => resourceQueryKey('listRegistries', args),
  config: (id?: string) => resourceQueryKey('getRegistryConfig', { id }),
};

export const registryQueries = {
  list: (client: Api<unknown>, args: RegistryListArgs = {}) => resourceReadOptions(client, 'listRegistries', args),
  config: (client: Api<unknown>, id?: string) => resourceReadOptions(client, 'getRegistryConfig', { id }),
};

export function useRegistryList(args: RegistryListArgs) {
  const { apiClient } = useApiClientContext();
  return useQuery(registryQueries.list(apiClient, args));
}

export function useRegistryConfig(id?: string) {
  const { apiClient } = useApiClientContext();
  return useQuery(registryQueries.config(apiClient, id));
}

// Every filtered list may change membership. Only affected configuration reads are stale.
export async function invalidateRegistries(client: QueryClient, ids: readonly string[] = []) {
  await Promise.all([
    client.invalidateQueries({ queryKey: registryKeys.lists }),
    ...ids.map((id) => client.invalidateQueries({ queryKey: registryKeys.config(id), exact: true })),
  ]);
}

export function useCreateRegistry() {
  const client = useQueryClient();
  return useMutate('createRegistry', { onSuccess: () => invalidateRegistries(client) });
}

export function useUpdateRegistry() {
  const client = useQueryClient();
  return useMutate('updateRegistry', { onSuccess: (_response, { id }) => invalidateRegistries(client, [id]) });
}

export function useRegistryDeletionEffects(ids: readonly string[]) {
  const client = useQueryClient();
  return () => invalidateRegistries(client, ids);
}
