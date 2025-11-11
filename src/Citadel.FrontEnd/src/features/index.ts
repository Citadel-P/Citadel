import { DockerResourceType, ResourceType } from '@/api/types';
import { RequiredComponents, RequiredDockerInfoComponents } from '@/pages/types';
import { ImageComponents } from './docker-resources/images';
import { VolumeComponents } from './docker-resources/volumes';
import { NetworkComponents } from './docker-resources/networks';
import { ContainerComponents } from './docker-resources/containers';
import { NetworkInfoComponents } from './docker-resources/networks/network-info';
import { VolumeInfoComponents } from './docker-resources/volumes/volume-info';
import { ImageInfoComponents } from './docker-resources/images/image-info';
import { ContainerInfoComponents } from './docker-resources/containers/container-info';
import { RegistryComponents } from './registries';
import { PlatformComponents } from './platforms';

export const ResourceComponents: {
  [key in ResourceType]: RequiredComponents;
} = {
  Image: ImageComponents,
  Volume: VolumeComponents,
  Network: NetworkComponents,
  Container: ContainerComponents,

  Platform: PlatformComponents,
  Registry: RegistryComponents,
};

export const DockerResourceInfoComponents: {
  [key in DockerResourceType]: RequiredDockerInfoComponents;
} = {
  Image: ImageInfoComponents,
  Volume: VolumeInfoComponents,
  Network: NetworkInfoComponents,
  Container: ContainerInfoComponents,
};
