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
  Custom = "Custom",
  DockerHub = "DockerHub",
  Azure = "Azure",
  AWS = "AWS",
  Gitlab = "Gitlab",
  GitHub = "GitHub",
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

export enum PlatformConnectorType {
  Unknown = "Unknown",
  Local = "Local",
  Agent = "Agent",
}

export enum LoggingDriverType {
  None = "none",
  Local = "local",
  JsonFile = "json-file",
  Syslog = "syslog",
  Journald = "journald",
  Gelf = "gelf",
  Fluentd = "fluentd",
  Awslogs = "awslogs",
  Splunk = "splunk",
  Etwlogs = "etwlogs",
}

export enum GhcrAccountType {
  Organization = "Organization",
  User = "User",
}

export enum DockerHubTagStatus {
  Active = "Active",
  Inactive = "Inactive",
}

export enum DockerHubImageStatus {
  Active = "Active",
  Inactive = "Inactive",
}

export enum DeploymentStatus {
  Created = "Created",
  Pending = "Pending",
  Applying = "Applying",
  Healthy = "Healthy",
  Degraded = "Degraded",
  Failed = "Failed",
  RolledBack = "RolledBack",
}

export enum DeploymentSource {
  UI = "UI",
  Git = "Git",
  API = "API",
  CLI = "CLI",
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

export enum ContainerRestartPolicy {
  No = "No",
  Always = "Always",
  OnFailure = "OnFailure",
  UnlessStopped = "UnlessStopped",
}

export interface BindOptions {
  propagation: null | string;
  nonRecursive: null | boolean;
  createMountpoint: null | boolean;
  readOnlyNonRecursive: null | boolean;
  readOnlyForceRecursive: null | boolean;
}

export interface ClusterVolume {
  id: string;
  version: null | VolumeVersionInfo;
  createdAt: string;
  updatedAt: string;
  spec: null | VolumeSpecification;
  info: null | ClusterVolumeInfo;
  publishStatus: VolumePublishStatus[];
}

export interface ClusterVolumeInfo {
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  capacityBytes: null | number | string;
  volumeContext: Record<string, string>;
  volumeID: string;
  accessibleTopology: TopologyEntry[];
}

export interface ComposeDeploymentEvent {
  stepId?: string;
  /** @format date-time */
  timestamp?: any;
}

export interface ComposeUpRequest {
  /** @format uuid */
  platformId: string;
  registryName: string;
  repositoryName: string;
  composeFileAsStr: string;
}

export interface ConfigFromInput {
  network: string;
}

export interface ContainerConfiguration {
  hostname: null | string;
  domainname: null | string;
  user: null | string;
  attachStdin: null | boolean;
  attachStdout: null | boolean;
  attachStderr: null | boolean;
  exposedPorts: null | any[];
  tty: null | boolean;
  openStdin: null | boolean;
  stdinOnce: null | boolean;
  env: string[];
  cmd: string[];
  image: null | string;
  volumes: null | any[];
  workingDir: null | string;
  entrypoint: string[];
  networkDisabled: null | boolean;
  macAddress: null | string;
  onBuild: string[];
  labels: Record<string, string>;
}

export interface ContainerHealthStatus {
  status: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  failingStreak: null | number | string;
}

export interface ContainerImageResult {
  id: string;
  name: string;
  state: ContainerStateStatus;
  volumes: string[];
  networks: Record<string, string>;
  ports: Record<string, HostPortBinding[]>;
}

export interface ContainerInfoView {
  name: string;
  containerId: string;
  /** @format uuid */
  platformId: string;
  startedAt: string;
  finishedAt: string;
  platformName: string;
  volumes: string[];
  ports: Record<string, HostPortBinding[]>;
  networks: Record<string, string>;
  state: ContainerStateStatus;
  imageView: null | ImageView;
}

export interface ContainerInspectView {
  id: string;
  created: string;
  path: null | string;
  state: null | ContainerRuntimeState;
  image: null | string;
  resolvConfPath: null | string;
  hostnamePath: null | string;
  hostsPath: null | string;
  logPath: null | string;
  name: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  restartCount: null | number | string;
  driver: null | string;
  platform: null | string;
  mountLabel: null | string;
  processLabel: null | string;
  appArmorProfile: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  sizeRw: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  sizeRootFs: null | number | string;
  args: null | any[];
  execIDs: string[];
  mounts: null | any[];
  hostConfig: null | HostConfiguration;
  graphDriver: null | GraphDriverDataInfo;
  config: null | ContainerConfiguration;
  networkSettings: null | NetworkSettingsInfo;
}

export interface ContainerRuntimeState {
  status: ContainerStateStatus;
  running: null | boolean;
  paused: null | boolean;
  restarting: null | boolean;
  oomKilled: null | boolean;
  dead: null | boolean;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  pid: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  exitCode: null | number | string;
  error: null | string;
  startedAt: null | string;
  finishedAt: null | string;
  health: null | ContainerHealthStatus;
}

export interface ContainerStatView {
  /** @format uuid */
  containerId?: string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  memoryActive?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  memoryCache?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  cpuUsage?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  memoryLimit?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  rxBytes?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  txBytes?: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  created?: number | string;
}

export interface ContainerStatsView {
  stats: ContainerStatView[];
}

export interface ContainerView {
  /** @format uuid */
  id: string;
  /** @format uuid */
  platformId: string;
  containerId: string;
  name: string;
  dockerImageId: string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  created: number | string;
  state: ContainerStateStatus;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  updated: number | string;
  stack: null | string;
  lastStats: null | ContainerStatView;
  ports: Record<string, HostPortBinding[]>;
  platform?: null | PlatformView;
  imageView?: null | ImageView;
  metadata?: null | EndpointMetadata;
}

export interface ContainerVolumeResult {
  id: string;
  name: string;
  image: string;
  imageId: string;
  state: ContainerStateStatus;
  networks: Record<string, string>;
  ports: Record<string, HostPortBinding[]>;
}

export interface ContainersView {
  containers: ContainerView[];
}

export interface CreateContainerInput {
  /** @format uuid */
  platformId: string;
  imageId: string;
  name: null | string;
  workingDir: null | string;
  user: null | string;
  /**
   * @format float
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  memoryLimit: null | number | string;
  /**
   * @format float
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  cpuLimit: null | number | string;
  /**
   * @format float
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  memoryReservation: null | number | string;
  autoRemove: null | boolean;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  stopTimeout: null | number | string;
  restartPolicy: ContainerRestartPolicy;
  labels: null | object;
  envVars: null | any[];
  ports: null | any[];
  volumes: null | any[];
  networks: null | any[];
  entryPoint: null | any[];
  command: null | any[];
  hostname?: null | string;
  dns?: null | any[];
  security?: null | SecurityConfig;
  loggingConfig?: null | LoggingConfig;
  healthCheck?: null | HealthCheckConfig;
}

export interface CreateContainerView {
  id: string;
}

export interface CreateNetworkInput {
  /** @format uuid */
  platformId: string;
  name: string;
  driver: string;
  scope: string;
  internal: null | boolean;
  attachable: null | boolean;
  ingress: null | boolean;
  enableIPv6: null | boolean;
  enableIPv4: null | boolean;
  configOnly: null | boolean;
  ipam?: null | IPAMInput;
  configFrom?: null | ConfigFromInput;
  labels?: null | object;
  options?: null | object;
}

export interface CreateNetworkView {
  id: string;
}

export interface CreateVolumeInput {
  /** @format uuid */
  platformId: string;
  name: string;
  driver: string;
  labels?: null | object;
  options?: null | object;
}

export interface DeleteContainersRequest {
  containerIds: string[];
  /** @default false */
  v?: null | boolean;
  /** @default false */
  force?: null | boolean;
  /** @default false */
  link?: null | boolean;
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

export interface DeletePlatformsInput {
  ids: string[];
}

export interface DeleteRegistriesInput {
  ids: string[];
}

export interface DeleteVolumesInput {
  /** @format uuid */
  platformId: string;
  names: string[];
  force: null | boolean;
}

export interface DeploymentSpec {
  imageId: string;
  name: null | string;
  workingDir: null | string;
  user: null | string;
  /**
   * @format float
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  memoryLimit: null | number | string;
  /**
   * @format float
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  cpuLimit: null | number | string;
  /**
   * @format float
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  memoryReservation: null | number | string;
  autoRemove: null | boolean;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  stopTimeout: null | number | string;
  restartPolicy: ContainerRestartPolicy;
  labels: null | object;
  envVars: null | any[];
  ports: null | any[];
  volumes: null | any[];
  networks: null | any[];
  entryPoint: null | any[];
  command: null | any[];
  hostname?: null | string;
  dns?: null | any[];
  security?: null | SecurityConfig;
  loggingConfig?: null | LoggingConfig;
  healthCheck?: null | HealthCheckConfig;
  metadata?: null | object;
}

export interface DeploymentVersionView {
  /** @format uuid */
  id: string;
  /** @format uuid */
  deploymentId: string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  version: number | string;
  /** @format uuid */
  platformId: string;
  spec: DeploymentSpec;
  status: DeploymentStatus;
  source: DeploymentSource;
  /** @format uuid */
  createdBy: string;
  /** @format date-time */
  createdAt: any;
  /** @format date-time */
  updatedAt: any;
  /** @format uuid */
  updatedBy: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  rolledBackFromVersion: null | number | string;
  gitRepoUrl: null | string;
  gitPath: null | string;
  gitCommitHash: null | string;
}

export interface DeploymentView {
  /** @format uuid */
  id: string;
  name: string;
  description: null | string;
  /** @format date-time */
  createdAt: any;
  /** @format date-time */
  updatedAt: any;
  /** @format uuid */
  createdBy: string;
  /** @format uuid */
  updatedBy: null | string;
  activeVersion: null | DeploymentVersionView;
}

export interface DeploymentsView {
  deployments: DeploymentView[];
}

export interface DockerHubImageView {
  architecture: string;
  digest: string;
  os: string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  size: number | string;
  status: DockerHubImageStatus;
  lastPulled: string;
}

export interface DockerHubRepositoryInfo {
  name: null | string;
  namespace: null | string;
  /** @format date-time */
  lastUpdated: any;
  isPrivate: boolean;
  isTrusted: boolean;
  isAutomated: boolean;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  pullCount: number | string;
}

export interface DockerHubTagView {
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  id: number | string;
  name: string;
  image: null | DockerHubImageView;
  lastUpdated: string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  fullSize: number | string;
  status: DockerHubTagStatus;
  lastPulled: string;
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
  configFrom: null | string;
  ipam: null | IpAddressManagementConfig;
  options: Record<string, string>;
  labels: Record<string, string>;
  containers: Record<string, NetworkConnectedContainer>;
  peers: NetworkPeerInfo[];
}

export interface DockerNetworkResult {
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
  configFrom: null | string;
  ipam: null | IpAddressManagementConfig;
  options: Record<string, string>;
  labels: Record<string, string>;
}

export interface DockerVolumeResult {
  id: string;
  name: string;
  inUse: boolean;
  scope: string;
  driver: string;
  mountpoint: string;
  createdAt: string;
  clusterVolume: null | ClusterVolume;
  usageData: null | VolumeUsageData;
  containers: ContainerVolumeResult[];
  status: Record<string, string>;
  labels: Record<string, string>;
  options: Record<string, string>;
}

export interface DriverConfiguration {
  name: null | string;
  options: Record<string, string>;
}

export interface EndpointIpamConfiguration {
  ipv4Address: null | string;
  ipv6Address: null | string;
  linkLocalIPs: string[];
}

export interface EndpointMetadata {
  /** @default false */
  canEdit?: null | boolean;
  /** @default false */
  canDelete?: null | boolean;
}

export interface EndpointSettingsInfo {
  ipamConfig: null | EndpointIpamConfiguration;
  links: string[];
  macAddress: null | string;
  aliases: string[];
  networkID: null | string;
  endpointID: null | string;
  gateway: null | string;
  ipAddress: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  ipPrefixLen: null | number | string;
  ipv6Gateway: null | string;
  globalIPv6Address: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  globalIPv6PrefixLen: null | number | string;
  driverOpts: Record<string, string>;
  dnsNames: string[];
}

export interface GitHubCrPackageVersion {
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  id: number | string;
  url: string;
  name: string;
  htmlUrl: null | string;
  createdAt: null | string;
  updatedAt: null | string;
  packageHtmlUrl: null | string;
  metadata: null | GitHubCrPackageVersionMetadata;
}

export interface GitHubCrPackageVersionContainerMetadata {
  tags: null | any[];
}

export interface GitHubCrPackageVersionMetadata {
  container: null | GitHubCrPackageVersionContainerMetadata;
}

export interface GraphDriverDataInfo {
  name: null | string;
  data: Record<string, string>;
}

export interface HealthCheckConfig {
  test: string[];
  /** @default "30s" */
  interval?: string;
  /** @default "5s" */
  timeout?: string;
  /**
   * @format int32
   * @default 3
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  retries?: number | string;
  /** @default "0s" */
  startPeriod?: string;
}

export interface HistoryImageResult {
  id: string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  created: number | string;
  createdBy: string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  size: number | string;
  comment: string;
}

export interface HostConfiguration {
  binds: string[];
  containerIDFile: null | string;
  logConfig: null | LogConfiguration;
  networkMode: null | string;
  portBindings: Record<string, HostPortBinding[]>;
  restartPolicy: null | RestartPolicy;
  autoRemove: null | boolean;
  volumeDriver: null | string;
  volumesFrom: string[];
  mounts: HostMount[];
  consoleSize: (number | string)[];
  annotations: Record<string, string>;
  capAdd: string[];
  capDrop: string[];
  cgroupnsMode: null | string;
  dns: string[];
  dnsOptions: string[];
  dnsSearch: string[];
  extraHosts: string[];
  groupAdd: string[];
  ipcMode: null | string;
  cgroup: null | string;
  links: string[];
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  oomScoreAdj: null | number | string;
  pidMode: null | string;
  privileged: null | boolean;
  publishAllPorts: null | boolean;
  readonlyRootfs: null | boolean;
  securityOpt: string[];
  storageOpt: Record<string, string>;
  tmpfs: Record<string, string>;
  utsMode: null | string;
  usernsMode: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  shmSize: number | string;
  sysctls: Record<string, string>;
  runtime: null | string;
  isolation: null | string;
  maskedPaths: string[];
  readonlyPaths: string[];
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  memorySwap: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  memorySwappiness: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  nanoCpus: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  pidsLimit: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  memory: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  memoryReservation: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  ioMaximumBandwidth: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cpuPeriod: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cpuPercent: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cpuCount: null | number | string;
  ulimits: Ulimit[];
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  kernelMemoryTCP: null | number | string;
}

export interface HostMount {
  target: null | string;
  source: null | string;
  type: null | string;
  readOnly: null | boolean;
  consistency: null | string;
  bindOptions: null | BindOptions;
  volumeOptions: null | VolumeOptions;
}

export interface HostPortBinding {
  hostIP?: null | string;
  hostPort?: null | string;
}

export interface HttpValidationProblemDetails {
  type?: null | string;
  title?: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  status?: null | number | string;
  detail?: null | string;
  instance?: null | string;
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
  name?: string;
  namespace?: null | string;
  /** @format date-time */
  lastUpdated?: any;
  isPrivate?: boolean;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  pullCount?: number | string;
}

export interface IImageRepositoryGitHubPackageResponse {
  $type?: "GitHub";
  id?: string;
  name?: string;
  createdAt?: null | string;
  updatedAt?: null | string;
  url?: null | string;
  htmlUrl?: null | string;
}

export interface IPAMConfigInput {
  subnet: string;
  ipRange: string;
  gateway: string;
}

export interface IPAMInput {
  driver: string;
  config?: null | any[];
  options?: null | object;
}

export interface ImagePullError {
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  code: null | number | string;
  message: null | string;
}

export interface ImagePullProgress {
  units: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  current: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  total: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  start: null | number | string;
}

export interface ImageView {
  /** @format uuid */
  id: string;
  tags: string[];
  name: string;
  dockerImageId: string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  size: number | string;
  isInUse: boolean;
  /** @format uuid */
  platformId: string;
  /** @format date-time */
  createdAt: any;
  isUpToDate?: null | boolean;
  updatedAt?: any;
  /** @format uuid */
  registryId?: null | string;
  registry?: null | RegistryView;
}

export interface ImagesView {
  images: ImageView[];
}

export interface InspectImageView {
  id: string;
  name: string;
  tag: string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  size: number | string;
  os: string;
  created: string;
  architecture: string;
  env: string[];
  cmd: string[];
  repoTags: string[];
  volumes: string[];
  exposedPorts: string[];
  layers: HistoryImageResult[];
  labels: Record<string, string>;
  containers: ContainerImageResult[];
  registry: null | RegistryView;
}

export interface IpAddressInfo {
  addr: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  prefixLen: null | number | string;
}

export interface IpAddressManagementConfig {
  driver: null | string;
  config: IpamSubnetConfiguration[];
  options: Record<string, string>;
}

export interface IpamSubnetConfiguration {
  subnet: null | string;
  ipRange: null | string;
  gateway: null | string;
}

export interface LogConfiguration {
  type: null | string;
  config: Record<string, string>;
}

export interface LoggingConfig {
  driver: LoggingDriverType;
  options?: null | object;
}

export interface LoginRequest {
  email: string;
  password: string;
}

export interface LoginResponse {
  accessToken: string;
}

export interface MountPointInfo {
  type: null | string;
  name: null | string;
  source: null | string;
  destination: null | string;
  driver: null | string;
  mode: null | string;
  rw: null | boolean;
  propagation: null | string;
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

export interface NetworkSettingsInfo {
  bridge: null | string;
  sandboxID: null | string;
  hairpinMode: null | boolean;
  linkLocalIPv6Address: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  linkLocalIPv6PrefixLen: null | number | string;
  sandboxKey: null | string;
  secondaryIPAddresses: IpAddressInfo[];
  secondaryIPv6Addresses: IpAddressInfo[];
  endpointID: null | string;
  gateway: null | string;
  globalIPv6Address: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  globalIPv6PrefixLen: null | number | string;
  ipAddress: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  ipPrefixLen: null | number | string;
  ipv6Gateway: null | string;
  macAddress: null | string;
  ports: Record<string, HostPortBinding[]>;
  networks: Record<string, EndpointSettingsInfo>;
}

export interface NetworksView {
  networks: DockerNetworkResult[];
}

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
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  containerCount: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  containersRunning: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  containersPaused: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  containersStopped: number | string;
  driver?: null | string;
  operatingSystem?: null | string;
  osVersion?: null | string;
  osType?: null | string;
  architecture?: null | string;
}

export interface PlatformDescriptorDockerSwarmPlatformDescriptor {
  $type?: "DockerSwarm";
  nodeID: string;
  nodeAddr: string;
  localNodeState: string;
  controlAvailable: boolean;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  nodes: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  managers: number | string;
  error?: null | string;
  remoteManagers?: null | any[];
  daemonId: string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  containerCount: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  containersRunning: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  containersPaused: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  containersStopped: number | string;
  driver?: null | string;
  operatingSystem?: null | string;
  osVersion?: null | string;
  osType?: null | string;
  architecture?: null | string;
}

export interface PlatformDescriptorKubernetesPlatformDescriptor {
  $type?: "Kubernetes";
  clusterName: null | string;
  clusterVersion: null | string;
  apiServerUrl: null | string;
  namespace: null | string;
}

export interface PlatformInput {
  name: string;
  address: null | string;
  type?: PlatformType;
  connectorType?: PlatformConnectorType;
}

export interface PlatformStatView {
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  created?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  txBytes?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  rxBytes?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  cpuUsage?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  memoryUsage?: number | string;
}

export interface PlatformView {
  /** @format uuid */
  id: string;
  name: string;
  address: string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  networkCount: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  volumeCount: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  imageCount: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cpuCount: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  memTotal: number | string;
  agentVersion: null | string;
  serverVersion: null | string;
  type: PlatformType;
  status: PlatformStatus;
  connectorType: PlatformConnectorType;
  stats: null | any[];
  platformDescriptor: null | PlatformDescriptor;
}

export interface PlatformsView {
  platforms: PlatformView[];
}

export interface ProblemDetails {
  type?: null | string;
  title?: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  status?: null | number | string;
  detail?: null | string;
  instance?: null | string;
}

export interface PullImageRequest {
  /** @format uuid */
  platformId: string;
  registryName: string;
  repositoryName: string;
  imageTag: string;
}

export interface PullImageResult {
  id?: null | string;
  from?: null | string;
  stream?: null | string;
  status?: null | string;
  errorMessage?: null | string;
  progressMessage?: null | string;
  progress?: null | ImagePullProgress;
  error?: null | ImagePullError;
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
    | BaseRegistryConfigurationBaseTypeMapping<
        "Custom",
        RegistryConfigurationBaseCustomRegistry
      >
  );

export interface RegistryConfigurationBaseAWSRegistry {
  $type?: "AWS";
  accessKey: string;
  authenticationRequired: boolean;
  secretAccessKey: string;
  region: string;
}

export interface RegistryConfigurationBaseAzureRegistry {
  $type?: "Azure";
  userName: string;
  password: string;
}

export interface RegistryConfigurationBaseCustomRegistry {
  $type?: "Custom";
  /** @default false */
  authEnabled?: null | boolean;
  userName?: null | string;
  password?: null | string;
}

export interface RegistryConfigurationBaseDockerHubRegistry {
  $type?: "DockerHub";
  userName?: null | string;
  pat?: null | string;
}

export interface RegistryConfigurationBaseGitHubRegistry {
  $type?: "GitHub";
  name: string;
  pat: string;
  type: GhcrAccountType;
}

export interface RegistryConfigurationBaseGitlabRegistry {
  $type?: "Gitlab";
  userName: string;
  pat: string;
  instanceUrl: string;
}

export interface RegistryInput {
  name: string;
  registryHost: string;
  type: RegistryType;
  configuration: RegistryConfigurationBase;
}

export interface RegistryView {
  /** @format uuid */
  id: string;
  name: string;
  registryHost: string;
  type: RegistryType;
  /** @format date-time */
  created: any;
  isDefault?: boolean;
}

export interface RegistryWithConfigView {
  /** @format uuid */
  id: string;
  name: string;
  registryHost: string;
  type: RegistryType;
  /** @format date-time */
  created: any;
  configuration: null | RegistryConfigurationBase;
}

export interface RestartPolicy {
  name: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  maximumRetryCount: null | number | string;
}

export interface RunImageInfoResult {
  volumes: string[];
  networks: string[];
  exposedPorts: string[];
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  memTotal: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  cpuCount: number | string;
}

export interface SecurityConfig {
  privileged: boolean;
  capAdd: string[];
  capDrop: string[];
  readOnlyRootFs: boolean;
  securityOpt: string[];
}

export interface SwarmPeer {
  nodeID: null | string;
  addr: null | string;
}

export interface TopologyEntry {
  labels: Record<string, string>;
}

export interface Ulimit {
  name: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  soft: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  hard: null | number | string;
}

export interface VolumeAccessMode {
  scope: VolumeScope;
  sharing: VolumeSharing;
  secrets: VolumeSecret[];
  capacityRange: null | VolumeCapacityRange;
  availability: string;
}

export interface VolumeCapacityRange {
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  requiredBytes: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  limitBytes: null | number | string;
}

export interface VolumeOptions {
  noCopy: null | boolean;
  labels: Record<string, string>;
  driverConfig: null | DriverConfiguration;
  subpath: null | string;
}

export interface VolumePublishStatus {
  nodeID: string;
  state: string;
  publishContext: Record<string, string>;
}

export interface VolumeSecret {
  key: string;
  secret: string;
}

export interface VolumeSpecification {
  group: string;
  accessMode: null | VolumeAccessMode;
}

export interface VolumeUsageData {
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  size: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  refCount: null | number | string;
}

export interface VolumeVersionInfo {
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  index: null | number | string;
}

export interface VolumesView {
  volumes: DockerVolumeResult[];
}

type BaseIImageRepository = object;

type BaseIImageRepositoryTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BasePlatformDescriptor = object;

type BasePlatformDescriptorTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BaseRegistryConfigurationBase = object;

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
  JsonApi = "application/vnd.api+json",
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
    [ContentType.JsonApi]: (input: any) =>
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
     * @name RefreshToken
     * @summary Request a new access token
     * @request GET:/api/v1/authentication/refresh
     * @response `200` `RefreshTokenResponse` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    refreshToken: (params: RequestParams = {}) =>
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
     * @name Login
     * @summary Check user credentials and issue an access token on successful login
     * @request POST:/api/v1/authentication/login
     * @response `200` `LoginResponse` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    login: (data: LoginRequest, params: RequestParams = {}) =>
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
     * @name Logout
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
    logout: (params: RequestParams = {}) =>
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
     * @name GetContainer
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
    getContainer: (id: string, params: RequestParams = {}) =>
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
     * @name GetContainerInfo
     * @summary Get basic container details
     * @request GET:/api/v1/containers/{id}/info
     * @secure
     * @response `200` `ContainerInfoView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getContainerInfo: (id: string, params: RequestParams = {}) =>
      this.request<
        ContainerInfoView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/containers/${id}/info`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name GetContainerStats
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
    getContainerStats: (id: string, params: RequestParams = {}) =>
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
     * @name InspectContainer
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
    inspectContainer: (id: string, params: RequestParams = {}) =>
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
     * @name CreateContainer
     * @summary Create a container
     * @request POST:/api/v1/containers
     * @secure
     * @response `200` `CreateContainerView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createContainer: (data: CreateContainerInput, params: RequestParams = {}) =>
      this.request<
        CreateContainerView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/containers`,
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
     * @tags Containers
     * @name DeleteContainers
     * @summary Delete the given container(s)
     * @request DELETE:/api/v1/containers
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteContainers: (
      data: DeleteContainersRequest,
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name StartContainers
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
    startContainers: (data: string[], params: RequestParams = {}) =>
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
     * @name StopContainers
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
    stopContainers: (data: string[], params: RequestParams = {}) =>
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
     * @name PauseContainers
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
    pauseContainers: (data: string[], params: RequestParams = {}) =>
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
     * @name RestartContainers
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
    restartContainers: (data: string[], params: RequestParams = {}) =>
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
     * @name UnpauseContainers
     * @summary Resume a container(s) which has been paused
     * @request PATCH:/api/v1/containers/unpause
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    unpauseContainers: (data: string[], params: RequestParams = {}) =>
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
     * @tags Platforms
     * @name ListPlatforms
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
    listPlatforms: (params: RequestParams = {}) =>
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
     * @name CreatePlatform
     * @summary Create a platform
     * @request POST:/api/v1/platforms
     * @secure
     * @response `200` `PlatformView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createPlatform: (data: PlatformInput, params: RequestParams = {}) =>
      this.request<PlatformView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/platforms`,
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
     * @tags Platforms
     * @name DeletePlatforms
     * @summary Delete platforms
     * @request DELETE:/api/v1/platforms
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deletePlatforms: (data: DeletePlatformsInput, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/platforms`,
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
     * @name GetPlatfom
     * @summary Get platform by Id
     * @request GET:/api/v1/platforms/{id}
     * @secure
     * @response `200` `PlatformView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getPlatfom: (id: string, params: RequestParams = {}) =>
      this.request<PlatformView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/platforms/${id}`,
          method: "GET",
          secure: true,
          format: "json",
          ...params,
        },
      ),

    /**
     * No description
     *
     * @tags Platforms
     * @name UpdatePlatform
     * @summary Patch a platform
     * @request PATCH:/api/v1/platforms/{id}
     * @secure
     * @response `200` `PlatformView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updatePlatform: (
      id: string,
      data: PlatformInput,
      params: RequestParams = {},
    ) =>
      this.request<PlatformView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/platforms/${id}`,
          method: "PATCH",
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
     * @tags Platforms
     * @name ListContainers
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
    listContainers: (id: string, params: RequestParams = {}) =>
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
     * @name ListRegistries
     * @summary Get all registries
     * @request GET:/api/v1/registries
     * @secure
     * @response `200` `RegistriesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listRegistries: (params: RequestParams = {}) =>
      this.request<
        RegistriesView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/registries`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * @description A discriminator should be provided in the request, this discriminator is based on RegistryType enum
     *
     * @tags Registries
     * @name CreateRegistry
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
    createRegistry: (data: RegistryInput, params: RequestParams = {}) =>
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
     * @name DeleteRegistries
     * @summary Delete registries
     * @request DELETE:/api/v1/registries
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteRegistries: (
      data: DeleteRegistriesInput,
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/registries`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Registries
     * @name GetRegistry
     * @summary Get registry by ID
     * @request GET:/api/v1/registries/{id}
     * @secure
     * @response `200` `RegistryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getRegistry: (id: string, params: RequestParams = {}) =>
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
     * @name UpdateRegistry
     * @summary Update a registry
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
    updateRegistry: (
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
     * No description
     *
     * @tags Registries
     * @name GetRegistryWithConfig
     * @summary Get registry and it's configuration
     * @request GET:/api/v1/registries/{id}/_cfg
     * @secure
     * @response `200` `RegistryWithConfigView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getRegistryWithConfig: (id: string, params: RequestParams = {}) =>
      this.request<
        RegistryWithConfigView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/registries/${id}/_cfg`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name ListImages
     * @summary Get all local images for the given platform
     * @request GET:/api/v1/images/{platformId}
     * @secure
     * @response `200` `ImagesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listImages: (platformId: string, params: RequestParams = {}) =>
      this.request<ImagesView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/images/${platformId}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name GetExternalRepositories
     * @summary List external repositories of the given registry
     * @request GET:/api/v1/images/{registryName}/repositories
     * @secure
     * @response `200` `(IImageRepository)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getExternalRepositories: (
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
     * @name GetGhcrPackageVersions
     * @summary List versions of GHCR package
     * @request GET:/api/v1/images/ghcr/{registryName}/{packageName}/versions
     * @secure
     * @response `200` `(GitHubCrPackageVersion)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getGhcrPackageVersions: (
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
     * @name GetDockerHubRepositories
     * @summary List DockerHub repositories
     * @request GET:/api/v1/images/dockerhub/{registryName}/repositories
     * @secure
     * @response `200` `(DockerHubRepositoryInfo)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getDockerHubRepositories: (
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
     * @name GetDockerHubRepositoryTags
     * @summary List DockerHub repository tags
     * @request GET:/api/v1/images/dockerhub/{registryName}/{repositoryName}/tags
     * @secure
     * @response `200` `(DockerHubTagView)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getDockerHubRepositoryTags: (
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
     * @name InspectImage
     * @summary Inspect an image
     * @request GET:/api/v1/images/{platformId}/{imageId}
     * @secure
     * @response `200` `InspectImageView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    inspectImage: (
      platformId: string,
      imageId: string,
      params: RequestParams = {},
    ) =>
      this.request<
        InspectImageView,
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
     * @name GetImageInfo
     * @summary Get image info
     * @request GET:/api/v1/images/{platformId}/{imageId}/_info
     * @secure
     * @response `200` `RunImageInfoResult` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getImageInfo: (
      platformId: string,
      imageId: string,
      params: RequestParams = {},
    ) =>
      this.request<
        RunImageInfoResult,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images/${platformId}/${imageId}/_info`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name PullImage
     * @summary Pull an image from a registry and returns logs as a stream
     * @request POST:/api/v1/images/pull
     * @secure
     * @response `200` `(PullImageResult)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    pullImage: (data: PullImageRequest, params: RequestParams = {}) =>
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
     * @name DeleteImages
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
    deleteImages: (data: DeleteImagesRequest, params: RequestParams = {}) =>
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
     * @name ListNetworks
     * @summary List all networks
     * @request GET:/api/v1/networks/{platformId}
     * @secure
     * @response `200` `NetworksView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listNetworks: (
      platformId: string,
      query?: {
        Dangling?: boolean;
        Driver?: string;
        Id?: string;
        Name?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<NetworksView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/networks/${platformId}`,
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
     * @name InspectNetwork
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
    inspectNetwork: (
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
     * @name CreateNetwork
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
    createNetwork: (data: CreateNetworkInput, params: RequestParams = {}) =>
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
     * @name DeleteNetworks
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
    deleteNetworks: (data: DeleteNetworksInput, params: RequestParams = {}) =>
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
     * @name ListVolumes
     * @summary List all volumes
     * @request GET:/api/v1/volumes/{platformId}
     * @secure
     * @response `200` `VolumesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listVolumes: (
      platformId: string,
      query?: {
        Dangling?: boolean;
        Driver?: string;
        Name?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<VolumesView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/volumes/${platformId}`,
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
     * @name InspectVolume
     * @summary Inspect a volume
     * @request GET:/api/v1/volumes/{platformId}/{name}
     * @secure
     * @response `200` `DockerVolumeResult` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `500` `ProblemDetails` Internal Server Error
     */
    inspectVolume: (
      platformId: string,
      name: string,
      params: RequestParams = {},
    ) =>
      this.request<
        DockerVolumeResult,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/volumes/${platformId}/${name}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Volumes
     * @name CreateVolume
     * @summary Create a volume
     * @request POST:/api/v1/volumes
     * @secure
     * @response `200` `DockerVolumeResult` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createVolume: (data: CreateVolumeInput, params: RequestParams = {}) =>
      this.request<
        DockerVolumeResult,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/volumes`,
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
     * @tags Volumes
     * @name DeleteVolumes
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
    deleteVolumes: (data: DeleteVolumesInput, params: RequestParams = {}) =>
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
     * @tags Compose
     * @name ComposeUp
     * @summary Deploy a stack
     * @request POST:/api/v1/compose/up
     * @secure
     * @response `200` `(ComposeDeploymentEvent)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    composeUp: (data: ComposeUpRequest, params: RequestParams = {}) =>
      this.request<
        ComposeDeploymentEvent[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/compose/up`,
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
     * @tags Deployments
     * @name ListDeployments
     * @summary List all deployments
     * @request GET:/api/v1/deployments
     * @secure
     * @response `200` `DeploymentsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listDeployments: (params: RequestParams = {}) =>
      this.request<
        DeploymentsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/deployments`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name GetDeployment
     * @summary Get deployment by Id
     * @request GET:/api/v1/deployments/{deploymentId}
     * @secure
     * @response `200` `DeploymentView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getDeployment: (deploymentId: string, params: RequestParams = {}) =>
      this.request<
        DeploymentView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/deployments/${deploymentId}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),
  };
}
