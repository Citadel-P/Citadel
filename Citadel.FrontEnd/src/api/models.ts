import { PlatformStatView } from './_generated';

export interface Cancellable {
  signal: AbortSignal;
}

export interface PlatformStatsBatchView {
  platformId: string;
  networksCount: number;
  volumesCount: number;
  containers: number;
  containersRunning: number;
  containersPaused: number;
  containersStopped: number;
  images: number;
  memTotal: number;
  stat: PlatformStatView;
}
