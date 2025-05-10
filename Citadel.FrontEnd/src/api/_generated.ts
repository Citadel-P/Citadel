/* eslint-disable */
/* tslint:disable */
/*
 * ---------------------------------------------------------------
 * ## THIS FILE WAS GENERATED VIA SWAGGER-TYPESCRIPT-API        ##
 * ##                                                           ##
 * ## AUTHOR: acacode                                           ##
 * ## SOURCE: https://github.com/acacode/swagger-typescript-api ##
 * ---------------------------------------------------------------
 */

export interface Address {
  addr?: string | null;
  /** @format int64 */
  prefixLen?: number | null;
}

export enum AppPermission {
  None = 'None',
  ListUsers = 'ListUsers',
  AddUsers = 'AddUsers',
  EditUsers = 'EditUsers',
  DeleteUsers = 'DeleteUsers',
  ListRoles = 'ListRoles',
  AddRoles = 'AddRoles',
  EditRoles = 'EditRoles',
  DeleteRoles = 'DeleteRoles',
  ListTeams = 'ListTeams',
  AddTeams = 'AddTeams',
  EditTeams = 'EditTeams',
  DeleteTeams = 'DeleteTeams',
  ListPlatforms = 'ListPlatforms',
  AddPlatforms = 'AddPlatforms',
  EditPlatforms = 'EditPlatforms',
  DeletePlatforms = 'DeletePlatforms',
  ListContainers = 'ListContainers',
  AddContainers = 'AddContainers',
  EditContainers = 'EditContainers',
  DeleteContainers = 'DeleteContainers',
  ListNetworks = 'ListNetworks',
  AddNetworks = 'AddNetworks',
  EditNetworks = 'EditNetworks',
  DeleteNetworks = 'DeleteNetworks',
  ListVolumes = 'ListVolumes',
  AddVolumes = 'AddVolumes',
  EditVolumes = 'EditVolumes',
  DeleteVolumes = 'DeleteVolumes',
}

export type AttestationDataView = {
  for: string | null;
};

export type BindOptions = {
  propagation?: string | null;
  nonRecursive?: boolean | null;
  createMountpoint?: boolean | null;
  readOnlyNonRecursive?: boolean | null;
  readOnlyForceRecursive?: boolean | null;
};

export type ClusterVolumeInfoView = {
  /** @format int64 */
  capacityBytes: number | null;
  volumeContext: Record<string, string>;
  volumeID: string | null;
  accessibleTopology: TopologyEntryView[] | null;
};

export type ClusterVolumeView = {
  id: string | null;
  version: VolumVersionView;
  createdAt: string | null;
  updatedAt: string | null;
  spec: VolumeSpecView;
  info: ClusterVolumeInfoView;
  publishStatus: PublishStatusView[] | null;
};

/** @default null */
export type ConfigFromInput = {
  network: string | null;
};

export type ConfigView = {
  hostname: string | null;
  domainname: string | null;
  user: string | null;
  attachStdin: boolean;
  attachStdout: boolean;
  attachStderr: boolean;
  exposedPorts: Record<string, Empty>;
  tty: boolean;
  openStdin: boolean;
  stdinOnce: boolean;
  env: string[] | null;
  cmd: string[] | null;
  healthcheck: HealthcheckView;
  argsEscaped: boolean;
  image: string | null;
  volumes: Record<string, any>;
  workingDir: string | null;
  entrypoint: string[] | null;
  onBuild: string[] | null;
  labels: Record<string, string>;
  stopSignal: string | null;
  shell: string[] | null;
};

export type ContainerConfig = {
  hostname?: string | null;
  domainname?: string | null;
  user?: string | null;
  attachStdin?: boolean | null;
  attachStdout?: boolean | null;
  attachStderr?: boolean | null;
  exposedPorts?: string[] | null;
  tty?: boolean | null;
  openStdin?: boolean | null;
  stdinOnce?: boolean | null;
  env?: string[] | null;
  cmd?: string[] | null;
  image?: string | null;
  volumes?: string[] | null;
  workingDir?: string | null;
  entrypoint?: string[] | null;
  networkDisabled?: boolean | null;
  macAddress?: string | null;
  onBuild?: string[] | null;
  labels?: Record<string, string>;
};

export interface ContainerInfoView {
  /** @format uuid */
  id: string;
  containerId: string | null;
  name: string | null;
  image: string | null;
  /** @format date-time */
  created: string;
  state: ContainerStateStatus;
  status: string | null;
  stack: string | null;
  lastStats: ContainerStatView;
  /** @default null */
  ports?: PortView[] | null;
  platform?: PlatformView;
}

export interface ContainerInspectView {
  id: string | null;
  created: string | null;
  path: string | null;
  args: string[] | null;
  state: ContainerState;
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
  execIDs: string[] | null;
  hostConfig: HostConfig;
  graphDriver: GraphDriverData;
  /** @format int64 */
  sizeRw: number | null;
  /** @format int64 */
  sizeRootFs: number | null;
  mounts: MountPoint[] | null;
  config: ContainerConfig;
  networkSettings: NetworkSettingsView;
}

export interface ContainerLogReply {
  containerId?: string | null;
  log?: string | null;
}

export interface ContainersInfoView {
  containers: ContainerInfoView[] | null;
}

export type ContainerState = {
  status?: ContainerStateType;
  running?: boolean | null;
  paused?: boolean | null;
  restarting?: boolean | null;
  oomKilled?: boolean | null;
  dead?: boolean | null;
  /** @format int32 */
  pid?: number | null;
  /** @format int32 */
  exitCode?: number | null;
  error?: string | null;
  startedAt?: string | null;
  finishedAt?: string | null;
  health?: Health;
};

export enum ContainerStateStatus {
  Unknown = 'Unknown',
  Created = 'Created',
  Running = 'Running',
  Paused = 'Paused',
  Restarting = 'Restarting',
  Exited = 'Exited',
  Removing = 'Removing',
  Dead = 'Dead',
  Offline = 'Offline',
}

export enum ContainerStateType {
  Unknown = 'Unknown',
  Created = 'Created',
  Running = 'Running',
  Paused = 'Paused',
  Restarting = 'Restarting',
  Exited = 'Exited',
  Removing = 'Removing',
  Dead = 'Dead',
}

export interface ContainerStatsView {
  stats: ContainerStatView[] | null;
}

export interface ContainerStatView {
  /** @format double */
  memoryUsage?: number;
  /** @format double */
  cpuUsage?: number;
  /** @format double */
  memoryLimit?: number;
  /** @format int64 */
  rxBytes?: number;
  /** @format int64 */
  txBytes?: number;
  /** @format int64 */
  created?: number;
}

export interface CreateNetworkInput {
  /** @format uuid */
  platformId: string;
  name: string | null;
  driver: string | null;
  scope: string | null;
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
  id: string | null;
}

export interface CreateRegistryInput {
  name: string | null;
  url: string | null;
  discriminator: RegistryDiscriminator;
  configuration: RegistryConfigurationBase;
}

export interface CreateVolumeInput {
  /** @format uuid */
  platformId: string;
  name: string | null;
  driver: string | null;
  /** @default null */
  labels?: Record<string, string>;
  /** @default null */
  options?: Record<string, string>;
}

export interface DeleteContainersRequest {
  containersIds: string[] | null;
  /** @default false */
  v?: boolean | null;
  /** @default false */
  force?: boolean | null;
  /** @default false */
  link?: boolean | null;
}

export interface DeleteImagesReply {
  replies?: DeleteImagesReplyItem[] | null;
}

export interface DeleteImagesReplyItem {
  result?: Record<string, string>;
}

export interface DeleteImagesRequest {
  /** @format uuid */
  platformId: string;
  ids: string[] | null;
  /** @default false */
  force?: boolean;
  /** @default false */
  noPrune?: boolean;
}

export interface DeleteNetworksInput {
  /** @format uuid */
  platformId: string;
  ids: string[] | null;
}

export interface DeleteRegistriesInput {
  ids: string[] | null;
}

export interface DeleteVolumesInput {
  /** @format uuid */
  platformId: string;
  names: string[] | null;
  force: boolean | null;
}

export type DescriptorView = {
  mediaType: string | null;
  digest: string | null;
  /** @format int64 */
  size: number;
  urls: string[] | null;
  annotations: Record<string, string>;
  data: string | null;
  platform: PlatformDescriptorView;
  artifactType: string | null;
};

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
  architecture: string | null;
  digest: string | null;
  os: string | null;
  /** @format int32 */
  size: number;
  status: ImageStatus;
  lastPulled: string | null;
};

export interface DockerHubRepository {
  name?: string | null;
  namespace?: string | null;
  /** @format date-time */
  last_updated?: string;
  is_private?: boolean;
  is_trusted?: boolean;
  is_automated?: boolean;
  /** @format int32 */
  pull_count?: number;
}

export interface DockerHubTagView {
  /** @format int32 */
  id: number;
  name: string | null;
  image: DockerHubImageView;
  lastUpdated: string | null;
  /** @format int32 */
  fullSize: number;
  status: TagStatus;
  lastPulled: string | null;
}

export type DriverConfig = {
  name?: string | null;
  options?: Record<string, string>;
};

export type Empty = object;

export type EndpointIPAMConfig = {
  ipv4Address?: string | null;
  ipv6Address?: string | null;
  linkLocalIPs?: string[] | null;
};

export interface EndpointSettingsView {
  ipamConfig: EndpointIPAMConfig;
  links: string[] | null;
  macAddress: string | null;
  aliases: string[] | null;
  networkID: string | null;
  endpointID: string | null;
  gateway: string | null;
  ipAddress: string | null;
  /** @format int64 */
  ipPrefixLen: number | null;
  iPv6Gateway: string | null;
  globalIPv6Address: string | null;
  /** @format int64 */
  globalIPv6PrefixLen: number | null;
  driverOpts: Record<string, string>;
  dnsNames: string[] | null;
}

export interface GhcrPackageVersion {
  /** @format int32 */
  id?: number;
  name?: string | null;
  url?: string | null;
  package_html_url?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
  html_url?: string | null;
  metadata?: PackageVersionMetadata;
}

export type GraphDriverData = {
  name?: string | null;
  data?: Record<string, string>;
};

export type GraphDriverDataView = {
  mergedDir: string | null;
  upperDir: string | null;
  workDir: string | null;
};

export type GraphDriverView = {
  name: string | null;
  data: GraphDriverDataView;
};

export type Health = {
  status?: string | null;
  /** @format int32 */
  failingStreak?: number | null;
};

export type HealthcheckView = {
  test: string[] | null;
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

export type HostConfig = {
  binds?: string[] | null;
  containerIDFile?: string | null;
  logConfig?: LogConfig;
  networkMode?: string | null;
  portBindings?: MapFieldPortBinding[] | null;
  restartPolicy?: RestartPolicy;
  autoRemove?: boolean | null;
  volumeDriver?: string | null;
  volumesFrom?: string[] | null;
  mounts?: Mount[] | null;
  consoleSize?: number[] | null;
  annotations?: Record<string, string>;
  capAdd?: string[] | null;
  capDrop?: string[] | null;
  cgroupnsMode?: string | null;
  dns?: string[] | null;
  dnsOptions?: string[] | null;
  dnsSearch?: string[] | null;
  extraHosts?: string[] | null;
  groupAdd?: string[] | null;
  ipcMode?: string | null;
  cgroup?: string | null;
  links?: string[] | null;
  /** @format int32 */
  oomScoreAdj?: number | null;
  pidMode?: string | null;
  privileged?: boolean | null;
  publishAllPorts?: boolean | null;
  readonlyRootfs?: boolean | null;
  securityOpt?: string[] | null;
  storageOpt?: Record<string, string>;
  tmpfs?: Record<string, string>;
  utsMode?: string | null;
  usernsMode?: string | null;
  /** @format int64 */
  shmSize?: number;
  sysctls?: Record<string, string>;
  runtime?: string | null;
  isolation?: string | null;
  maskedPaths?: string[] | null;
  readonlyPaths?: string[] | null;
  /** @format int64 */
  memorySwap?: number | null;
  /** @format int64 */
  memorySwappiness?: number | null;
  /** @format int64 */
  nanoCpus?: number | null;
  /** @format int64 */
  pidsLimit?: number | null;
  /** @format int64 */
  memory?: number | null;
  /** @format int64 */
  memoryReservation?: number | null;
  /** @format int64 */
  ioMaximumBandwidth?: number | null;
  /** @format int64 */
  cpuPeriod?: number | null;
  /** @format int64 */
  cpuPercent?: number | null;
  /** @format int64 */
  cpuCount?: number | null;
  ulimits?: Ulimits[] | null;
  /** @format int64 */
  kernelMemoryTCP?: number | null;
};

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
    | BaseIImageRepositoryTypeMapping<'GitHub', IImageRepositoryGitHubPackageResponse>
    | BaseIImageRepositoryTypeMapping<'DockerHub', IImageRepositoryDockerHubRepositoryResponse>
  );

export interface IImageRepositoryDockerHubRepositoryResponse {
  $type?: 'DockerHub';
  name?: string | null;
  namespace?: string | null;
  /** @format date-time */
  lastUpdated?: string;
  isPrivate?: boolean;
  /** @format int32 */
  pullCount?: number;
}

export interface IImageRepositoryGitHubPackageResponse {
  $type?: 'GitHub';
  id?: string | null;
  name?: string | null;
  createdAt?: string | null;
  updatedAt?: string | null;
  url?: string | null;
  htmlUrl?: string | null;
}

export type ImageDataView = {
  platform: PlatformDescriptorView;
  containers: string[] | null;
  size: SizeView;
};

export enum ImageStatus {
  Active = 'Active',
  Inactive = 'Inactive',
}

export interface ImagesView {
  images: ImageView[] | null;
}

export interface ImageView {
  id: string | null;
  /** @format int64 */
  created: number;
  parentId: string | null;
  repoDigests: string[] | null;
  repoTags: string[] | null;
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

export interface InspectImageView {
  id: string | null;
  descriptor: DescriptorView;
  manifests: ManifestView[] | null;
  repoTags: string[] | null;
  repoDigests: string[] | null;
  parent: string | null;
  comment: string | null;
  created: string | null;
  dockerVersion: string | null;
  author: string | null;
  config: ConfigView;
  architecture: string | null;
  variant: string | null;
  os: string | null;
  osVersion: string | null;
  /** @format int64 */
  size: number;
  /** @format int64 */
  virtualSize: number;
  graphDriver: GraphDriverView;
  rootFS: RootFSView;
  metadata: MetadataView;
}

export interface InspectNetworkView {
  name: string | null;
  id: string | null;
  created: string | null;
  driver: string | null;
  scope: string | null;
  enableIPv4: boolean | null;
  enableIPv6: boolean | null;
  internal: boolean | null;
  attachable: boolean | null;
  ingress: boolean | null;
  inUse: boolean | null;
  configOnly: boolean | null;
  /** @default null */
  configFrom?: string | null;
  ipam?: IPAMView;
  /** @default null */
  peers?: PeerInfoView[] | null;
  /** @default null */
  options?: Record<string, string>;
  /** @default null */
  labels?: Record<string, string>;
  /** @default null */
  containers?: Record<string, NetworkContainerView>;
}

export interface InspectVolumeView {
  id: string | null;
  driver: string | null;
  mountpoint: string | null;
  createdAt: string | null;
  scope: string | null;
  inUse: boolean;
  usageData: UsageDataView;
  clusterVolume: ClusterVolumeView;
  labels: Record<string, string>;
  status: Record<string, string>;
  options: Record<string, string>;
}

export interface IPAMConfigInput {
  subnet: string | null;
  ipRange: string | null;
  gateway: string | null;
}

export interface IPAMConfigView {
  subnet: string | null;
  gateway: string | null;
  /** @default null */
  ipRange?: string | null;
}

/** @default null */
export type IPAMInput = {
  driver: string | null;
  /** @default null */
  config?: IPAMConfigInput[] | null;
  /** @default null */
  options?: Record<string, string>;
};

/** @default null */
export type IPAMView = {
  driver: string | null;
  /** @default null */
  config?: IPAMConfigView[] | null;
  /** @default null */
  options?: Record<string, string>;
};

export type JSONErrorReply = {
  /** @format int64 */
  code?: number | null;
  message?: string | null;
};

export type JSONProgressReply = {
  /** @format int64 */
  current?: number | null;
  /** @format int64 */
  total?: number | null;
  /** @format int64 */
  start?: number | null;
  units?: string | null;
};

export type LogConfig = {
  type?: string | null;
  config?: Record<string, string>;
};

export interface LoginRequest {
  email: string | null;
  password: string | null;
}

export interface LoginResponse {
  accessToken: string | null;
  permissions: AppPermission[] | null;
}

export interface ManifestView {
  id: string | null;
  descriptor: DescriptorView;
  available: boolean;
  size: SizeView;
  kind: string | null;
  imageData: ImageDataView;
  attestationData: AttestationDataView;
}

export interface MapFieldPortBinding {
  key?: string | null;
  value?: PortBinding[] | null;
}

export interface MapFieldPortBindingView {
  key: string | null;
  value: PortBindingView[] | null;
}

export type MetadataView = {
  lastTagTime: string | null;
};

export interface Mount {
  target?: string | null;
  source?: string | null;
  type?: string | null;
  readOnly?: boolean | null;
  consistency?: string | null;
  bindOptions?: BindOptions;
  volumeOptions?: VolumeOptions;
}

export interface MountPoint {
  type?: string | null;
  name?: string | null;
  source?: string | null;
  destination?: string | null;
  driver?: string | null;
  mode?: string | null;
  rw?: boolean | null;
  propagation?: string | null;
}

export interface NetworkContainerView {
  name: string | null;
  endpointId: string | null;
  macAddress: string | null;
  iPv4Address: string | null;
  iPv6Address: string | null;
}

export type NetworkSettingsView = {
  bridge: string | null;
  sandboxID: string | null;
  hairpinMode: boolean | null;
  linkLocalIPv6Address: string | null;
  /** @format int64 */
  linkLocalIPv6PrefixLen: number | null;
  ports: MapFieldPortBindingView[] | null;
  sandboxKey: string | null;
  secondaryIPAddresses: Address[] | null;
  endpointID: string | null;
  gateway: string | null;
  globalIPv6Address: string | null;
  /** @format int64 */
  globalIPv6PrefixLen: number | null;
  ipAddress: string | null;
  /** @format int64 */
  ipPrefixLen: number | null;
  iPv6Gateway: string | null;
  macAddress: string | null;
  networks: Record<string, EndpointSettingsView>;
};

export interface NetworksView {
  networks: NetworkView[] | null;
}

export interface NetworkView {
  name: string | null;
  id: string | null;
  created: string | null;
  driver: string | null;
  scope: string | null;
  enableIPv4: boolean | null;
  enableIPv6: boolean | null;
  internal: boolean | null;
  attachable: boolean | null;
  ingress: boolean | null;
  inUse: boolean | null;
  configOnly: boolean | null;
  /** @default null */
  configFrom?: string | null;
  ipam?: IPAMView;
  /** @default null */
  options?: Record<string, string>;
  /** @default null */
  labels?: Record<string, string>;
}

export type PackageVersionContainerMetadata = {
  tags?: string[] | null;
};

export type PackageVersionMetadata = {
  container?: PackageVersionContainerMetadata;
} | null;

export interface PatchRegistryInput {
  /** @format uuid */
  id: string;
  name: string | null;
  url: string | null;
  discriminator: RegistryDiscriminator;
  configuration: RegistryConfigurationBase;
}

export interface PeerInfoView {
  name: string | null;
  ip: string | null;
}

export type PlatformDescriptorView = {
  architecture: string | null;
  os: string | null;
  osVersion: string | null;
  osFeatures: string[] | null;
  variant: string | null;
};

export enum PlatformStatus {
  Offline = 'Offline',
  Online = 'Online',
}

export interface PlatformStatView {
  /** @format double */
  memoryUsage?: number;
  /** @format double */
  cpuUsage?: number;
  /** @format int64 */
  created?: number;
  /** @format double */
  rxBytes?: number;
  /** @format double */
  txBytes?: number;
}

export interface PlatformsView {
  platforms: PlatformView2[] | null;
}

/** @default null */
export type PlatformView = {
  /** @format uuid */
  id: string;
  name: string | null;
  address: string | null;
  status: PlatformStatus;
  daemonId: string | null;
  /** @format int32 */
  networksCount: number;
  /** @format int32 */
  volumesCount: number;
  /** @format int64 */
  containers: number;
  /** @format int64 */
  containersRunning: number;
  /** @format int64 */
  containersPaused: number;
  /** @format int64 */
  containersStopped: number;
  /** @format int64 */
  images: number;
  driver: string | null;
  operatingSystem: string | null;
  osVersion: string | null;
  osType: string | null;
  architecture: string | null;
  /** @format int64 */
  ncpu: number;
  /** @format int64 */
  memTotal: number;
  serverVersion: string | null;
  agentVersion: string | null;
  swarmInfo: SwarmInfoView;
  stats: PlatformStatView[] | null;
};

export interface PlatformView2 {
  /** @format uuid */
  id: string;
  name: string | null;
  address: string | null;
  status: PlatformStatus;
  daemonId: string | null;
  /** @format int32 */
  networksCount: number;
  /** @format int32 */
  volumesCount: number;
  /** @format int64 */
  containers: number;
  /** @format int64 */
  containersRunning: number;
  /** @format int64 */
  containersPaused: number;
  /** @format int64 */
  containersStopped: number;
  /** @format int64 */
  images: number;
  driver: string | null;
  operatingSystem: string | null;
  osVersion: string | null;
  osType: string | null;
  architecture: string | null;
  /** @format int64 */
  ncpu: number;
  /** @format int64 */
  memTotal: number;
  serverVersion: string | null;
  agentVersion: string | null;
  swarmInfo: SwarmInfoView;
  stats: PlatformStatView[] | null;
}

export interface PortBinding {
  hostIP?: string | null;
  hostPort?: string | null;
}

export interface PortBindingView {
  hostIP: string | null;
  hostPort: string | null;
}

export interface PortView {
  ip?: string | null;
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

export interface PublishStatusView {
  nodeID: string | null;
  state: string | null;
  publishContext: Record<string, string>;
}

export interface PullImageReply {
  stream?: string | null;
  status?: string | null;
  progressMessage?: string | null;
  id?: string | null;
  from?: string | null;
  errorMessage?: string | null;
  progress?: JSONProgressReply;
  error?: JSONErrorReply;
}

export interface PullImageRequest {
  /** @format uuid */
  platformId: string;
  registryName: string | null;
  repositoryName: string | null;
  imageTag: string | null;
}

export interface PutPlatformRequest {
  /** @format uuid */
  id: string | null;
  name: string | null;
  address: string | null;
}

export interface RefreshTokenResponse {
  accessToken: string | null;
}

export interface RegistriesView {
  registries: RegistryView[] | null;
}

export type RegistryConfigurationBase = BaseRegistryConfigurationBase &
  (
    | BaseRegistryConfigurationBaseTypeMapping<'AWS', RegistryConfigurationBaseAWSRegistry>
    | BaseRegistryConfigurationBaseTypeMapping<'Azure', RegistryConfigurationBaseAzureRegistry>
    | BaseRegistryConfigurationBaseTypeMapping<'Gitlab', RegistryConfigurationBaseGitlabRegistry>
    | BaseRegistryConfigurationBaseTypeMapping<'DockerHub', RegistryConfigurationBaseDockerHubRegistry>
    | BaseRegistryConfigurationBaseTypeMapping<'GitHub', RegistryConfigurationBaseGitHubRegistry>
  );

export interface RegistryConfigurationBaseAWSRegistry {
  $type?: 'AWS';
  registryUrl?: string | null;
  authenticationRequired: boolean;
  accessKey: string | null;
  secretAccessKey: string | null;
  region: string | null;
}

export interface RegistryConfigurationBaseAzureRegistry {
  $type?: 'Azure';
  registryUrl?: string | null;
  userName: string | null;
  password: string | null;
}

export interface RegistryConfigurationBaseDockerHubRegistry {
  $type?: 'DockerHub';
  registryUrl?: string | null;
  /** @default null */
  userName?: string | null;
  /** @default null */
  pat?: string | null;
}

export interface RegistryConfigurationBaseGitHubRegistry {
  $type?: 'GitHub';
  registryUrl?: string | null;
  name: string | null;
  type: 'Organization' | 'User' | null;
  pat: string | null;
}

export interface RegistryConfigurationBaseGitlabRegistry {
  $type?: 'Gitlab';
  registryUrl?: string | null;
  userName: string | null;
  pat: string | null;
  instanceUrl: string | null;
}

export enum RegistryDiscriminator {
  DockerHub = 'DockerHub',
  Azure = 'Azure',
  AWS = 'AWS',
  Gitlab = 'Gitlab',
  GitHub = 'GitHub',
}

export interface RegistryView {
  /** @format uuid */
  id: string;
  name: string | null;
  url: string | null;
  discriminator: RegistryDiscriminator;
  /** @format date-time */
  created: string;
  configuration: RegistryConfigurationBase;
  isDefault?: boolean;
}

export type RestartPolicy = {
  name?: string | null;
  /** @format int32 */
  maximumRetryCount?: number | null;
};

export type RootFSView = {
  type: string | null;
  layers: string[] | null;
};

export type SizeView = {
  /** @format int64 */
  total: number | null;
  /** @format int64 */
  content: number | null;
  /** @format int64 */
  unpacked: number | null;
};

export interface StreamLogsRequest {
  containerId: string | null;
}

export type SwarmInfoView = {
  /** @format uuid */
  id: string;
  nodeID: string | null;
  nodeAddr: string | null;
  localNodeState: string | null;
  controlAvailable: boolean;
  error: string | null;
  /** @format int64 */
  nodes: number;
  /** @format int64 */
  managers: number;
  remoteManagers: SwarmPeerView[] | null;
};

export interface SwarmPeerView {
  nodeID?: string | null;
  addr?: string | null;
}

export enum TagStatus {
  Active = 'Active',
  Inactive = 'Inactive',
}

export interface TopologyEntryView {
  labels: Record<string, string>;
}

export interface Ulimits {
  name?: string | null;
  /** @format int64 */
  soft?: number | null;
  /** @format int64 */
  hard?: number | null;
}

export type UsageDataView = {
  /** @format int64 */
  size: number | null;
  /** @format int64 */
  refCount: number | null;
};

export type VolumeAccessModeView = {
  scope: VolumeScopeType;
  sharing: VolumeSharingType;
  secrets: VolumeSecretView[] | null;
  capacityRange: VolumeCapacityRange;
  availability: string | null;
};

export type VolumeCapacityRange = {
  /** @format int64 */
  requiredBytes: number | null;
  /** @format int64 */
  limitBytes: number | null;
};

export type VolumeOptions = {
  noCopy?: boolean | null;
  labels?: Record<string, string>;
  driverConfig?: DriverConfig;
  subpath?: string | null;
};

export enum VolumeScopeType {
  Single = 'Single',
  Multi = 'Multi',
}

export interface VolumeSecretView {
  key: string | null;
  secret: string | null;
}

export enum VolumeSharingType {
  None = 'None',
  Readonly = 'Readonly',
  Onewriter = 'Onewriter',
  All = 'All',
}

export type VolumeSpecView = {
  group: string | null;
  accessMode: VolumeAccessModeView;
};

export interface VolumesView {
  volumes: VolumeView[] | null;
}

export interface VolumeView {
  id: string | null;
  driver: string | null;
  mountpoint: string | null;
  createdAt: string | null;
  scope: string | null;
  inUse: boolean;
  usageData: UsageDataView;
  labels: Record<string, string>;
  status: Record<string, string>;
  options: Record<string, string>;
}

export type VolumVersionView = {
  /** @format int64 */
  index: number | null;
};

type BaseIImageRepository = object;

type BaseIImageRepositoryTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BaseRegistryConfigurationBase = object | null;

type BaseRegistryConfigurationBaseTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

export type QueryParamsType = Record<string | number, any>;
export type ResponseFormat = keyof Omit<Body, 'body' | 'bodyUsed'>;

export interface FullRequestParams extends Omit<RequestInit, 'body'> {
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

export type RequestParams = Omit<FullRequestParams, 'body' | 'method' | 'query' | 'path'>;

export interface ApiConfig<SecurityDataType = unknown> {
  baseUrl?: string;
  baseApiParams?: Omit<RequestParams, 'baseUrl' | 'cancelToken' | 'signal'>;
  securityWorker?: (securityData: SecurityDataType | null) => Promise<RequestParams | void> | RequestParams | void;
  customFetch?: typeof fetch;
}

export interface HttpResponse<D extends unknown, E extends unknown = unknown> extends Response {
  data: D;
  error: E;
}

type CancelToken = Symbol | string | number;

export enum ContentType {
  Json = 'application/json',
  FormData = 'multipart/form-data',
  UrlEncoded = 'application/x-www-form-urlencoded',
  Text = 'text/plain',
}

export class HttpClient<SecurityDataType = unknown> {
  public baseUrl: string = '';
  private securityData: SecurityDataType | null = null;
  private securityWorker?: ApiConfig<SecurityDataType>['securityWorker'];
  private abortControllers = new Map<CancelToken, AbortController>();
  private customFetch = (...fetchParams: Parameters<typeof fetch>) => fetch(...fetchParams);

  private baseApiParams: RequestParams = {
    credentials: 'same-origin',
    headers: {},
    redirect: 'follow',
    referrerPolicy: 'no-referrer',
  };

  constructor(apiConfig: ApiConfig<SecurityDataType> = {}) {
    Object.assign(this, apiConfig);
  }

  public setSecurityData = (data: SecurityDataType | null) => {
    this.securityData = data;
  };

  protected encodeQueryParam(key: string, value: any) {
    const encodedKey = encodeURIComponent(key);
    return `${encodedKey}=${encodeURIComponent(typeof value === 'number' ? value : `${value}`)}`;
  }

  protected addQueryParam(query: QueryParamsType, key: string) {
    return this.encodeQueryParam(key, query[key]);
  }

  protected addArrayQueryParam(query: QueryParamsType, key: string) {
    const value = query[key];
    return value.map((v: any) => this.encodeQueryParam(key, v)).join('&');
  }

  protected toQueryString(rawQuery?: QueryParamsType): string {
    const query = rawQuery || {};
    const keys = Object.keys(query).filter((key) => 'undefined' !== typeof query[key]);
    return keys
      .map((key) => (Array.isArray(query[key]) ? this.addArrayQueryParam(query, key) : this.addQueryParam(query, key)))
      .join('&');
  }

  protected addQueryParams(rawQuery?: QueryParamsType): string {
    const queryString = this.toQueryString(rawQuery);
    return queryString ? `?${queryString}` : '';
  }

  private contentFormatters: Record<ContentType, (input: any) => any> = {
    [ContentType.Json]: (input: any) =>
      input !== null && (typeof input === 'object' || typeof input === 'string') ? JSON.stringify(input) : input,
    [ContentType.Text]: (input: any) => (input !== null && typeof input !== 'string' ? JSON.stringify(input) : input),
    [ContentType.FormData]: (input: any) =>
      Object.keys(input || {}).reduce((formData, key) => {
        const property = input[key];
        formData.append(
          key,
          property instanceof Blob
            ? property
            : typeof property === 'object' && property !== null
              ? JSON.stringify(property)
              : `${property}`,
        );
        return formData;
      }, new FormData()),
    [ContentType.UrlEncoded]: (input: any) => this.toQueryString(input),
  };

  protected mergeRequestParams(params1: RequestParams, params2?: RequestParams): RequestParams {
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

  protected createAbortSignal = (cancelToken: CancelToken): AbortSignal | undefined => {
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
      ((typeof secure === 'boolean' ? secure : this.baseApiParams.secure) &&
        this.securityWorker &&
        (await this.securityWorker(this.securityData))) ||
      {};
    const requestParams = this.mergeRequestParams(params, secureParams);
    const queryString = query && this.toQueryString(query);
    const payloadFormatter = this.contentFormatters[type || ContentType.Json];
    const responseFormat = format || requestParams.format;

    return this.customFetch(`${baseUrl || this.baseUrl || ''}${path}${queryString ? `?${queryString}` : ''}`, {
      ...requestParams,
      headers: {
        ...(requestParams.headers || {}),
        ...(type && type !== ContentType.FormData ? { 'Content-Type': type } : {}),
      },
      signal: (cancelToken ? this.createAbortSignal(cancelToken) : requestParams.signal) || null,
      body: typeof body === 'undefined' || body === null ? null : payloadFormatter(body),
    }).then(async (response) => {
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
export class Api<SecurityDataType extends unknown> extends HttpClient<SecurityDataType> {
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
      this.request<RefreshTokenResponse, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/authentication/refresh`,
        method: 'GET',
        format: 'json',
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
      this.request<LoginResponse, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/authentication/login`,
        method: 'POST',
        body: data,
        type: ContentType.Json,
        format: 'json',
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
        method: 'POST',
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
     * @response `200` `ContainerInfoView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    containersGetById: (id: string, params: RequestParams = {}) =>
      this.request<ContainerInfoView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/${id}`,
        method: 'GET',
        secure: true,
        format: 'json',
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
      this.request<ContainerStatsView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/${id}/stats`,
        method: 'GET',
        secure: true,
        format: 'json',
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
      this.request<ContainerInspectView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/${id}/inspect`,
        method: 'GET',
        secure: true,
        format: 'json',
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
     * @response `200` `(ContainerLogReply)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    containersStreamLogs: (data: StreamLogsRequest, params: RequestParams = {}) =>
      this.request<ContainerLogReply[], HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/stream-logs`,
        method: 'POST',
        body: data,
        secure: true,
        type: ContentType.Json,
        format: 'json',
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
        method: 'PATCH',
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
        method: 'PATCH',
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
        method: 'PATCH',
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
        method: 'PATCH',
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
        method: 'PATCH',
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
    containersDeleteContainers: (data: DeleteContainersRequest, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/delete`,
        method: 'DELETE',
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
      this.request<PlatformsView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/platforms`,
        method: 'GET',
        secure: true,
        format: 'json',
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name PlatformsPut
     * @summary Create or update a platform
     * @request PUT:/api/v1/platforms
     * @secure
     * @response `200` `PlatformView2` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    platformsPut: (data: PutPlatformRequest, params: RequestParams = {}) =>
      this.request<PlatformView2, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/platforms`,
        method: 'PUT',
        body: data,
        secure: true,
        type: ContentType.Json,
        format: 'json',
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
        method: 'DELETE',
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
      this.request<PlatformView2, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/platforms/${id}`,
        method: 'GET',
        secure: true,
        format: 'json',
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
      this.request<PlatformView2, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/platforms/${id}/info`,
        method: 'GET',
        secure: true,
        format: 'json',
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
     * @response `200` `ContainersInfoView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    platformsListContainers: (id: string, params: RequestParams = {}) =>
      this.request<ContainersInfoView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/platforms/${id}/containers`,
        method: 'GET',
        secure: true,
        format: 'json',
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
      this.request<RegistriesView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/registries/all`,
        method: 'GET',
        secure: true,
        format: 'json',
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
     * @response `500` `ProblemDetails` Internal Server Error
     */
    registriesGetById: (id: string, params: RequestParams = {}) =>
      this.request<RegistryView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/registries/${id}`,
        method: 'GET',
        secure: true,
        format: 'json',
        ...params,
      }),

    /**
     * @description A discriminator should be provided in the request, this discriminator is based on RegistryDiscriminator enum
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
    registriesCreate: (data: CreateRegistryInput, params: RequestParams = {}) =>
      this.request<RegistryView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/registries`,
        method: 'POST',
        body: data,
        secure: true,
        type: ContentType.Json,
        format: 'json',
        ...params,
      }),

    /**
     * @description A discriminator should be provided in the request, this discriminator is based on RegistryDiscriminator enum
     *
     * @tags Registries
     * @name RegistriesPatch
     * @summary Patch a registry
     * @request PATCH:/api/v1/registries
     * @secure
     * @response `200` `RegistryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    registriesPatch: (data: PatchRegistryInput, params: RequestParams = {}) =>
      this.request<RegistryView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/registries`,
        method: 'PATCH',
        body: data,
        secure: true,
        type: ContentType.Json,
        format: 'json',
        ...params,
      }),

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
     * @response `500` `ProblemDetails` Internal Server Error
     */
    registriesDelete: (data: DeleteRegistriesInput, params: RequestParams = {}) =>
      this.request<RegistriesView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/registries`,
        method: 'DELETE',
        body: data,
        secure: true,
        type: ContentType.Json,
        format: 'json',
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
        method: 'GET',
        secure: true,
        format: 'json',
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
    imagesGetExternalRepositories: (registryName: string, params: RequestParams = {}) =>
      this.request<IImageRepository[], HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/images/${registryName}/repositories`,
        method: 'GET',
        secure: true,
        format: 'json',
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
     * @response `200` `(GhcrPackageVersion)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    imagesGetGhcrPackageVersions: (registryName: string, packageName: string, params: RequestParams = {}) =>
      this.request<GhcrPackageVersion[], HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/images/ghcr/${registryName}/${packageName}/versions`,
        method: 'GET',
        secure: true,
        format: 'json',
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
     * @response `200` `(DockerHubRepository)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    imagesGetDockerHubRepositories: (registryName: string, params: RequestParams = {}) =>
      this.request<DockerHubRepository[], HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/images/dockerhub/${registryName}/repositories`,
        method: 'GET',
        secure: true,
        format: 'json',
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
    imagesGetDockerHubRepositoryTags: (registryName: string, repositoryName: string, params: RequestParams = {}) =>
      this.request<DockerHubTagView[], HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/images/dockerhub/${registryName}/${repositoryName}/tags`,
        method: 'GET',
        secure: true,
        format: 'json',
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
      this.request<DockerHubImageModel[], HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/images/dockerhub`,
        method: 'GET',
        query: query,
        secure: true,
        format: 'json',
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
     * @response `200` `InspectImageView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    imagesInspect: (platformId: string, imageId: string, params: RequestParams = {}) =>
      this.request<InspectImageView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/images/${platformId}/${imageId}`,
        method: 'GET',
        secure: true,
        format: 'json',
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
     * @response `200` `(PullImageReply)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `500` `ProblemDetails` Internal Server Error
     */
    imagesPullImage: (data: PullImageRequest, params: RequestParams = {}) =>
      this.request<PullImageReply[], HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/images/pull`,
        method: 'POST',
        body: data,
        secure: true,
        type: ContentType.Json,
        format: 'json',
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
     * @response `200` `DeleteImagesReply` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `500` `ProblemDetails` Internal Server Error
     */
    imagesDelete: (data: DeleteImagesRequest, params: RequestParams = {}) =>
      this.request<DeleteImagesReply, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/images`,
        method: 'DELETE',
        body: data,
        secure: true,
        type: ContentType.Json,
        format: 'json',
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
      this.request<NetworksView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/networks/${id}`,
        method: 'GET',
        query: query,
        secure: true,
        format: 'json',
        ...params,
      }),

    /**
     * No description
     *
     * @tags Networks
     * @name NetworksInspect
     * @summary Inspect a network
     * @request GET:/api/v1/networks/{platformId}/{networkId}
     * @secure
     * @response `200` `InspectNetworkView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `500` `ProblemDetails` Internal Server Error
     */
    networksInspect: (platformId: string, networkId: string, params: RequestParams = {}) =>
      this.request<InspectNetworkView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/networks/${platformId}/${networkId}`,
        method: 'GET',
        secure: true,
        format: 'json',
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
      this.request<CreateNetworkView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/networks`,
        method: 'POST',
        body: data,
        secure: true,
        type: ContentType.Json,
        format: 'json',
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
        method: 'DELETE',
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
        method: 'GET',
        query: query,
        secure: true,
        format: 'json',
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
     * @response `200` `InspectVolumeView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `500` `ProblemDetails` Internal Server Error
     */
    volumesInspect: (platformId: string, name: string, params: RequestParams = {}) =>
      this.request<InspectVolumeView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/volumes/${platformId}/${name}`,
        method: 'GET',
        secure: true,
        format: 'json',
        ...params,
      }),

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
        method: 'DELETE',
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
     * @response `200` `VolumeView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `500` `ProblemDetails` Internal Server Error
     */
    volumesCreate: (data: CreateVolumeInput, params: RequestParams = {}) =>
      this.request<VolumeView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/volumes`,
        method: 'POST',
        body: data,
        secure: true,
        type: ContentType.Json,
        format: 'json',
        ...params,
      }),
  };
}
