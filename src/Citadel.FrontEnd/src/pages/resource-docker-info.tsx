import { useResourceParamType } from '@/lib/hooks';
import { DockerResourceType } from '@/api/types';
import NotFound from './not-found';
import { DockerResourceInfoComponents } from '@/features';
import { ResourceInfoView } from './resource-info';

const ResourceDockerInfoPage = () => {
  const { type } = useResourceParamType()!;

  const Components = DockerResourceInfoComponents[type as DockerResourceType];
  if (!Components) return <NotFound />;

  return <ResourceInfoView key={type} Components={Components} type={type as DockerResourceType} />;
};

export default ResourceDockerInfoPage;
