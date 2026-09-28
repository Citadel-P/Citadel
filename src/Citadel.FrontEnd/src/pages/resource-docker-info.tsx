import { Suspense } from 'react';
import { ResourceSkeleton } from './resource-skeleton';
import { useResourceParamType } from '@/lib/hooks';
import { DockerResourceType } from '@/api/types';
import NotFound from './not-found';
import { DockerResourceInfoPages } from '@/features';

const ResourceDockerInfoPage = () => {
  const { type } = useResourceParamType()!;

  const Page = DockerResourceInfoPages[type as DockerResourceType];
  if (!Page) return <NotFound />;

  return (
    <Suspense fallback={<ResourceSkeleton variant={'detail'} />}>
      <Page key={type} type={type as DockerResourceType} showHeaderId={false} />
    </Suspense>
  );
};

export default ResourceDockerInfoPage;
