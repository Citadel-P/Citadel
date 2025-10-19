import { DockerResourceType } from '@/api/types';
import { VolumeComponents } from '@/features/docker-resources/volumes';

export interface RequiredDockerComponents {
  DeleteDialog: React.FC;
  Icon: React.ReactElement;
  ActionBar: React.FC<{ items: any[] }>;
  Table: React.FC<{ items: any[]; isLoading: boolean }>;
}

export const DockerResourceComponents: {
  [key in DockerResourceType]: RequiredDockerComponents;
} = {
  Volume: VolumeComponents,
};
