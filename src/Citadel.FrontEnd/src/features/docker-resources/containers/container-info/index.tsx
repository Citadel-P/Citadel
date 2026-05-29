import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredDockerInfoComponents } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { ContainerLogs } from './container-logs';
import { useContainerInfoGroup } from '../hooks/useContainerInfoGroup';
import { DockerContainerView } from '@/api/types';
import { ContainerInspect } from './container-inspect';
import { ContainerStats } from './container-stats';
import { ContainerInfoActions } from './actions';
import { ContainerInfoTable } from './container-info-table';
import { ContainerStateStatus, ImageView, ResourceControlState } from '@/api/generated/api.types';
import { hasCapability } from '@/lib/resource-capabilities';
import { Link } from 'react-router';
import { truncate } from '@/lib/truncate';
import { ContainerExec } from './container-exec';

export const ContainerInfoComponents: RequiredDockerInfoComponents<DockerContainerView> = {
  Header: {
    Indicator: ({ resource }) => {
      return (
        <StateIndicator
          value={resource.state}
          isProcessing={resource.controlState === ResourceControlState.Processing}
        />
      );
    },
    ActionButtons: ({ resource }) => {
      return <GenericActionBarButtons resource={resource} actions={Object.values(ContainerInfoActions)} />;
    },
  },
  SubHeader: ({ resource }) => {
    return (
      <ContainerInfoTable
        container={resource}
        displayOptions={{
          DisplayStatus: true,
          DisplayPlatformName: true,
        }}
      />
    );
  },
  Tabs: [
    {
      label: 'Logs',
      disabled: (resource: DockerContainerView) => !hasCapability(resource, 'canViewLogs'),
      Content: ({ resource }) => <ContainerLogs containerId={resource?.id} />,
    },
    {
      label: 'Inspect',
      disabled: (resource: DockerContainerView) => !hasCapability(resource, 'canInspect'),
      Content: ({ resource }) => <ContainerInspect containerId={resource?.id} />,
    },
    {
      label: 'Terminal',
      disabled: (resource: DockerContainerView) =>
        resource.state !== ContainerStateStatus.Running || !hasCapability(resource, 'canOpenTerminal'),
      Content: ({ resource }) => (
        <ContainerExec
          containerId={resource?.id}
          disabled={resource.state !== ContainerStateStatus.Running || !hasCapability(resource, 'canOpenTerminal')}
        />
      ),
    },
    {
      label: 'Stats',
      Content: ({ resource }) => <ContainerStats resource={resource} />,
    },
  ],

  useData: (platformId: string, resourceId: string) => {
    const { containerInfo, isLoading, error } = useContainerInfoGroup(resourceId, platformId);
    return { resource: containerInfo, isLoading, error: error as any };
  },
};

export const ImageName = ({ image }: { image: ImageView | undefined }) => {
  if (image === undefined || image?.name === undefined) return <div className="text-muted">{'<none>'}</div>;
  return (
    <Link
      to={`/platforms/${image.platformId}/images/${image?.dockerImageId?.slice(0, 24)}`}
      title={image?.name}
      className="table-link">
      {truncate(image?.name?.length > 0 ? image?.name : '<none>', 24)}
    </Link>
  );
};
