/* eslint-disable */
/* tslint:disable */
// @ts-nocheck
/*
 * ---------------------------------------------------------------
 * ## THIS FILE WAS GENERATED VIA SWAGGER-TYPESCRIPT-API        ##
 * ##                                                           ##
 * ## AUTHOR: acacode                                           ##
 * ## SOURCE: https://github.com/acacode/swagger-typescript-api ##
 * ---------------------------------------------------------------
 */

export enum VolumeSharing {
  None = "None",
  ReadOnly = "ReadOnly",
  OneWriter = "OneWriter",
  All = "All",
}

export enum VolumeScope {
  Single = "Single",
  Multi = "Multi",
}

export enum RegistryType {
  DockerHub = "DockerHub",
  Azure = "Azure",
  AWS = "AWS",
  Gitlab = "Gitlab",
  GitHub = "GitHub",
}

/** @default "Docker" */
export enum PlatformType2 {
  Docker = "Docker",
  DockerSwarm = "DockerSwarm",
  Kubernetes = "Kubernetes",
}

export enum PlatformType {
  Docker = "Docker",
  DockerSwarm = "DockerSwarm",
  Kubernetes = "Kubernetes",
}

export enum PlatformStatus {
  Offline = "Offline",
  Online = "Online",
}

/** @default "local" */
export enum PlatformConnectorType2 {
  Local = "Local",
  Agent = "Agent",
}

export enum PlatformConnectorType {
  Local = "Local",
  Agent = "Agent",
}

export enum NullableOfRegistryType {
  DockerHub = "DockerHub",
  Azure = "Azure",
  AWS = "AWS",
  Gitlab = "Gitlab",
  GitHub = "GitHub",
}

export enum DockerHubTagStatus {
  Active = "Active",
  Inactive = "Inactive",
}

export enum DockerHubImageStatus {
  Active = "Active",
  Inactive = "Inactive",
}

export enum ContainerStateStatus {
  Unknown = "Unknown",
  Created = "Created",
  Running = "Running",
  Paused = "Paused",
  Restarting = "Restarting",
  Exited = "Exited",
  Removing = "Removing",
  Dead = "Dead",
  Offline = "Offline",
}

export type AttestationData = {
  for: string | null;
};

export type BindOptions = {
  propagation: string | null;
  nonRecursive: boolean | null;
  createMountpoint: boolean | null;
  readOnlyNonRecursive: boolean | null;
  readOnlyForceRecursive: boolean | null;
};

export type ClusterVolume = {
  id: string;
  version: VolumeVersionInfo;
  createdAt: string;
  updatedAt: string;
  spec: VolumeSpecification;
  info: ClusterVolumeInfo;
  publishStatus: VolumePublishStatus[];
} | null;

export type ClusterVolumeInfo = {
  /** @format int64 */
  capacityBytes: number | null;
  volumeContext: Record<string, string>;
  volumeID: string;
  accessibleTopology: TopologyEntry[];
};

/** @default null */
export type ConfigFromInput = {
  network: string;
} | null;

export type ContainerConfiguration = {
  hostname: string | null;
  domainname: string | null;
  user: string | null;
  attachStdin: boolean | null;
  attachStdout: boolean | null;
  attachStderr: boolean | null;
  exposedPorts: string[] | null;
  tty: boolean | null;
  openStdin: boolean | null;
  stdinOnce: boolean | null;
  env: string[];
  cmd: string[];
  image: string | null;
  volumes: string[] | null;
  workingDir: string | null;
  entrypoint: string[];
  networkDisabled: boolean | null;
  macAddress: string | null;
  onBuild: string[];
  labels: Record<string, string>;
};

export type ContainerHealthStatus = {
  status: string | null;
  /** @format int32 */
  failingStreak: number | null;
};

export interface ContainerInspectView {
  id: string;
  created: string;
  path: string | null;
  state: ContainerRuntimeState;
  image: string | null;
  resolvConfPath: string | null;
  hostnamePath: string | null;
  hostsPath: string | null;
  logPath: string | null;
  name: string | null;
  /** @format int32 */
  restartCount: number | null;
  driver: string | null;
  platform: string | null;
  mountLabel: string | null;
  processLabel: string | null;
  appArmorProfile: string | null;
  /** @format int64 */
  sizeRw: number | null;
  /** @format int64 */
  sizeRootFs: number | null;
  args: string[] | null;
  execIDs: string[];
  mounts: MountPointInfo[] | null;
  hostConfig: HostConfiguration;
  graphDriver: GraphDriverDataInfo;
  config: ContainerConfiguration;
  networkSettings: NetworkSettingsInfo;
}

export type ContainerRuntimeState = {
  status: ContainerStateStatus;
  running: boolean | null;
  paused: boolean | null;
  restarting: boolean | null;
  oomKilled: boolean | null;
  dead: boolean | null;
  /** @format int32 */
  pid: number | null;
  /** @format int32 */
  exitCode: number | null;
  error: string | null;
  startedAt: string | null;
  finishedAt: string | null;
  health: ContainerHealthStatus;
};

export interface ContainerStatView {
  /** @format uuid */
  containerId?: string;
  /** @format double */
  memoryUsage?: number;
  /** @format double */
  cpuUsage?: number;
  /** @format double */
  memoryLimit?: number;
  /** @format double */
  rxBytes?: number;
  /** @format double */
  txBytes?: number;
  /** @format int64 */
  created?: number;
}

export interface ContainerStatsView {
  stats: ContainerStatView[];
}

export interface ContainerView {
  /** @format uuid */
  id: string;
  containerId: string;
  name: string;
  image: string;
  /** @format date-time */
  created: string;
  state: ContainerStateStatus;
  /** @format date-time */
  updated: string;
  stack: string | null;
  lastStats: NullableOfContainerStatView;
  /** @default null */
  ports?: PortView[] | null;
  platform?: PlatformView;
  metadata?: EndpointMetadata;
}

export interface ContainersView {
  containers: ContainerView[];
}

export interface CreateNetworkInput {
  /** @format uuid */
  platformId: string;
  name: string;
  driver: string;
  scope: string;
  internal: boolean | null;
  attachable: boolean | null;
  ingress: boolean | null;
  enableIPv6: boolean | null;
  enableIPv4: boolean | null;
  configOnly: boolean | null;
  ipam?: IPAMInput;
  configFrom?: ConfigFromInput;
  /** @default null */
  labels?: Record<string, string>;
  /** @default null */
  options?: Record<string, string>;
}

export interface CreateNetworkView {
  id: string;
}

export interface CreateVolumeInput {
  /** @format uuid */
  platformId: string;
  name: string;
  driver: string;
  /** @default null */
  labels?: Record<string, string>;
  /** @default null */
  options?: Record<string, string>;
}

export interface DeleteContainersRequest {
  containersIds: string[];
  /** @default false */
  v?: boolean | null;
  /** @default false */
  force?: boolean | null;
  /** @default false */
  link?: boolean | null;
}

export interface DeleteImageResponseItem {
  result: Record<string, string>;
}

export interface DeleteImageResult {
  items: DeleteImageResponseItem[];
}

export interface DeleteImagesRequest {
  /** @format uuid */
  platformId: string;
  ids: string[];
  /** @default false */
  force?: boolean;
  /** @default false */
  noPrune?: boolean;
}

export interface DeleteNetworksInput {
  /** @format uuid */
  platformId: string;
  ids: string[];
}

export interface DeleteRegistriesInput {
  ids: string[];
}

export interface DeleteVolumesInput {
  /** @format uuid */
  platformId: string;
  names: string[];
  force: boolean | null;
}

export interface DockerHubImageModel {
  repo_name?: string | null;
  short_description?: string | null;
  is_official?: boolean;
  /** @format int64 */
  star_count?: number;
  /** @format int64 */
  pull_count?: number;
  url?: string | null;
  icon?: string | null;
}

export type DockerHubImageView = {
  architecture: string;
  digest: string;
  os: string;
  /** @format int32 */
  size: number;
  status: DockerHubImageStatus;
  lastPulled: string;
} | null;

export interface DockerHubRepositoryInfo {
  name: string | null;
  namespace: string | null;
  /** @format date-time */
  lastUpdated: string;
  isPrivate: boolean;
  isTrusted: boolean;
  isAutomated: boolean;
  /** @format int32 */
  pullCount: number;
}

export interface DockerHubTagView {
  /** @format int32 */
  id: number;
  name: string;
  image: DockerHubImageView;
  lastUpdated: string;
  /** @format int32 */
  fullSize: number;
  status: DockerHubTagStatus;
  lastPulled: string;
}

export interface DockerNetwork {
  name: string;
  id: string;
  created: string;
  driver: string;
  scope: string;
  enableIPv4: boolean;
  enableIPv6: boolean;
  internal: boolean;
  attachable: boolean;
  ingress: boolean;
  configOnly: boolean;
  inUse: boolean;
  configFrom: string | null;
  ipam: IpAddressManagementConfig;
  options: Record<string, string>;
  labels: Record<string, string>;
}

export interface DockerNetworkDetails {
  name: string;
  id: string;
  created: string;
  driver: string;
  scope: string;
  enableIPv4: boolean;
  enableIPv6: boolean;
  internal: boolean;
  attachable: boolean;
  ingress: boolean;
  configOnly: boolean;
  inUse: boolean;
  configFrom: string | null;
  ipam: IpAddressManagementConfig;
  options: Record<string, string>;
  labels: Record<string, string>;
  containers: Record<string, NetworkConnectedContainer>;
  peers: NetworkPeerInfo[];
}

export interface DockerVolume {
  id: string;
  inUse: boolean;
  scope: string;
  driver: string;
  mountpoint: string;
  createdAt: string;
  clusterVolume: ClusterVolume;
  usageData: VolumeUsageData;
  status: Record<string, string>;
  labels: Record<string, string>;
  options: Record<string, string>;
}

export type DriverConfiguration = {
  name: string | null;
  options: Record<string, string>;
};

export type Empty = object;

export type EndpointIpamConfiguration = {
  ipv4Address: string | null;
  ipv6Address: string | null;
  linkLocalIPs: string[];
};

/** @default null */
export type EndpointMetadata = {
  /** @default false */
  canEdit?: boolean | null;
  /** @default false */
  canDelete?: boolean | null;
};

export interface EndpointSettingsInfo {
  ipamConfig: EndpointIpamConfiguration;
  links: string[];
  macAddress: string | null;
  aliases: string[];
  networkID: string | null;
  endpointID: string | null;
  gateway: string | null;
  ipAddress: string | null;
  /** @format int64 */
  ipPrefixLen: number | null;
  ipv6Gateway: string | null;
  globalIPv6Address: string | null;
  /** @format int64 */
  globalIPv6PrefixLen: number | null;
  driverOpts: Record<string, string>;
  dnsNames: string[];
}

export interface GitHubCrPackageVersion {
  /** @format int32 */
  id: number;
  url: string;
  name: string;
  htmlUrl: string | null;
  createdAt: string | null;
  updatedAt: string | null;
  packageHtmlUrl: string | null;
  metadata: GitHubCrPackageVersionMetadata;
}

export type GitHubCrPackageVersionContainerMetadata = {
  tags: string[] | null;
};

export type GitHubCrPackageVersionMetadata = {
  container: GitHubCrPackageVersionContainerMetadata;
} | null;

export type GraphDriverDataInfo = {
  name: string | null;
  data: Record<string, string>;
};

export type HostConfiguration = {
  binds: string[];
  containerIDFile: string | null;
  logConfig: LogConfiguration;
  networkMode: string | null;
  portBindings: Record<string, HostPortBinding[]>[];
  restartPolicy: RestartPolicy;
  autoRemove: boolean | null;
  volumeDriver: string | null;
  volumesFrom: string[];
  mounts: HostMount[];
  consoleSize: number[];
  annotations: Record<string, string>;
  capAdd: string[];
  capDrop: string[];
  cgroupnsMode: string | null;
  dns: string[];
  dnsOptions: string[];
  dnsSearch: string[];
  extraHosts: string[];
  groupAdd: string[];
  ipcMode: string | null;
  cgroup: string | null;
  links: string[];
  /** @format int32 */
  oomScoreAdj: number | null;
  pidMode: string | null;
  privileged: boolean | null;
  publishAllPorts: boolean | null;
  readonlyRootfs: boolean | null;
  securityOpt: string[];
  storageOpt: Record<string, string>;
  tmpfs: Record<string, string>;
  utsMode: string | null;
  usernsMode: string | null;
  /** @format int64 */
  shmSize: number;
  sysctls: Record<string, string>;
  runtime: string | null;
  isolation: string | null;
  maskedPaths: string[];
  readonlyPaths: string[];
  /** @format int64 */
  memorySwap: number | null;
  /** @format int64 */
  memorySwappiness: number | null;
  /** @format int64 */
  nanoCpus: number | null;
  /** @format int64 */
  pidsLimit: number | null;
  /** @format int64 */
  memory: number | null;
  /** @format int64 */
  memoryReservation: number | null;
  /** @format int64 */
  ioMaximumBandwidth: number | null;
  /** @format int64 */
  cpuPeriod: number | null;
  /** @format int64 */
  cpuPercent: number | null;
  /** @format int64 */
  cpuCount: number | null;
  ulimits: Ulimit[];
  /** @format int64 */
  kernelMemoryTCP: number | null;
};

export interface HostMount {
  target: string | null;
  source: string | null;
  type: string | null;
  readOnly: boolean | null;
  consistency: string | null;
  bindOptions: BindOptions;
  volumeOptions: VolumeOptions;
}

export interface HostPortBinding {
  hostIP: string | null;
  hostPort: string | null;
}

export interface HttpValidationProblemDetails {
  type?: string | null;
  title?: string | null;
  /** @format int32 */
  status?: number | null;
  detail?: string | null;
  instance?: string | null;
  errors?: Record<string, string[]>;
}

export type IImageRepository = BaseIImageRepository &
  (
    | BaseIImageRepositoryTypeMapping<
        "GitHub",
        IImageRepositoryGitHubPackageResponse
      >
    | BaseIImageRepositoryTypeMapping<
        "DockerHub",
        IImageRepositoryDockerHubRepositoryResponse
      >
  );

export interface IImageRepositoryDockerHubRepositoryResponse {
  $type?: "DockerHub";
  name?: string | null;
  namespace?: string | null;
  /** @format date-time */
  lastUpdated?: string;
  isPrivate?: boolean;
  /** @format int32 */
  pullCount?: number;
}

export interface IImageRepositoryGitHubPackageResponse {
  $type?: "GitHub";
  id?: string;
  name?: string;
  createdAt?: string | null;
  updatedAt?: string | null;
  url?: string | null;
  htmlUrl?: string | null;
}

export interface IPAMConfigInput {
  subnet: string;
  ipRange: string;
  gateway: string;
}

/** @default null */
export type IPAMInput = {
  driver: string;
  /** @default null */
  config?: IPAMConfigInput[] | null;
  /** @default null */
  options?: Record<string, string>;
};

export type ImageConfig = {
  tty: boolean;
  user: string;
  image: string;
  hostname: string;
  domainname: string;
  attachStdin: boolean;
  attachStdout: boolean;
  attachStderr: boolean;
  stopSignal: string;
  openStdin: boolean;
  stdinOnce: boolean;
  argsEscaped: boolean;
  workingDir: string;
  healthCheck: ImageHealthCheck;
  shell: string[];
  env: string[];
  cmd: string[];
  onBuild: string[];
  entryPoint: string[];
  volumes: Record<string, Empty>;
  labels: Record<string, string>;
  exposedPorts: Record<string, any>;
} | null;

export type ImageData = {
  platform: ImagePlatformDescriptor;
  containers: string[];
  size: SizeInfo;
} | null;

export type ImageDescriptor = {
  /** @format int64 */
  size: number;
  data: string | null;
  digest: string;
  mediaType: string;
  artifactType: string;
  platform: ImagePlatformDescriptor;
  urls: string[];
  annotations: Record<string, string>;
};

export type ImageGraphDriverData = {
  mergedDir: string;
  upperDir: string;
  workDir: string;
} | null;

export type ImageGraphicDriver = {
  name: string;
  data: ImageGraphDriverData;
} | null;

export type ImageHealthCheck = {
  test: string[];
  /** @format int64 */
  interval: number | null;
  /** @format int64 */
  timeout: number | null;
  /** @format int64 */
  retries: number | null;
  /** @format int64 */
  startPeriod: number | null;
  /** @format int64 */
  startInterval: number | null;
};

export interface ImageManifest {
  id: string;
  kind: string;
  available: boolean;
  size: SizeInfo;
  imageData: ImageData;
  descriptor: ImageDescriptor;
  attestationData: AttestationData;
}

export type ImageMetadata = {
  lastTagTime: string;
} | null;

export type ImagePlatformDescriptor = {
  os: string;
  variant: string;
  osVersion: string;
  architecture: string;
  osFeatures: string[];
} | null;

export type ImagePullError = {
  /** @format int64 */
  code: number | null;
  message: string | null;
};

export type ImagePullProgress = {
  units: string | null;
  /** @format int64 */
  current: number | null;
  /** @format int64 */
  total: number | null;
  /** @format int64 */
  start: number | null;
};

export type ImageRootFs = {
  type: string;
  layers: string[];
} | null;

export interface ImageView {
  id: string;
  /** @format int64 */
  created: number;
  parentId: string;
  repoDigests: string[];
  repoTags: string[];
  /** @format int64 */
  sharedSize: number;
  /** @format double */
  size: number;
  /** @format double */
  virtualSize: number;
  isInUse: boolean;
  labels: Record<string, string>;
  name?: string | null;
  tag?: string | null;
}

export interface ImagesView {
  images: ImageView[];
}

export interface InspectImageResult {
  id: string;
  author: string;
  parent: string;
  comment: string;
  created: string;
  dockerVersion: string;
  architecture: string;
  osVersion: string;
  /** @format int64 */
  virtualSize: number;
  variant: string;
  os: string;
  /** @format int64 */
  size: number;
  rootFS: ImageRootFs;
  metadata: ImageMetadata;
  config: ImageConfig;
  descriptor: ImageDescriptor;
  graphDriver: ImageGraphicDriver;
  manifests: ImageManifest[];
  repoTags: string[];
  repoDigests: string[];
}

export interface IpAddressInfo {
  addr: string | null;
  /** @format int64 */
  prefixLen: number | null;
}

export type IpAddressManagementConfig = {
  driver: string | null;
  config: IpamSubnetConfiguration[];
  options: Record<string, string>;
};

export interface IpamSubnetConfiguration {
  subnet: string | null;
  ipRange: string | null;
  gateway: string | null;
}

export type LogConfiguration = {
  type: string | null;
  config: Record<string, string>;
};

export interface LoginRequest {
  email: string;
  password: string;
}

export interface LoginResponse {
  accessToken: string;
}

export interface MountPointInfo {
  type: string | null;
  name: string | null;
  source: string | null;
  destination: string | null;
  driver: string | null;
  mode: string | null;
  rw: boolean | null;
  propagation: string | null;
}

export interface NetworkConnectedContainer {
  name: string;
  endpointId: string;
  macAddress: string;
  ipV4Address: string;
  ipv6Address: string;
}

export interface NetworkPeerInfo {
  name: string;
  ip: string;
}

export type NetworkSettingsInfo = {
  bridge: string | null;
  sandboxID: string | null;
  hairpinMode: boolean | null;
  linkLocalIPv6Address: string | null;
  /** @format int64 */
  linkLocalIPv6PrefixLen: number | null;
  sandboxKey: string | null;
  secondaryIPAddresses: IpAddressInfo[];
  secondaryIPv6Addresses: any[];
  endpointID: string | null;
  gateway: string | null;
  globalIPv6Address: string | null;
  /** @format int64 */
  globalIPv6PrefixLen: number | null;
  ipAddress: string | null;
  /** @format int64 */
  ipPrefixLen: number | null;
  ipv6Gateway: string | null;
  macAddress: string | null;
  ports: Record<string, HostPortBinding[]>[];
  networks: Record<string, EndpointSettingsInfo>;
};

export interface NetworksView {
  networks: DockerNetwork[];
}

export type NullableOfContainerStatView = {
  /** @format uuid */
  containerId?: string;
  /** @format double */
  memoryUsage?: number;
  /** @format double */
  cpuUsage?: number;
  /** @format double */
  memoryLimit?: number;
  /** @format double */
  rxBytes?: number;
  /** @format double */
  txBytes?: number;
  /** @format int64 */
  created?: number;
} | null;

export type PlatformDescriptor = BasePlatformDescriptor &
  (
    | BasePlatformDescriptorTypeMapping<
        "Docker",
        PlatformDescriptorDockerPlatformDescriptor
      >
    | BasePlatformDescriptorTypeMapping<
        "DockerSwarm",
        PlatformDescriptorDockerSwarmPlatformDescriptor
      >
    | BasePlatformDescriptorTypeMapping<
        "Kubernetes",
        PlatformDescriptorKubernetesPlatformDescriptor
      >
  );

export interface PlatformDescriptorDockerPlatformDescriptor {
  $type?: "Docker";
  daemonId: string;
  /** @format int64 */
  containerCount: number;
  /** @format int64 */
  containersRunning: number;
  /** @format int64 */
  containersPaused: number;
  /** @format int64 */
  containersStopped: number;
  /** @default null */
  driver?: string | null;
  /** @default null */
  operatingSystem?: string | null;
  /** @default null */
  osVersion?: string | null;
  /** @default null */
  osType?: string | null;
  /** @default null */
  architecture?: string | null;
}

export interface PlatformDescriptorDockerSwarmPlatformDescriptor {
  $type?: "DockerSwarm";
  nodeID: string;
  nodeAddr: string;
  localNodeState: string;
  controlAvailable: boolean;
  /** @format int64 */
  nodes: number;
  /** @format int64 */
  managers: number;
  /** @default null */
  error?: string | null;
  /** @default null */
  remoteManagers?: {
    nodeID: string | null;
    addr: string | null;
  }[];
  daemonId: string;
  /** @format int64 */
  containerCount: number;
  /** @format int64 */
  containersRunning: number;
  /** @format int64 */
  containersPaused: number;
  /** @format int64 */
  containersStopped: number;
  /** @default null */
  driver?: string | null;
  /** @default null */
  operatingSystem?: string | null;
  /** @default null */
  osVersion?: string | null;
  /** @default null */
  osType?: string | null;
  /** @default null */
  architecture?: string | null;
}

export interface PlatformDescriptorKubernetesPlatformDescriptor {
  $type?: "Kubernetes";
  clusterName: string | null;
  clusterVersion: string | null;
  apiServerUrl: string | null;
  namespace: string | null;
}

export interface PlatformInput {
  name: string;
  address: string;
  type?: PlatformType2;
  connectorType?: PlatformConnectorType2;
}

export interface PlatformStatView {
  /** @format int64 */
  created?: number;
  /** @format double */
  txBytes?: number;
  /** @format double */
  rxBytes?: number;
  /** @format double */
  cpuUsage?: number;
  /** @format double */
  memoryUsage?: number;
}

/** @default null */
export type PlatformView = {
  /** @format uuid */
  id: string;
  name: string;
  address: string;
  /** @format int32 */
  networkCount: number;
  /** @format int32 */
  volumeCount: number;
  /** @format int64 */
  imageCount: number;
  /** @format int64 */
  cpuCount: number;
  /** @format int64 */
  memTotal: number;
  agentVersion: string | null;
  serverVersion: string | null;
  type: PlatformType;
  status: PlatformStatus;
  connectorType: PlatformConnectorType;
  stats: PlatformStatView[] | null;
  platformDescriptor: PlatformDescriptor;
};

export interface PlatformView2 {
  /** @format uuid */
  id: string;
  name: string;
  address: string;
  /** @format int32 */
  networkCount: number;
  /** @format int32 */
  volumeCount: number;
  /** @format int64 */
  imageCount: number;
  /** @format int64 */
  cpuCount: number;
  /** @format int64 */
  memTotal: number;
  agentVersion: string | null;
  serverVersion: string | null;
  type: PlatformType;
  status: PlatformStatus;
  connectorType: PlatformConnectorType;
  stats: PlatformStatView[] | null;
  platformDescriptor: PlatformDescriptor;
}

export interface PlatformsView {
  platforms: PlatformView2[];
}

export interface PortView {
  ip?: string;
  /** @format int32 */
  privatePort?: number;
  /** @format int32 */
  publicPort?: number;
}

export interface ProblemDetails {
  type?: string | null;
  title?: string | null;
  /** @format int32 */
  status?: number | null;
  detail?: string | null;
  instance?: string | null;
}

export interface PullImageRequest {
  /** @format uuid */
  platformId: string;
  registryName: string;
  repositoryName: string;
  imageTag: string;
}

export interface PullImageResult {
  id: string | null;
  from: string | null;
  stream: string | null;
  status: string | null;
  errorMessage: string | null;
  progressMessage: string | null;
  progress: ImagePullProgress;
  error: ImagePullError;
}

export interface RefreshTokenResponse {
  accessToken: string;
}

export interface RegistriesView {
  registries: RegistryView[];
}

export type RegistryConfigurationBase = BaseRegistryConfigurationBase &
  (
    | BaseRegistryConfigurationBaseTypeMapping<
        "AWS",
        RegistryConfigurationBaseAWSRegistry
      >
    | BaseRegistryConfigurationBaseTypeMapping<
        "Azure",
        RegistryConfigurationBaseAzureRegistry
      >
    | BaseRegistryConfigurationBaseTypeMapping<
        "Gitlab",
        RegistryConfigurationBaseGitlabRegistry
      >
    | BaseRegistryConfigurationBaseTypeMapping<
        "DockerHub",
        RegistryConfigurationBaseDockerHubRegistry
      >
    | BaseRegistryConfigurationBaseTypeMapping<
        "GitHub",
        RegistryConfigurationBaseGitHubRegistry
      >
  );

export interface RegistryConfigurationBaseAWSRegistry {
  $type?: "AWS";
  accessKey: string;
  authenticationRequired: boolean;
  secretAccessKey: string;
  region: string;
  registryUrl?: string | null;
}

export interface RegistryConfigurationBaseAzureRegistry {
  $type?: "Azure";
  userName: string;
  password: string;
  registryUrl?: string | null;
}

export interface RegistryConfigurationBaseDockerHubRegistry {
  $type?: "DockerHub";
  /** @default null */
  userName?: string | null;
  /** @default null */
  pat?: string | null;
  registryUrl?: string | null;
}

export interface RegistryConfigurationBaseGitHubRegistry {
  $type?: "GitHub";
  name: string;
  pat: string;
  type: "Organization" | "User" | null;
  registryUrl?: string | null;
}

export interface RegistryConfigurationBaseGitlabRegistry {
  $type?: "Gitlab";
  userName: string;
  pat: string;
  instanceUrl: string;
  registryUrl?: string | null;
}

export interface RegistryInput {
  name: string | null;
  url: string | null;
  type: NullableOfRegistryType;
  configuration: RegistryConfigurationBase;
}

export interface RegistryView {
  /** @format uuid */
  id: string;
  name: string;
  url: string;
  type: RegistryType;
  /** @format date-time */
  created: string;
  configuration: RegistryConfigurationBase;
  isDefault?: boolean;
}

export type RestartPolicy = {
  name: string | null;
  /** @format int32 */
  maximumRetryCount: number | null;
};

export type SizeInfo = {
  /** @format int64 */
  total: number | null;
  /** @format int64 */
  content: number | null;
  /** @format int64 */
  unpacked: number | null;
};

export interface StreamLogsRequest {
  containerId: string;
}

export interface TopologyEntry {
  labels: Record<string, string>;
}

export interface Ulimit {
  name: string | null;
  /** @format int64 */
  soft: number | null;
  /** @format int64 */
  hard: number | null;
}

export type VolumeAccessMode = {
  scope: VolumeScope;
  sharing: VolumeSharing;
  secrets: VolumeSecret[];
  capacityRange: VolumeCapacityRange;
  availability: string;
} | null;

export type VolumeCapacityRange = {
  /** @format int64 */
  requiredBytes: number | null;
  /** @format int64 */
  limitBytes: number | null;
};

export type VolumeOptions = {
  noCopy: boolean | null;
  labels: Record<string, string>;
  driverConfig: DriverConfiguration;
  subpath: string | null;
};

export interface VolumePublishStatus {
  nodeID: string;
  state: string;
  publishContext: Record<string, string>;
}

export interface VolumeSecret {
  key: string;
  secret: string;
}

export type VolumeSpecification = {
  group: string;
  accessMode: VolumeAccessMode;
} | null;

export type VolumeUsageData = {
  /** @format int64 */
  size: number | null;
  /** @format int64 */
  refCount: number | null;
};

export type VolumeVersionInfo = {
  /** @format int64 */
  index: number | null;
};

export interface VolumesView {
  volumes: DockerVolume[];
}

type BaseIImageRepository = object;

type BaseIImageRepositoryTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BasePlatformDescriptor = object | null;

type BasePlatformDescriptorTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BaseRegistryConfigurationBase = object | null;

type BaseRegistryConfigurationBaseTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

export type QueryParamsType = Record<string | number, any>;
export type ResponseFormat = keyof Omit<Body, "body" | "bodyUsed">;

export interface FullRequestParams extends Omit<RequestInit, "body"> {
  /** set parameter to `true` for call `securityWorker` for this request */
  secure?: boolean;
  /** request path */
  path: string;
  /** content type of request body */
  type?: ContentType;
  /** query params */
  query?: QueryParamsType;
  /** format of response (i.e. response.json() -> format: "json") */
  format?: ResponseFormat;
  /** request body */
  body?: unknown;
  /** base url */
  baseUrl?: string;
  /** request cancellation token */
  cancelToken?: CancelToken;
}

export type RequestParams = Omit<
  FullRequestParams,
  "body" | "method" | "query" | "path"
>;

export interface ApiConfig<SecurityDataType = unknown> {
  baseUrl?: string;
  baseApiParams?: Omit<RequestParams, "baseUrl" | "cancelToken" | "signal">;
  securityWorker?: (
    securityData: SecurityDataType | null,
  ) => Promise<RequestParams | void> | RequestParams | void;
  customFetch?: typeof fetch;
}

export interface HttpResponse<D extends unknown, E extends unknown = unknown>
  extends Response {
  data: D;
  error: E;
}

type CancelToken = Symbol | string | number;

export enum ContentType {
  Json = "application/json",
  FormData = "multipart/form-data",
  UrlEncoded = "application/x-www-form-urlencoded",
  Text = "text/plain",
}

export class HttpClient<SecurityDataType = unknown> {
  public baseUrl: string = "";
  private securityData: SecurityDataType | null = null;
  private securityWorker?: ApiConfig<SecurityDataType>["securityWorker"];
  private abortControllers = new Map<CancelToken, AbortController>();
  private customFetch = (...fetchParams: Parameters<typeof fetch>) =>
    fetch(...fetchParams);

  private baseApiParams: RequestParams = {
    credentials: "same-origin",
    headers: {},
    redirect: "follow",
    referrerPolicy: "no-referrer",
  };

  constructor(apiConfig: ApiConfig<SecurityDataType> = {}) {
    Object.assign(this, apiConfig);
  }

  public setSecurityData = (data: SecurityDataType | null) => {
    this.securityData = data;
  };

  protected encodeQueryParam(key: string, value: any) {
    const encodedKey = encodeURIComponent(key);
    return `${encodedKey}=${encodeURIComponent(typeof value === "number" ? value : `${value}`)}`;
  }

  protected addQueryParam(query: QueryParamsType, key: string) {
    return this.encodeQueryParam(key, query[key]);
  }

  protected addArrayQueryParam(query: QueryParamsType, key: string) {
    const value = query[key];
    return value.map((v: any) => this.encodeQueryParam(key, v)).join("&");
  }

  protected toQueryString(rawQuery?: QueryParamsType): string {
    const query = rawQuery || {};
    const keys = Object.keys(query).filter(
      (key) => "undefined" !== typeof query[key],
    );
    return keys
      .map((key) =>
        Array.isArray(query[key])
          ? this.addArrayQueryParam(query, key)
          : this.addQueryParam(query, key),
      )
      .join("&");
  }

  protected addQueryParams(rawQuery?: QueryParamsType): string {
    const queryString = this.toQueryString(rawQuery);
    return queryString ? `?${queryString}` : "";
  }

  private contentFormatters: Record<ContentType, (input: any) => any> = {
    [ContentType.Json]: (input: any) =>
      input !== null && (typeof input === "object" || typeof input === "string")
        ? JSON.stringify(input)
        : input,
    [ContentType.Text]: (input: any) =>
      input !== null && typeof input !== "string"
        ? JSON.stringify(input)
        : input,
    [ContentType.FormData]: (input: any) =>
      Object.keys(input || {}).reduce((formData, key) => {
        const property = input[key];
        formData.append(
          key,
          property instanceof Blob
            ? property
            : typeof property === "object" && property !== null
              ? JSON.stringify(property)
              : `${property}`,
        );
        return formData;
      }, new FormData()),
    [ContentType.UrlEncoded]: (input: any) => this.toQueryString(input),
  };

  protected mergeRequestParams(
    params1: RequestParams,
    params2?: RequestParams,
  ): RequestParams {
    return {
      ...this.baseApiParams,
      ...params1,
      ...(params2 || {}),
      headers: {
        ...(this.baseApiParams.headers || {}),
        ...(params1.headers || {}),
        ...((params2 && params2.headers) || {}),
      },
    };
  }

  protected createAbortSignal = (
    cancelToken: CancelToken,
  ): AbortSignal | undefined => {
    if (this.abortControllers.has(cancelToken)) {
      const abortController = this.abortControllers.get(cancelToken);
      if (abortController) {
        return abortController.signal;
      }
      return void 0;
    }

    const abortController = new AbortController();
    this.abortControllers.set(cancelToken, abortController);
    return abortController.signal;
  };

  public abortRequest = (cancelToken: CancelToken) => {
    const abortController = this.abortControllers.get(cancelToken);

    if (abortController) {
      abortController.abort();
      this.abortControllers.delete(cancelToken);
    }
  };

  public request = async <T = any, E = any>({
    body,
    secure,
    path,
    type,
    query,
    format,
    baseUrl,
    cancelToken,
    ...params
  }: FullRequestParams): Promise<HttpResponse<T, E>> => {
    const secureParams =
      ((typeof secure === "boolean" ? secure : this.baseApiParams.secure) &&
        this.securityWorker &&
        (await this.securityWorker(this.securityData))) ||
      {};
    const requestParams = this.mergeRequestParams(params, secureParams);
    const queryString = query && this.toQueryString(query);
    const payloadFormatter = this.contentFormatters[type || ContentType.Json];
    const responseFormat = format || requestParams.format;

    return this.customFetch(
      `${baseUrl || this.baseUrl || ""}${path}${queryString ? `?${queryString}` : ""}`,
      {
        ...requestParams,
        headers: {
          ...(requestParams.headers || {}),
          ...(type && type !== ContentType.FormData
            ? { "Content-Type": type }
            : {}),
        },
        signal:
          (cancelToken
            ? this.createAbortSignal(cancelToken)
            : requestParams.signal) || null,
        body:
          typeof body === "undefined" || body === null
            ? null
            : payloadFormatter(body),
      },
    ).then(async (response) => {
      const r = response.clone() as HttpResponse<T, E>;
      r.data = null as unknown as T;
      r.error = null as unknown as E;

      const data = !responseFormat
        ? r
        : await response[responseFormat]()
            .then((data) => {
              if (r.ok) {
                r.data = data;
              } else {
                r.error = data;
              }
              return r;
            })
            .catch((e) => {
              r.error = e;
              return r;
            });

      if (cancelToken) {
        this.abortControllers.delete(cancelToken);
      }

      if (!response.ok) throw data;
      return data;
    });
  };
}

/**
 * @title Citadel.WebApi | v1
 * @version 1.0.0
 */
export class Api<
  SecurityDataType extends unknown,
> extends HttpClient<SecurityDataType> {
  api = {
    /**
     * No description
     *
     * @tags Authentication
     * @name AuthenticationRefreshToken
     * @summary Request a new access token
     * @request GET:/api/v1/authentication/refresh
     * @response `200` `RefreshTokenResponse` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    authenticationRefreshToken: (params: RequestParams = {}) =>
      this.request<
        RefreshTokenResponse,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/authentication/refresh`,
        method: "GET",
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Authentication
     * @name AuthenticationLogin
     * @summary Check user credentials and issue an access token on successful login
     * @request POST:/api/v1/authentication/login
     * @response `200` `LoginResponse` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    authenticationLogin: (data: LoginRequest, params: RequestParams = {}) =>
      this.request<
        LoginResponse,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/authentication/login`,
        method: "POST",
        body: data,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Authentication
     * @name AuthenticationLogout
     * @summary Log out
     * @request POST:/api/v1/authentication/logout
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    authenticationLogout: (params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/authentication/logout`,
        method: "POST",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name ContainersGetById
     * @summary Get container by Id
     * @request GET:/api/v1/containers/{id}
     * @secure
     * @response `200` `ContainerView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    containersGetById: (id: string, params: RequestParams = {}) =>
      this.request<
        ContainerView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/containers/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name ContainersGetStats
     * @summary Get container stats
     * @request GET:/api/v1/containers/{id}/stats
     * @secure
     * @response `200` `ContainerStatsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    containersGetStats: (id: string, params: RequestParams = {}) =>
      this.request<
        ContainerStatsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/containers/${id}/stats`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name ContainersInspect
     * @summary Inspect a container
     * @request GET:/api/v1/containers/{id}/inspect
     * @secure
     * @response `200` `ContainerInspectView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    containersInspect: (id: string, params: RequestParams = {}) =>
      this.request<
        ContainerInspectView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/containers/${id}/inspect`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name ContainersStreamLogs
     * @summary Stream container logs
     * @request POST:/api/v1/containers/stream-logs
     * @secure
     * @response `200` `void` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    containersStreamLogs: (
      data: StreamLogsRequest,
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/stream-logs`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name ContainersStartContainers
     * @summary Starts the given container(s)
     * @request PATCH:/api/v1/containers/start
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    containersStartContainers: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/start`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name ContainersStopContainers
     * @summary Stops the given container(s)
     * @request PATCH:/api/v1/containers/stop
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    containersStopContainers: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/stop`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name ContainersPauseContainers
     * @summary Pause the given container(s)
     * @request PATCH:/api/v1/containers/pause
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    containersPauseContainers: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/pause`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name ContainersRestartContainers
     * @summary Restarts the given container(s)
     * @request PATCH:/api/v1/containers/restart
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    containersRestartContainers: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/restart`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name ContainersUnpauseContainers
     * @summary Unpause the given container(s)
     * @request PATCH:/api/v1/containers/unpause
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    containersUnpauseContainers: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/unpause`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name ContainersDeleteContainers
     * @summary Delete the given container(s)
     * @request DELETE:/api/v1/containers/delete
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    containersDeleteContainers: (
      data: DeleteContainersRequest,
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/delete`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name PlatformsList
     * @summary List all platforms
     * @request GET:/api/v1/platforms
     * @secure
     * @response `200` `PlatformsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    platformsList: (params: RequestParams = {}) =>
      this.request<
        PlatformsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/platforms`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name PlatformsCreate
     * @summary Create a platform
     * @request POST:/api/v1/platforms
     * @secure
     * @response `200` `PlatformView2` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    platformsCreate: (data: PlatformInput, params: RequestParams = {}) =>
      this.request<
        PlatformView2,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/platforms`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name PlatformsDelete
     * @summary Delete a platform
     * @request DELETE:/api/v1/platforms
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    platformsDelete: (
      query: {
        /**
         * The platform id
         * @format uuid
         */
        id: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/platforms`,
        method: "DELETE",
        query: query,
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name PlatformsGetById
     * @summary Get platform by Id
     * @request GET:/api/v1/platforms/{id}
     * @secure
     * @response `200` `PlatformView2` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    platformsGetById: (id: string, params: RequestParams = {}) =>
      this.request<
        PlatformView2,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/platforms/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name PlatformsPatch
     * @summary Patch a platform
     * @request PATCH:/api/v1/platforms/{id}
     * @secure
     * @response `200` `PlatformView2` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `500` `ProblemDetails` Internal Server Error
     */
    platformsPatch: (
      id: string,
      data: PlatformInput,
      params: RequestParams = {},
    ) =>
      this.request<
        PlatformView2,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/platforms/${id}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name PlatformsGetInfo
     * @summary Get platform by Id
     * @request GET:/api/v1/platforms/{id}/info
     * @secure
     * @response `200` `PlatformView2` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    platformsGetInfo: (id: string, params: RequestParams = {}) =>
      this.request<
        PlatformView2,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/platforms/${id}/info`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name PlatformsListContainers
     * @summary Returns the list of containers of the given platform
     * @request GET:/api/v1/platforms/{id}/containers
     * @secure
     * @response `200` `ContainersView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    platformsListContainers: (id: string, params: RequestParams = {}) =>
      this.request<
        ContainersView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/platforms/${id}/containers`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Registries
     * @name RegistriesGetAll
     * @summary Get all registries
     * @request GET:/api/v1/registries/all
     * @secure
     * @response `200` `RegistriesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    registriesGetAll: (params: RequestParams = {}) =>
      this.request<
        RegistriesView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/registries/all`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Registries
     * @name RegistriesGetById
     * @summary Get all registries
     * @request GET:/api/v1/registries/{id}
     * @secure
     * @response `200` `RegistryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    registriesGetById: (id: string, params: RequestParams = {}) =>
      this.request<RegistryView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/registries/${id}`,
          method: "GET",
          secure: true,
          format: "json",
          ...params,
        },
      ),

    /**
     * @description A discriminator should be provided in the request, this discriminator is based on RegistryType enum
     *
     * @tags Registries
     * @name RegistriesPatch
     * @summary Patch a registry
     * @request PATCH:/api/v1/registries/{id}
     * @secure
     * @response `200` `RegistryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `500` `ProblemDetails` Internal Server Error
     */
    registriesPatch: (
      id: string,
      data: RegistryInput,
      params: RequestParams = {},
    ) =>
      this.request<RegistryView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/registries/${id}`,
          method: "PATCH",
          body: data,
          secure: true,
          type: ContentType.Json,
          format: "json",
          ...params,
        },
      ),

    /**
     * @description A discriminator should be provided in the request, this discriminator is based on RegistryType enum
     *
     * @tags Registries
     * @name RegistriesCreate
     * @summary Create a registry
     * @request POST:/api/v1/registries
     * @secure
     * @response `200` `RegistryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    registriesCreate: (data: RegistryInput, params: RequestParams = {}) =>
      this.request<RegistryView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/registries`,
          method: "POST",
          body: data,
          secure: true,
          type: ContentType.Json,
          format: "json",
          ...params,
        },
      ),

    /**
     * No description
     *
     * @tags Registries
     * @name RegistriesDelete
     * @summary Delete registries
     * @request DELETE:/api/v1/registries
     * @secure
     * @response `200` `RegistriesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    registriesDelete: (
      data: DeleteRegistriesInput,
      params: RequestParams = {},
    ) =>
      this.request<
        RegistriesView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/registries`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name ImagesGetAllLocalImages
     * @summary Get all local images for the given platform
     * @request GET:/api/v1/images/{platformId}/local-images
     * @secure
     * @response `200` `ImagesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    imagesGetAllLocalImages: (platformId: string, params: RequestParams = {}) =>
      this.request<ImagesView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/images/${platformId}/local-images`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name ImagesGetExternalRepositories
     * @summary List external repositories of the given registry
     * @request GET:/api/v1/images/{registryName}/repositories
     * @secure
     * @response `200` `(IImageRepository)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    imagesGetExternalRepositories: (
      registryName: string,
      params: RequestParams = {},
    ) =>
      this.request<
        IImageRepository[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images/${registryName}/repositories`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name ImagesGetGhcrPackageVersions
     * @summary List versions of GHCR package
     * @request GET:/api/v1/images/ghcr/{registryName}/{packageName}/versions
     * @secure
     * @response `200` `(GitHubCrPackageVersion)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    imagesGetGhcrPackageVersions: (
      registryName: string,
      packageName: string,
      params: RequestParams = {},
    ) =>
      this.request<
        GitHubCrPackageVersion[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images/ghcr/${registryName}/${packageName}/versions`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name ImagesGetDockerHubRepositories
     * @summary List DockerHub repositories
     * @request GET:/api/v1/images/dockerhub/{registryName}/repositories
     * @secure
     * @response `200` `(DockerHubRepositoryInfo)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    imagesGetDockerHubRepositories: (
      registryName: string,
      params: RequestParams = {},
    ) =>
      this.request<
        DockerHubRepositoryInfo[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images/dockerhub/${registryName}/repositories`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name ImagesGetDockerHubRepositoryTags
     * @summary List DockerHub repository tags
     * @request GET:/api/v1/images/dockerhub/{registryName}/{repositoryName}/tags
     * @secure
     * @response `200` `(DockerHubTagView)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    imagesGetDockerHubRepositoryTags: (
      registryName: string,
      repositoryName: string,
      params: RequestParams = {},
    ) =>
      this.request<
        DockerHubTagView[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images/dockerhub/${registryName}/${repositoryName}/tags`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name ImagesGetDockerHubPublicImages
     * @summary Search for DockerHub public images, if imageName is empty a default list of docker images will be returned
     * @request GET:/api/v1/images/dockerhub
     * @secure
     * @response `200` `(DockerHubImageModel)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    imagesGetDockerHubPublicImages: (
      query?: {
        /** @default null */
        imageName?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<
        DockerHubImageModel[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images/dockerhub`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name ImagesInspect
     * @summary Inspect an image
     * @request GET:/api/v1/images/{platformId}/{imageId}
     * @secure
     * @response `200` `InspectImageResult` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    imagesInspect: (
      platformId: string,
      imageId: string,
      params: RequestParams = {},
    ) =>
      this.request<
        InspectImageResult,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images/${platformId}/${imageId}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name ImagesPullImage
     * @summary Pull an image from a registry and returns logs as a stream
     * @request POST:/api/v1/images/pull
     * @secure
     * @response `200` `(PullImageResult)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    imagesPullImage: (data: PullImageRequest, params: RequestParams = {}) =>
      this.request<
        PullImageResult[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images/pull`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name ImagesDelete
     * @summary Remove an image(s), along with any untagged parent images that were referenced by that image
     * @request DELETE:/api/v1/images
     * @secure
     * @response `200` `DeleteImageResult` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    imagesDelete: (data: DeleteImagesRequest, params: RequestParams = {}) =>
      this.request<
        DeleteImageResult,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Networks
     * @name NetworksList
     * @summary List all networks
     * @request GET:/api/v1/networks/{id}
     * @secure
     * @response `200` `NetworksView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `500` `ProblemDetails` Internal Server Error
     */
    networksList: (
      id: string,
      query?: {
        /** @default null */
        Dangling?: boolean;
        /** @default null */
        Driver?: string;
        /** @default null */
        Id?: string;
        /** @default null */
        Name?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<NetworksView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/networks/${id}`,
          method: "GET",
          query: query,
          secure: true,
          format: "json",
          ...params,
        },
      ),

    /**
     * No description
     *
     * @tags Networks
     * @name NetworksInspect
     * @summary Inspect a network
     * @request GET:/api/v1/networks/{platformId}/{networkId}
     * @secure
     * @response `200` `DockerNetworkDetails` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `500` `ProblemDetails` Internal Server Error
     */
    networksInspect: (
      platformId: string,
      networkId: string,
      params: RequestParams = {},
    ) =>
      this.request<
        DockerNetworkDetails,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/networks/${platformId}/${networkId}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Networks
     * @name NetworksCreate
     * @summary Create a network
     * @request POST:/api/v1/networks
     * @secure
     * @response `200` `CreateNetworkView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `500` `ProblemDetails` Internal Server Error
     */
    networksCreate: (data: CreateNetworkInput, params: RequestParams = {}) =>
      this.request<
        CreateNetworkView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/networks`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Networks
     * @name NetworksDelete
     * @summary Delete a network(s)
     * @request DELETE:/api/v1/networks
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    networksDelete: (data: DeleteNetworksInput, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/networks`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Volumes
     * @name VolumesList
     * @summary List all volumes
     * @request GET:/api/v1/volumes/{id}
     * @secure
     * @response `200` `VolumesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `500` `ProblemDetails` Internal Server Error
     */
    volumesList: (
      id: string,
      query?: {
        /** @default null */
        Dangling?: boolean;
        /** @default null */
        Driver?: string;
        /** @default null */
        Name?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<VolumesView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/volumes/${id}`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Volumes
     * @name VolumesInspect
     * @summary Inspect a volume
     * @request GET:/api/v1/volumes/{platformId}/{name}
     * @secure
     * @response `200` `DockerVolume` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `500` `ProblemDetails` Internal Server Error
     */
    volumesInspect: (
      platformId: string,
      name: string,
      params: RequestParams = {},
    ) =>
      this.request<DockerVolume, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/volumes/${platformId}/${name}`,
          method: "GET",
          secure: true,
          format: "json",
          ...params,
        },
      ),

    /**
     * No description
     *
     * @tags Volumes
     * @name VolumesDelete
     * @summary Delete a volume(s)
     * @request DELETE:/api/v1/volumes
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    volumesDelete: (data: DeleteVolumesInput, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/volumes`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Volumes
     * @name VolumesCreate
     * @summary Create a volume
     * @request POST:/api/v1/volumes
     * @secure
     * @response `200` `DockerVolume` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `500` `ProblemDetails` Internal Server Error
     */
    volumesCreate: (data: CreateVolumeInput, params: RequestParams = {}) =>
      this.request<DockerVolume, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/volumes`,
          method: "POST",
          body: data,
          secure: true,
          type: ContentType.Json,
          format: "json",
          ...params,
        },
      ),
  };
}
