import { PlatformCapabilitiesView } from '@/api/generated/api.types';
import { SwarmInventoryUpdate, useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useQueryClient } from '@tanstack/react-query';
import { useCallback } from 'react';
import { useNavigate } from 'react-router';

type QueryState<T> = { data?: { data?: T } };
type CachedResponse<T> = { data: T };
type LiveSwarmResource = { id: string; capabilities?: PlatformCapabilitiesView | null };
type SwarmCollection<T> = { items: T[]; capabilities?: PlatformCapabilitiesView | null };

export const useLiveSwarmItems = <T extends LiveSwarmResource>(
  platformId: string,
  queryName: string,
  queryArgs: object,
  query: QueryState<SwarmCollection<T>>,
  select: (inventory: SwarmInventoryUpdate) => T[],
  onInventoryUpdated?: (inventory: SwarmInventoryUpdate) => void,
) => {
  const queryClient = useQueryClient();
  const onSwarmInventoryUpdated = useCallback(
    (inventory: SwarmInventoryUpdate) => {
      if (inventory.platformId !== platformId) return;

      const queryKey = [queryName, queryArgs] as const;
      const previous = queryClient.getQueryData<CachedResponse<SwarmCollection<T>>>(queryKey);
      const items = select(inventory).map((item) => ({
        ...item,
        capabilities: previous?.data.capabilities,
      }));
      queryClient.setQueryData<CachedResponse<SwarmCollection<T>>>(
        queryKey,
        previous ? { ...previous, data: { ...previous.data, items } } : { data: { items } },
      );
      void queryClient.cancelQueries({ queryKey, exact: true }, { revert: false });
      onInventoryUpdated?.(inventory);
    },
    [onInventoryUpdated, platformId, queryArgs, queryClient, queryName, select],
  );

  useDockerDaemonGroup(platformId, { onSwarmInventoryUpdated });
  return query.data?.data?.items ?? [];
};

export const useLiveSwarmResource = <T extends LiveSwarmResource>(
  platformId: string,
  resourceId: string,
  queryName: string,
  queryArgs: object,
  listPath: string,
  query: QueryState<T>,
  select: (inventory: SwarmInventoryUpdate) => T[],
) => {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const onSwarmInventoryUpdated = useCallback(
    (inventory: SwarmInventoryUpdate) => {
      if (inventory.platformId !== platformId) return;

      const current = select(inventory).find((item) => item.id === resourceId);
      if (!current) {
        navigate(listPath, { replace: true });
        return;
      }

      const queryKey = [queryName, queryArgs] as const;
      const previous = queryClient.getQueryData<CachedResponse<T>>(queryKey);
      queryClient.setQueryData<CachedResponse<T>>(queryKey, {
        ...previous,
        data: {
          ...current,
          capabilities: previous?.data.capabilities,
        },
      });
      void queryClient.cancelQueries({ queryKey, exact: true }, { revert: false });
    },
    [listPath, navigate, platformId, queryArgs, queryClient, queryName, resourceId, select],
  );

  useDockerDaemonGroup(platformId, { onSwarmInventoryUpdated });
  return query.data?.data;
};
