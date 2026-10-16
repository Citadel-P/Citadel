import { canShowCachedResource } from '@/lib/request-error';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { PlatformType } from '@/api/generated/api.types';
import { SwarmResourcePathMap } from '@/api/types';
import { ResourceSkeleton } from './resource-skeleton';
import { useAppContext } from '@/lib/context/app-context';
import { useParams } from 'react-router';
import ResourceDockerInfoPage from './resource-docker-info';
import ResourcePage from './resource';
import ResourceSwarmPage from './resource-swarm';
import TaskSheet from '@/components/custom/task-sheet';

const PlatformResourcePage = () => {
  return (
    <>
      <PlatformResourceContent />
      <TaskSheet type="Image" />
    </>
  );
};

const PlatformResourceContent = () => {
  const { type = '', resourceId } = useParams<{ type: string; resourceId?: string }>();
  const { currentPlatform, isLoading, platformRead } = useAppContext();

  if (isLoading && !currentPlatform) return <ResourceSkeleton variant={resourceId ? 'detail' : 'list'} />;

  if (!currentPlatform || (platformRead?.error && !canShowCachedResource(platformRead.error)))
    return <ResourceReadError {...platformRead} error={platformRead?.error ?? { status: 404 }} />;

  if (currentPlatform?.type === PlatformType.DockerSwarm && type in SwarmResourcePathMap) {
    return <ResourceSwarmPage />;
  }

  return resourceId ? <ResourceDockerInfoPage /> : <ResourcePage showTaskSheet={type !== 'images'} />;
};

export default PlatformResourcePage;
