import { DockerResourceType } from '@/api/types';
import { RequiredDockerComponents, RequiredDockerInfoComponents } from '@/pages/types';
import { ImageComponents } from './images';
import { VolumeComponents } from './volumes';
import { NetworkComponents } from './networks';
import { ContainerComponents } from './containers';
import { NetworkInfoComponents } from './networks/network-info';
import { VolumeInfoComponents } from './volumes/volume-info';
import { ImageInfoComponents } from './images/image-info';
import { ContainerInfoComponents } from './containers/container-info';

export const DockerResourceComponents: {
  [key in DockerResourceType]: RequiredDockerComponents;
} = {
  Image: ImageComponents,
  Volume: VolumeComponents,
  Network: NetworkComponents,
  Container: ContainerComponents,
};

export const DockerResourceInfoComponents: {
  [key in DockerResourceType]: RequiredDockerInfoComponents;
} = {
  Image: ImageInfoComponents,
  Volume: VolumeInfoComponents,
  Network: NetworkInfoComponents,
  Container: ContainerInfoComponents,
};
