import { DockerResourceType, ResourceType } from '@/api/types';
import { RequiredComponents, RequiredDockerInfoComponents, RequiredFormComponents } from '@/pages/types';
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

export const ResourceComponents: {
  [key in ResourceType]: RequiredComponents;
} = {
  Image: ImageComponents,
  Volume: VolumeComponents,
  Network: NetworkComponents,
  Container: ContainerComponents,

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
};

export const ResourceFormComponents: {
  [key in ResourceType]: RequiredFormComponents | undefined;
} = {
  Volume: VolumeFormComponents,
  Network: NetworkFormComponents,
  Container: undefined,
  Image: undefined,
  Platform: undefined,

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
};

export const DockerResourceInfoComponents: {
  [key in DockerResourceType]: RequiredDockerInfoComponents;
} = {
  Image: ImageInfoComponents,
  Volume: VolumeInfoComponents,
  Network: NetworkInfoComponents,
  Container: ContainerInfoComponents,
};
