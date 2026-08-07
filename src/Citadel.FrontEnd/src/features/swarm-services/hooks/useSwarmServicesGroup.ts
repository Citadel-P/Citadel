import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { ManagedSwarmServiceView, ResourceCapabilities } from '@/api/generated/api.types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';
import { useResourceTagFilter } from '@/features/tags/components';
import { useResourcePlatformFilter } from '@/features/platforms/platform-filter';
import { SwarmInventoryUpdate } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { normalizeManagedSwarmService } from './useSwarmServiceGroup';

const EMPTY_TAG_NAMES: string[] = [];

export const useSwarmServicesGroup = (platformIdOverride?: string) => {
  const { selectedTagNames } = useResourceTagFilter();
  const { selectedPlatformId } = useResourcePlatformFilter();
  const effectivePlatformId = platformIdOverride ?? selectedPlatformId;
  const effectiveTagNames = platformIdOverride ? EMPTY_TAG_NAMES : selectedTagNames;
  const listenForInventory = platformIdOverride === undefined;
  const args = useMemo(() => {
    const query: { tags?: string[]; platformId?: string } = {};
    if (effectiveTagNames.length) query.tags = effectiveTagNames;
    if (effectivePlatformId) query.platformId = effectivePlatformId;
    return Object.keys(query).length ? { query } : undefined;
  }, [effectivePlatformId, effectiveTagNames]);
  const { data, isLoading } = useRead('listManagedSwarmServices', args);
  const [services, setServices] = useState<ManagedSwarmServiceView[]>();
  const [capabilities, setCapabilities] = useState<ResourceCapabilities>();
  const lastResult = useRef<ManagedSwarmServiceView[]>(undefined);

  useEffect(() => {
    if (!data || lastResult.current === data.data.swarmServices) return;
    lastResult.current = data.data.swarmServices;
    setServices(data.data.swarmServices);
    setCapabilities(data.data.capabilities);
  }, [data]);

  const matchesFilters = useCallback(
    (service: ManagedSwarmServiceView) => {
      if (effectivePlatformId && service.platformId !== effectivePlatformId) return false;
      if (!effectiveTagNames.length) return true;
      const tags = new Set(service.tags.map((tag) => tag.name.trim().toLowerCase()));
      return effectiveTagNames.every((tag) => tags.has(tag.trim().toLowerCase()));
    },
    [effectivePlatformId, effectiveTagNames],
  );

  const onUpdated = useCallback(
    (service: ManagedSwarmServiceView, action: string) => {
      setServices((current) => {
        if (!current) return current;
        if (action === 'delete') return current.filter((item) => item.id !== service.id);
        const index = current.findIndex((item) => item.id === service.id);
        if (!matchesFilters(service)) return index < 0 ? current : current.filter((item) => item.id !== service.id);
        if (index < 0) return [...current, normalizeManagedSwarmService(undefined, service)];
        const next = [...current];
        next[index] = normalizeManagedSwarmService(current[index], service);
        return next;
      });
    },
    [matchesFilters],
  );

  const onSwarmInventoryUpdated = useCallback((inventory: SwarmInventoryUpdate) => {
    const runtimeByServiceId = new Map(inventory.services.items.map((item) => [item.id, item]));
    const tasksByServiceId = new Map<string, typeof inventory.tasks.items>();
    for (const task of inventory.tasks.items) {
      const tasks = tasksByServiceId.get(task.serviceId);
      if (tasks) tasks.push(task);
      else tasksByServiceId.set(task.serviceId, [task]);
    }

    setServices((current) =>
      current?.map((service) => {
        if (service.platformId !== inventory.platformId || !service.dockerServiceId) return service;

        const runtime = runtimeByServiceId.get(service.dockerServiceId);
        return {
          ...service,
          runningTaskCount: runtime?.runningTaskCount ?? null,
          desiredTaskCount: runtime?.desiredTaskCount ?? null,
          updateState: runtime?.updateState ?? null,
          tasks: tasksByServiceId.get(service.dockerServiceId) ?? [],
        };
      }),
    );
  }, []);

  const setupEventListeners = useCallback(
    (connection: HubConnection) => {
      connection.on('SwarmServiceInfoUpdated', onUpdated);
      if (listenForInventory) connection.on('SwarmInventoryUpdated', onSwarmInventoryUpdated);
    },
    [listenForInventory, onSwarmInventoryUpdated, onUpdated],
  );
  const removeEventListeners = useCallback(
    (connection: HubConnection) => {
      connection.off('SwarmServiceInfoUpdated', onUpdated);
      if (listenForInventory) connection.off('SwarmInventoryUpdated', onSwarmInventoryUpdated);
    },
    [listenForInventory, onSwarmInventoryUpdated, onUpdated],
  );
  const platformGroupKey = useMemo(
    () => effectivePlatformId ?? [...new Set((services ?? []).map((service) => service.platformId))].sort().join(','),
    [effectivePlatformId, services],
  );
  const groupNames = useMemo(
    () =>
      platformGroupKey
        ? platformGroupKey
            .split(',')
            .flatMap((platformId) => [
              `swarm-services:${platformId}`,
              ...(listenForInventory ? [`docker-daemon:${platformId}`] : []),
            ])
        : [],
    [listenForInventory, platformGroupKey],
  );
  useSignalRGroup({ groupName: groupNames, setupEventListeners, removeEventListeners });

  return { services, capabilities, isLoading };
};
