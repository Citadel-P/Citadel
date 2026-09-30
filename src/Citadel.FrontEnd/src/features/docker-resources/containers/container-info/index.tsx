import { StateIndicator } from '@/components/custom/state-indicator';
import { StateBadge } from '@/components/custom/state-badge';
import { RequiredDockerInfoComponents } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { ContainerLogs } from './container-logs';
import { ContainerDetailsView, useContainerInfoGroup } from '../hooks/useContainerInfoGroup';
import { ContainerInspect } from './container-inspect';
import { ContainerStats } from './container-stats';
import { ContainerInfoActions, getContainerManagementAction } from './actions';
import { ContainerOverview } from './container-info-table';
import { ContainerStateStatus, ContainerRuntimeView, ImageView, ResourceControlState } from '@/api/generated/api.types';
import { hasCapability } from '@/lib/resource-capabilities';
import { Link } from 'react-router';
import { truncate } from '@/lib/truncate';
import { ContainerExec } from './container-exec';
import { SystemContainerBadge } from '../system-container-badge';
import { useAppContext } from '@/lib/context/app-context';
import { UnmanagedResourceIcon } from '@/components/custom/common';
import { Box } from 'lucide-react';

const groupedContainerInfoActions = Object.values(ContainerInfoActions).filter(
  (action) => action !== ContainerInfoActions.adopt && action !== ContainerInfoActions.importStack,
);

const ContainerInfoActionButtons = ({ resource }: { resource: ContainerDetailsView }) => {
  const { currentPlatform } = useAppContext();
  const managementAction = getContainerManagementAction(resource, currentPlatform?.type);

  return (
    <GenericActionBarButtons
      resource={resource}
      actions={groupedContainerInfoActions}
      standaloneActions={managementAction ? [managementAction] : []}
    />
  );
};

const ContainerNameSuffix = ({ resource }: { resource: ContainerDetailsView }) => {
  const { currentPlatform } = useAppContext();

  if (resource.isSystem) return <SystemContainerBadge role={resource.systemRole} />;
  if (getContainerManagementAction(resource, currentPlatform?.type) !== ContainerInfoActions.adopt) return null;

  return <UnmanagedResourceIcon title="Unmanaged Container" />;
};

export const ContainerInfoComponents: RequiredDockerInfoComponents<ContainerDetailsView> = {
  Header: {
    Icon: Box,
    Status: ({ resource }) => (
      <StateBadge
        indicator={<StateIndicator value={resource.state} className="mr-0" kind="container" />}
        value={resource.state}
        kind="container"
        isProcessing={resource.controlState === ResourceControlState.Processing}
      />
    ),
    NameSuffix: ContainerNameSuffix,
    ActionButtons: ContainerInfoActionButtons,
  },
  SubHeader: ({ resource }) => {
    return <ContainerOverview container={resource} />;
  },
  Tabs: [
    {
      label: 'Logs',
      disabled: (resource: ContainerRuntimeView) => !hasCapability(resource, 'canViewLogs'),
      Content: ({ resource }) => (
        <ContainerLogs containerId={(resource as ContainerDetailsView | undefined)?.resourceId ?? resource?.id} />
      ),
    },
    {
      label: 'Inspect',
      disabled: (resource: ContainerRuntimeView) => !hasCapability(resource, 'canInspect'),
      Content: ({ resource }) => <ContainerInspect containerId={resource?.id} />,
    },
    {
      label: 'Terminal',
      disabled: (resource: ContainerRuntimeView) =>
        resource.state !== ContainerStateStatus.Running || !hasCapability(resource, 'canOpenTerminal'),
      Content: ({ resource }) => (
        <ContainerExec
          containerId={(resource as ContainerDetailsView | undefined)?.resourceId ?? resource?.id}
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
    const { containerInfo, isLoading, error, refetch, isFetching } = useContainerInfoGroup(resourceId, platformId);
    return { refetch, isFetching, resource: containerInfo, isLoading, error: error as any };
  },
};

export const ImageName = ({ image }: { image: ImageView | undefined }) => {
  if (image === undefined || image?.name === undefined) return <div className="text-muted-foreground">{'<none>'}</div>;
  return (
    <Link
      to={`/platforms/${image.platformId}/images/${image?.dockerImageId?.slice(0, 24)}`}
      title={image?.name}
      className="table-link">
      {truncate(image?.name?.length > 0 ? image?.name : '<none>', 24)}
    </Link>
  );
};
