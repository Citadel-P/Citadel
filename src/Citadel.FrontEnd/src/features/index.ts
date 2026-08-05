import { DockerResourceType, ResourceType, SwarmResourceType } from '@/api/types';
import {
  RequiredComponents,
  RequiredDockerInfoComponents,
  RequiredFormComponents,
  RequiredSwarmInfoComponents,
  RegularResourceComponents,
} from '@/pages/types';
import { ImageComponents } from './docker-resources/images';
import { VolumeComponents, VolumeFormComponents } from './docker-resources/volumes';
import { NetworkComponents, NetworkFormComponents } from './docker-resources/networks';
import { ContainerComponents } from './docker-resources/containers';
import { NetworkInfoComponents } from './docker-resources/networks/network-info';
import { VolumeInfoComponents } from './docker-resources/volumes/volume-info';
import { ImageInfoComponents } from './docker-resources/images/image-info';
import { ContainerInfoComponents } from './docker-resources/containers/container-info';
import { RegistryComponents } from './registries';
import { PlatformComponents } from './platforms';
import { DeploymentComponents } from './deployments';
import { RegistryFormComponents } from './registries/form';
import { PlatformFormComponents } from './platforms/forms';
import { DeploymentFormComponents } from './deployments/form';
import { ActivityComponents } from './activities';
import { AlertRuleComponents } from './alerters/alert-rules';
import { AlertRuleFormComponents } from './alerters/alert-rules/form';
import { AlertEventComponents } from './alerters/alert-events';
import { GitRepoComponents } from './git-repos';
import { GitRepoFormComponents } from './git-repos/form';
import { AccessComponents } from './access';
import { UserFormComponents } from './access/users/form';
import { TeamFormComponents } from './access/teams/form';
import { StackFormComponents } from './stacks/form';
import { StackComponents } from './stacks';
import { BindingComponents } from './bindings';
import { TagComponents } from './tags';
import { OidcProviderComponents } from './oidc-providers';
import { OidcProviderFormComponents } from './oidc-providers/form';
import { AutomationActionComponents } from './automation-actions';
import { AutomationActionFormComponents } from './automation-actions/form';
import { BuildComponents } from './builds';
import { BuildFormComponents } from './builds/form';
import { BuildPoolComponents } from './build-pools';
import { BuildPoolFormComponents } from './build-pools/form';
import { BackupRepositoryComponents } from './backup-repositories';
import { BackupRepositoryFormComponents } from './backup-repositories/form';
import { BackupPolicyComponents } from './backup-policies';
import { BackupPolicyFormComponents } from './backup-policies/form';
import { NodeComponents } from './swarm-resources/nodes';
import { NodeInfoComponents } from './swarm-resources/nodes/node-info';
import { ServiceComponents } from './swarm-resources/services';
import { ServiceInfoComponents } from './swarm-resources/services/service-info';
import { TaskComponents } from './swarm-resources/tasks';
import { TaskInfoComponents } from './swarm-resources/tasks/task-info';
import { SecretComponents } from './swarm-resources/secrets';
import { SecretInfoComponents } from './swarm-resources/secrets/secret-info';
import { ConfigComponents } from './swarm-resources/configs';
import { ConfigInfoComponents } from './swarm-resources/configs/config-info';
import { SecretFormComponents } from './swarm-resources/secrets/form';
import { ConfigFormComponents } from './swarm-resources/configs/form';

export const SwarmResourceComponents: {
  [key in SwarmResourceType]: RegularResourceComponents;
} = {
  Node: NodeComponents,
  Service: ServiceComponents,
  Task: TaskComponents,
  Secret: SecretComponents,
  Config: ConfigComponents,
};

export const ResourceComponents: {
  [key in ResourceType]: RequiredComponents;
} = {
  Image: ImageComponents,
  Volume: VolumeComponents,
  Network: NetworkComponents,
  Container: ContainerComponents,
  ...SwarmResourceComponents,

  Platform: PlatformComponents,
  Registry: RegistryComponents,
  Deployment: DeploymentComponents,
  Stack: StackComponents,
  Activity: ActivityComponents,
  AlertRule: AlertRuleComponents,
  Alert: AlertEventComponents,
  GitRepository: GitRepoComponents,
  Access: AccessComponents,
  User: AccessComponents,
  Team: AccessComponents,
  Role: AccessComponents,
  Binding: BindingComponents,
  Tag: TagComponents,
  OidcProvider: OidcProviderComponents,
  Automation: AutomationActionComponents,
  AutomationAction: AutomationActionComponents,
  Build: BuildComponents,
  BuildAgentPool: BuildPoolComponents,
  BackupRepository: BackupRepositoryComponents,
  BackupPolicy: BackupPolicyComponents,
};

export const ResourceFormComponents: {
  [key in ResourceType]: RequiredFormComponents | undefined;
} = {
  Volume: VolumeFormComponents,
  Network: NetworkFormComponents,
  Container: undefined,
  Image: undefined,
  Node: undefined,
  Service: undefined,
  Task: undefined,
  Secret: SecretFormComponents,
  Config: ConfigFormComponents,
  Platform: PlatformFormComponents,

  Registry: RegistryFormComponents,
  Deployment: DeploymentFormComponents,
  AlertRule: AlertRuleFormComponents,
  Activity: undefined,
  Alert: undefined,
  AlertChannel: undefined,
  Stack: StackFormComponents,
  GitRepository: GitRepoFormComponents,
  GitAccount: undefined,
  Access: undefined,
  User: UserFormComponents,
  Team: TeamFormComponents,
  Role: undefined,
  Binding: undefined,
  Tag: undefined,
  OidcProvider: OidcProviderFormComponents,
  Automation: undefined,
  AutomationAction: AutomationActionFormComponents,
  Build: BuildFormComponents,
  BuildAgentPool: BuildPoolFormComponents,
  BackupRepository: BackupRepositoryFormComponents,
  BackupPolicy: BackupPolicyFormComponents,
};

export const DockerResourceInfoComponents: {
  [key in DockerResourceType]: RequiredDockerInfoComponents;
} = {
  Image: ImageInfoComponents,
  Volume: VolumeInfoComponents,
  Network: NetworkInfoComponents,
  Container: ContainerInfoComponents,
};

export const SwarmResourceInfoComponents: {
  [key in SwarmResourceType]: RequiredSwarmInfoComponents;
} = {
  Node: NodeInfoComponents,
  Service: ServiceInfoComponents,
  Task: TaskInfoComponents,
  Secret: SecretInfoComponents,
  Config: ConfigInfoComponents,
};
