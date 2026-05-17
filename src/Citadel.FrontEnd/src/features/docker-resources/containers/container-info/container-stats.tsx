import { ContainerStateStatus, ContainerStatView } from '@/api/generated/api.types';
import { DockerContainerView } from '@/api/types';
import { useRead } from '@/lib/hooks';
import { useState, useMemo, useEffect, useRef } from 'react';
import MemoryUsage from './stats/memory-usage';
import CpuUsage from './stats/cpu-usage';
import NetworkUsage from './stats/network-usage';

type StatsPanelProps = {
  resource: DockerContainerView | undefined;
  baseStats: ContainerStatView[];
  isLoading: boolean;
};

const StatsPanel = ({ resource, baseStats, isLoading }: StatsPanelProps) => {
  const [liveStats, setLiveStats] = useState<ContainerStatView[]>([]);
  const lastStatRef = useRef<ContainerStatView | undefined>(resource?.containerStat);

  useEffect(() => {
    const stat = resource?.containerStat;
    const isRunning = resource?.state === ContainerStateStatus.Running;

    if (isRunning && stat && stat !== lastStatRef.current) {
      lastStatRef.current = stat;
      setLiveStats((prev) => [...prev, { ...stat, created: Math.floor(Date.now() / 1000) }]);
    }
  }, [resource?.containerStat, resource?.state]);

  const stats = useMemo(() => {
    const base = baseStats ?? [];
    return [...base, ...liveStats].sort((a, b) => Number(a.created) - Number(b.created));
  }, [baseStats, liveStats]);

  return (
    <div className="flex flex-col gap gap-y-4">
      <MemoryUsage container={resource} stats={stats} isLoading={isLoading} />
      <CpuUsage container={resource} stats={stats} isLoading={isLoading} />
      <NetworkUsage container={resource} stats={stats} isLoading={isLoading} />
    </div>
  );
};

export const ContainerStats = ({ resource }: { resource: DockerContainerView | undefined }) => {
  const { data, isLoading } = useRead('getContainerStats', { id: resource?.id });
  const baseStats = data?.data?.stats ?? [];
  return <StatsPanel resource={resource} baseStats={baseStats} isLoading={isLoading} />;
};

export const DeploymentStats = ({
  resource,
  deploymentId,
}: {
  resource: DockerContainerView | undefined;
  deploymentId: string;
}) => {
  const { data, isLoading } = useRead('getDeploymentStats', { id: deploymentId });
  const baseStats = data?.data?.stats ?? [];
  return <StatsPanel resource={resource} baseStats={baseStats} isLoading={isLoading} />;
};
