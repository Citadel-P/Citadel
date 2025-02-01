export interface ContainerStat {
  memoryUsage: number;
  cpuUsage: number;
  memoryLimit: number;
}

export interface ContainerRequest {
  platformId: number;
  containersIds: string[];
}

export interface ContainerLogsMessage {
  /** Daemon Id  */
  id: string;
  containerId: string;
  log: string;
}
