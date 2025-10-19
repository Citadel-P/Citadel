import { DockerResourceType } from '@/api/types';
import { ContainerComponents } from '@/features/docker-resources/containers';
import { NetworkComponents } from '@/features/docker-resources/networks';
import { VolumeComponents } from '@/features/docker-resources/volumes';

export interface ResourceDataHookResult<T> {
  items: T[];
  isLoading: boolean;
}

export interface RequiredDockerComponents<T = any> {
  DeleteDialog: React.FC;
  Icon: React.ReactElement;
  ActionBar: React.FC<{ items: any[] }>;
  Table: React.FC<{ items: any[]; isLoading: boolean }>;
  useData: (platformId: string) => ResourceDataHookResult<T>;
  filterItems?: (items: T[], search: string) => T[];
}

export const DockerResourceComponents: {
  [key in DockerResourceType]: RequiredDockerComponents;
} = {
  Volume: VolumeComponents,
  Network: NetworkComponents,
  Container: ContainerComponents
};
