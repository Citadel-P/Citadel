import { ContainerStateStatus, ContainerStatView, ContainerRuntimeView } from '@/api/generated/api.types';
import { StatsChartRange } from '@/components/custom/stats-chart-range';
import { useStatsRange, type StatsRangeState } from '@/hooks/useStatsRange';
import { filterStatsRange, statsBucketSeconds } from '@/lib/stats-range';
import { useRead } from '@/lib/hooks';
import { STREAMED_STATS_QUERY_OPTIONS } from '@/lib/live-stats';
import { appendLiveStatBucket, mergeStatsBuckets } from '@/lib/stats-buckets';
import { useState, useMemo, useEffect, useRef } from 'react';
import MemoryUsage from './stats/memory-usage';
import CpuUsage from './stats/cpu-usage';
import NetworkUsage from './stats/network-usage';
import type { ContainerStatsResource } from './stats/types';

type StatsPanelProps = {
  resource: ContainerStatsResource | undefined;
  query: StatsQueryState;
  liveStats: Omit<ContainerStatView, 'containerId'>[];
};

export type StatsQueryState = {
  baseStats: Omit<ContainerStatView, 'containerId'>[];
  isLoading: boolean;
  error?: unknown;
  range: StatsRangeState;
};

export const ContainerStatsCharts = ({ resource, query, liveStats }: StatsPanelProps) => {
  const combined = useCombinedStats(
    query.baseStats,
    query.range.live ? liveStats : [],
    statsBucketSeconds(query.range.bounds),
  );
  const stats = filterStatsRange(combined, query.range.bounds);
  const chartResource = query.range.live ? resource : { ...resource, state: 'Running', containerStat: stats.at(-1) };
  return (
    <div className="flex flex-col gap-4">
      <StatsChartRange range={query.range} error={query.error}>
        {!query.isLoading && stats.length === 0 ? (
          <div className="rounded-md border border-dashed px-6 py-12 text-center text-sm text-muted-foreground">
            No metrics recorded in this range. Try another time range.
          </div>
        ) : (
          <>
            <MemoryUsage container={chartResource} stats={stats} isLoading={query.isLoading} />
            <CpuUsage container={chartResource} stats={stats} isLoading={query.isLoading} />
            <NetworkUsage container={chartResource} stats={stats} isLoading={query.isLoading} />
          </>
        )}
      </StatsChartRange>
    </div>
  );
};

const useLiveStats = (resource: ContainerRuntimeView | undefined, range: StatsRangeState, historyId = resource?.id) => {
  const seconds = statsBucketSeconds(range.bounds);
  const windowSeconds = range.hours * 3600;
  const [liveStats, setLiveStats] = useState<Omit<ContainerStatView, 'containerId'>[]>([]);
  const lastStatRef = useRef<ContainerStatView | undefined>(resource?.containerStat);
  const resourceIdRef = useRef<string | undefined>(historyId);

  useEffect(() => {
    if (resourceIdRef.current === historyId) {
      return;
    }

    resourceIdRef.current = historyId;
    setLiveStats([]);
    lastStatRef.current = resource?.containerStat;
  }, [resource?.containerStat, historyId]);

  useEffect(() => {
    const stat = resource?.containerStat;
    const isRunning = resource?.state === ContainerStateStatus.Running;

    if (isRunning && stat && stat !== lastStatRef.current) {
      lastStatRef.current = stat;
      setLiveStats((prev) =>
        appendLiveStatBucket(
          prev,
          {
            ...stat,
            created: Number(stat.created) > 0 ? stat.created : Math.floor(Date.now() / 1000),
          },
          seconds,
          windowSeconds,
        ),
      );
    }
  }, [resource?.containerStat, resource?.state, seconds, windowSeconds]);

  return liveStats;
};

const useCombinedStats = (
  baseStats: Omit<ContainerStatView, 'containerId'>[],
  liveStats: Omit<ContainerStatView, 'containerId'>[],
  seconds: number,
) => useMemo(() => mergeStatsBuckets(baseStats ?? [], liveStats, seconds), [baseStats, liveStats, seconds]);

const useContainerStatsWindow = (containerId: string | undefined): StatsQueryState => {
  const range = useStatsRange();
  const readArgs = useMemo(() => ({ id: containerId, query: range.query }), [containerId, range.query]);
  const { data, isLoading, error } = useRead('getContainerStats', readArgs, STREAMED_STATS_QUERY_OPTIONS);

  return {
    baseStats: data?.data?.stats ?? [],
    isLoading,
    error,
    range,
  };
};

const useDeploymentStatsWindow = (deploymentId: string, containerId: string | undefined): StatsQueryState => {
  const range = useStatsRange();
  const readArgs = useMemo(() => ({ id: deploymentId, query: range.query }), [deploymentId, range.query]);
  const { data, isLoading, isFetching, error, refetch } = useRead(
    'getDeploymentStats',
    readArgs,
    STREAMED_STATS_QUERY_OPTIONS,
  );
  const previousContainer = useRef(containerId);
  useEffect(() => {
    if (previousContainer.current === containerId || isFetching) return;
    previousContainer.current = containerId;
    void refetch({ cancelRefetch: false });
  }, [containerId, isFetching, refetch]);

  return {
    baseStats: data?.data?.stats ?? [],
    isLoading,
    error,
    range,
  };
};

export const ContainerStats = ({ resource }: { resource: ContainerRuntimeView | undefined }) => {
  const query = useContainerStatsWindow(resource?.id);
  const liveStats = useLiveStats(resource, query.range);

  return <ContainerStatsCharts resource={resource} query={query} liveStats={liveStats} />;
};

export const DeploymentStats = ({
  resource,
  deploymentId,
}: {
  resource: ContainerRuntimeView | undefined;
  deploymentId: string;
}) => {
  const query = useDeploymentStatsWindow(deploymentId, resource?.id);
  const liveStats = useLiveStats(resource, query.range, deploymentId);

  return <ContainerStatsCharts resource={resource} query={query} liveStats={liveStats} />;
};
