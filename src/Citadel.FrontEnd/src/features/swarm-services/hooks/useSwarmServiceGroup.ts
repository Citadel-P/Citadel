import { useCallback } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useQueryClient } from '@tanstack/react-query';
import { ManagedSwarmServiceView, SwarmServiceSynchronizationState } from '@/api/generated/api.types';
import { ResourceResponse } from '@/api/types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';

export const normalizeManagedSwarmService = (
  current: ManagedSwarmServiceView | undefined,
  update: ManagedSwarmServiceView,
): ManagedSwarmServiceView => {
  const image = normalizeMessagePackUnion(update.spec.image);
  const normalizedImage = isSwarmServiceImage(image) ? image : (current?.spec.image ?? image);

  return {
    ...update,
    capabilities: update.capabilities ?? current?.capabilities,
    tasks:
      update.tasks ??
      (update.synchronizationState === SwarmServiceSynchronizationState.RuntimeMissing ? [] : current?.tasks ?? []),
    spec: {
      ...update.spec,
      image: normalizedImage as ManagedSwarmServiceView['spec']['image'],
    },
  };
};

const isSwarmServiceImage = (value: unknown): value is ManagedSwarmServiceView['spec']['image'] =>
  value !== null &&
  typeof value === 'object' &&
  !Array.isArray(value) &&
  ('$type' in value && (value.$type === 'External' || value.$type === 'Build'));

const normalizeMessagePackUnion = (value: unknown): unknown => {
  if (
    !Array.isArray(value) ||
    typeof value[0] !== 'string' ||
    value[1] === null ||
    typeof value[1] !== 'object' ||
    Array.isArray(value[1])
  ) {
    return value;
  }

  return { $type: value[0], ...value[1] };
};

export const useSwarmServiceGroup = (id: string) => {
  const { data, isLoading } = useRead('getManagedSwarmService', { id });
  const queryClient = useQueryClient();
  const onUpdated = useCallback(
    (service: ManagedSwarmServiceView, action: string) => {
      if (service.id !== id || action === 'delete') return;
      queryClient.setQueryData<ResourceResponse<'getManagedSwarmService'>>(
        ['getManagedSwarmService', { id }],
        (current) => (current ? { ...current, data: normalizeManagedSwarmService(current.data, service) } : current),
      );
    },
    [id, queryClient],
  );
  const setupEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.on('SwarmServiceInfoUpdated', onUpdated),
    [onUpdated],
  );
  const removeEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.off('SwarmServiceInfoUpdated', onUpdated),
    [onUpdated],
  );
  useRealtimeGroup({ groupName: `swarm-service:${id}`, skip: !id, setupEventListeners, removeEventListeners });
  return { service: data?.data, isLoading };
};
