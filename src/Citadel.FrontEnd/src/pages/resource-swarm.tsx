import { SwarmResourcePathMap } from '@/api/types';
import { SwarmResourceComponents, SwarmResourceInfoComponents } from '@/features';
import NotFound from './not-found';
import { RegularResourceView } from './regular-resource';
import { ResourceInfoView } from './resource-info';
import { useParams } from 'react-router';

const ResourceSwarmPage = () => {
  const { type = '', resourceId } = useParams<{ type: string; resourceId?: string }>();
  const swarmType = SwarmResourcePathMap[type as keyof typeof SwarmResourcePathMap];
  if (!swarmType) return <NotFound />;

  if (resourceId) {
    return (
      <ResourceInfoView
        key={`${swarmType}-${resourceId}`}
        type={swarmType}
        Components={SwarmResourceInfoComponents[swarmType]}
      />
    );
  }

  return (
    <RegularResourceView
      key={swarmType}
      type={swarmType}
      Components={SwarmResourceComponents[swarmType]}
      showTaskSheet={false}
    />
  );
};

export default ResourceSwarmPage;
