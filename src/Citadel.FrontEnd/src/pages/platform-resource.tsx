import { PlatformType } from '@/api/generated/api.types';
import { SwarmResourcePathMap } from '@/api/types';
import Loader from '@/components/ui/loader';
import { useAppContext } from '@/lib/context/app-context';
import { useParams } from 'react-router';
import ResourceDockerInfoPage from './resource-docker-info';
import ResourcePage from './resource';
import ResourceSwarmPage from './resource-swarm';

const PlatformResourcePage = () => {
  const { type = '', resourceId } = useParams<{ type: string; resourceId?: string }>();
  const { currentPlatform, isLoading } = useAppContext();

  if (isLoading && !currentPlatform) return <Loader />;

  if (currentPlatform?.type === PlatformType.DockerSwarm && type in SwarmResourcePathMap) {
    return <ResourceSwarmPage />;
  }

  return resourceId ? <ResourceDockerInfoPage /> : <ResourcePage />;
};

export default PlatformResourcePage;
