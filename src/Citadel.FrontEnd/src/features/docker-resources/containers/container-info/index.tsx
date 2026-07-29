import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredDockerInfoComponents } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { ContainerLogs } from './container-logs';
import { useContainerInfoGroup } from '../hooks/useContainerInfoGroup';
import { ContainerInspect } from './container-inspect';
import { ContainerStats } from './container-stats';
import { ContainerInfoActions } from './actions';
import { ContainerInfoTable } from './container-info-table';
import { ContainerStateStatus, ContainerDataView, ImageView, ResourceControlState } from '@/api/generated/api.types';
import { hasCapability } from '@/lib/resource-capabilities';
import { Link } from 'react-router';
import { truncate } from '@/lib/truncate';
import { ContainerExec } from './container-exec';
import { Unlink } from 'lucide-react';
import { isUnmanagedContainer } from '@/lib/utils';
import { SystemContainerBadge } from '../system-container-badge';

export const ContainerInfoComponents: RequiredDockerInfoComponents<ContainerDataView> = {
  Header: {
    Indicator: ({ resource }) => {
      return (
        <StateIndicator
          value={resource.state}
          isProcessing={resource.controlState === ResourceControlState.Processing}
          kind="container"
        />
      );
    },
    NameSuffix: ({ resource }) =>
      resource.isSystem ? (
        <SystemContainerBadge role={resource.systemRole} />
      ) : isUnmanagedContainer(resource) ? (
        <span
          className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-sm text-amber-500"
          title="Unmanaged container">
          <Unlink className="h-3.5 w-3.5" />
          <span className="sr-only">Unmanaged container</span>
        </span>
      ) : null,
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
      disabled: (resource: ContainerDataView) => !hasCapability(resource, 'canViewLogs'),
      Content: ({ resource }) => <ContainerLogs containerId={resource?.id} />,
    },
    {
      label: 'Inspect',
      disabled: (resource: ContainerDataView) => !hasCapability(resource, 'canInspect'),
      Content: ({ resource }) => <ContainerInspect containerId={resource?.id} />,
    },
    {
      label: 'Terminal',
      disabled: (resource: ContainerDataView) =>
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
