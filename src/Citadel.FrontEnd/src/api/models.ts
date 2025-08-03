import { ContainerStateStatus, NullableOfContainerStatView, PlatformStatView } from './_generated';

export interface Cancellable {
  signal: AbortSignal;
}

export interface PlatformStatsBatchView {
  platformId: string;
  networkCount: number;
  volumeCount: number;
  containerCount: number;
  containersRunning: number;
  containersPaused: number;
  containersStopped: number;
  imageCount: number;
  memTotal: number;
  stat: PlatformStatView;
}

export interface DockerContainerView {
  name: string;
  image: string;
  containerId: string;
  state: ContainerStateStatus;
  created: number | null;
  stack: string | null;
  containerStat: NullableOfContainerStatView;
  containerPort: null | [];
}
