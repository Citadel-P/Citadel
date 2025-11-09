import { ContainerStateStatus, ContainerStatView } from '@/api/generated/api.types';
import { DockerContainerView } from '@/api/types';
import { useRead } from '@/lib/hooks';
import { useEffect, useState } from 'react';
import MemoryUsage from './stats/memory-usage';
import CpuUsage from './stats/cpu-usage';
import NetworkUsage from './stats/network-usage';

export const ContainerStats = ({ resource }: { resource: DockerContainerView | undefined }) => {
  const { data, isSuccess, isLoading } = useRead('getContainerStats', { id: resource?.id });
  const [stats, setStats] = useState<ContainerStatView[]>([]);

  useEffect(() => {
    if (isSuccess && data?.data) {
      setStats(data?.data.stats);
    }
  }, [isSuccess, data]);

  useEffect(() => {
    if (resource?.containerStat && resource?.state === ContainerStateStatus.Running) {
      resource.containerStat.created = Math.floor(Date.now() / 1000);
      setStats((prev) => [...prev, resource.containerStat]);
    }
  }, [resource]);

  return (
    <div className="flex flex-col gap gap-y-4">
      <MemoryUsage container={resource} stats={stats} isLoading={isLoading} />
      <CpuUsage container={resource} stats={stats} isLoading={isLoading} />
      <NetworkUsage container={resource} stats={stats} isLoading={isLoading} />
    </div>
  );
};
