import { ContainerStateStatus, ContainerStatView, ContainerRuntimeView } from '@/api/generated/api.types';
import { StatsWindowHours, StatsWindowSelect } from '@/components/custom/common';
import { useRead } from '@/lib/hooks';
import { appendBoundedLiveStat, mergeStatsByCreated, STREAMED_STATS_QUERY_OPTIONS } from '@/lib/live-stats';
import { useState, useMemo, useEffect, useRef } from 'react';
import MemoryUsage from './stats/memory-usage';
import CpuUsage from './stats/cpu-usage';
import NetworkUsage from './stats/network-usage';
import type { ContainerStatsResource } from './stats/types';

type StatsPanelProps = {
  resource: ContainerStatsResource | undefined;
  memory: StatsQueryState;
  cpu: StatsQueryState;
  network: StatsQueryState;
  liveStats: Omit<ContainerStatView, 'containerId'>[];
};

export type StatsQueryState = {
  baseStats: Omit<ContainerStatView, 'containerId'>[];
  isLoading: boolean;
  windowHours: StatsWindowHours;
  onWindowHoursChange: (hours: StatsWindowHours) => void;
};

export const ContainerStatsCharts = ({ resource, memory, cpu, network, liveStats }: StatsPanelProps) => {
  const memoryStats = useCombinedStats(memory.baseStats, liveStats);
  const cpuStats = useCombinedStats(cpu.baseStats, liveStats);
  const networkStats = useCombinedStats(network.baseStats, liveStats);

  return (
    <div className="flex flex-col gap gap-y-4">
      <MemoryUsage
        container={resource}
        stats={memoryStats}
        isLoading={memory.isLoading}
        windowHours={memory.windowHours}
        controls={<StatsWindowSelect value={memory.windowHours} onChange={memory.onWindowHoursChange} />}
      />
      <CpuUsage
        container={resource}
        stats={cpuStats}
        isLoading={cpu.isLoading}
        windowHours={cpu.windowHours}
        controls={<StatsWindowSelect value={cpu.windowHours} onChange={cpu.onWindowHoursChange} />}
      />
      <NetworkUsage
        container={resource}
        stats={networkStats}
        isLoading={network.isLoading}
        windowHours={network.windowHours}
        controls={<StatsWindowSelect value={network.windowHours} onChange={network.onWindowHoursChange} />}
      />
    </div>
  );
};

const useLiveStats = (resource: ContainerRuntimeView | undefined) => {
  const [liveStats, setLiveStats] = useState<Omit<ContainerStatView, 'containerId'>[]>([]);
  const lastStatRef = useRef<ContainerStatView | undefined>(resource?.containerStat);
  const resourceIdRef = useRef<string | undefined>(resource?.id);

  useEffect(() => {
    if (resourceIdRef.current === resource?.id) {
      return;
    }

    resourceIdRef.current = resource?.id;
    setLiveStats([]);
    lastStatRef.current = resource?.containerStat;
  }, [resource?.containerStat, resource?.id]);

  useEffect(() => {
    const stat = resource?.containerStat;
    const isRunning = resource?.state === ContainerStateStatus.Running;

    if (isRunning && stat && stat !== lastStatRef.current) {
      lastStatRef.current = stat;
      setLiveStats((prev) =>
        appendBoundedLiveStat(prev, {
          ...stat,
          created: Number(stat.created) > 0 ? stat.created : Math.floor(Date.now() / 1000),
        }),
      );
    }
  }, [resource?.containerStat, resource?.state]);

  return liveStats;
};

const useCombinedStats = (
  baseStats: Omit<ContainerStatView, 'containerId'>[],
  liveStats: Omit<ContainerStatView, 'containerId'>[],
) => useMemo(() => mergeStatsByCreated(baseStats ?? [], liveStats), [baseStats, liveStats]);

const useContainerStatsWindow = (containerId: string | undefined): StatsQueryState => {
  const [windowHours, setWindowHours] = useState<StatsWindowHours>(24);
  const readArgs = useMemo(() => ({ id: containerId, query: { hours: windowHours } }), [containerId, windowHours]);
  const { data, isLoading } = useRead('getContainerStats', readArgs, STREAMED_STATS_QUERY_OPTIONS);

  return {
    baseStats: data?.data?.stats ?? [],
    isLoading,
    windowHours,
    onWindowHoursChange: setWindowHours,
  };
};

const useDeploymentStatsWindow = (deploymentId: string): StatsQueryState => {
  const [windowHours, setWindowHours] = useState<StatsWindowHours>(24);
  const readArgs = useMemo(() => ({ id: deploymentId, query: { hours: windowHours } }), [deploymentId, windowHours]);
  const { data, isLoading } = useRead('getDeploymentStats', readArgs, STREAMED_STATS_QUERY_OPTIONS);

  return {
    baseStats: data?.data?.stats ?? [],
    isLoading,
    windowHours,
    onWindowHoursChange: setWindowHours,
  };
};

export const ContainerStats = ({ resource }: { resource: ContainerRuntimeView | undefined }) => {
  const liveStats = useLiveStats(resource);
  const memory = useContainerStatsWindow(resource?.id);
  const cpu = useContainerStatsWindow(resource?.id);
  const network = useContainerStatsWindow(resource?.id);

  return <ContainerStatsCharts resource={resource} memory={memory} cpu={cpu} network={network} liveStats={liveStats} />;
};

export const DeploymentStats = ({
  resource,
  deploymentId,
}: {
  resource: ContainerRuntimeView | undefined;
  deploymentId: string;
}) => {
  const liveStats = useLiveStats(resource);
  const memory = useDeploymentStatsWindow(deploymentId);
  const cpu = useDeploymentStatsWindow(deploymentId);
  const network = useDeploymentStatsWindow(deploymentId);

  return <ContainerStatsCharts resource={resource} memory={memory} cpu={cpu} network={network} liveStats={liveStats} />;
};
