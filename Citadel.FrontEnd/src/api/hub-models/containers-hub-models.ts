export interface ContainerStat {
  memoryUsage: number;
  cpuUsage: number;
  memoryLimit: number;
}

export interface ContainerRequest {
  platformId: number;
  containersIds: string[];
}
