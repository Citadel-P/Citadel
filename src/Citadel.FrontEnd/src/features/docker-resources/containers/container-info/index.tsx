import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredDockerInfoComponents } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { ContainerLogs } from './container-logs';
import { useContainerInfoGroup } from '../hooks/useContainerInfoGroup';
import { DockerContainerView } from '@/api/types';
import ContainerInspect from './container-inspect';
import { ContainerStats } from './container-stats';
import { ContainerInfoActions } from './actions';
import { ContainerInfoTable } from './container-info-table';
import { ImageView } from '@/api/generated/api.types';
import { Link } from 'react-router';
import { truncate } from '@/lib/truncate';

export const ContainerInfoComponents: RequiredDockerInfoComponents<DockerContainerView> = {
  Header: {
    Indicator: ({ resource }) => {
      return <StateIndicator value={resource.state} />;
    },
    ActionButtons: ({ resource }) => {
      return <GenericActionBarButtons resource={resource} actions={Object.values(ContainerInfoActions)} />;
    },
  },
  SubHeader: ({ resource }) => {
    return <ContainerInfoTable container={resource} />;
  },
  Tabs: [
    {
      label: 'Logs',
      Content: ({ resource }) => <ContainerLogs resource={resource} />,
    },
    {
      label: 'Inspect',
      Content: ({ resource }) => <ContainerInspect resource={resource} />,
    },
    {
      label: 'Stats',
      Content: ({ resource }) => <ContainerStats resource={resource} />,
    },
  ],

  useData: (platformId: string, resourceId: string) => {
    const { containerInfo, isLoading } = useContainerInfoGroup(resourceId, platformId);
    return { resource: containerInfo, isLoading, error: null };
  },
};

export const ImageName = ({ image }: { image: ImageView | undefined }) => {
  if (image === undefined || image?.name === undefined) return <div className="text-muted">{'<none>'}</div>;
  return (
    <Link to={`/platforms/${image.platformId}/images/${image?.dockerImageId?.slice(0, 24)}`} className="table-link">
      {truncate(image?.name, 24)}
    </Link>
  );
};
