import type { ContainerStatView } from '@/api/generated/api.types';

export type ContainerStatsResource = {
  state?: string;
  containerStat?: ContainerStatView | null;
};
