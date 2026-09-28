import { StateIndicator } from '@/components/custom/state-indicator';
import { Network } from 'lucide-react';
import { DetailSection, DetailMetadata } from '@/components/custom/resource-detail';
import { DockerNetworkDetailsView } from '@/api/generated/api.types';
import { Box, Info, Share2 } from 'lucide-react';
import { ContainerInfoTable } from './container-info-table';
import { NetworkInfoTable } from './network-info-table';
import { IPAMInfoTable } from './ipam-info-table';
import { useRead } from '@/lib/hooks';
import { StateBadge } from '@/components/custom/state-badge';
import { RequiredDockerInfoComponents } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { NetworkInfoActions } from './actions';
import { hasCapability } from '@/lib/resource-capabilities';
import { SystemBadge } from '@/components/custom/system-badge';
import { useSearchParams } from 'react-router';

export const NetworkInfoComponents: RequiredDockerInfoComponents<DockerNetworkDetailsView> = {
  Header: {
    Icon: Network,
    Status: ({ resource }) => (
      <StateBadge
        indicator={<StateIndicator value={Object.keys(resource.containers ?? {}).length > 0} className="mr-0" />}
        value={Object.keys(resource.containers ?? {}).length > 0}
        label={`${Object.keys(resource.containers ?? {}).length} connected containers`}
      />
    ),
    NameSuffix: ({ resource }) => (resource.isSystem ? <SystemBadge description="Docker system network" /> : null),
    ActionButtons: ({ resource }) => {
      return <GenericActionBarButtons resource={resource} actions={Object.values(NetworkInfoActions)} />;
    },
  },

  Tabs: [
    {
      label: 'Inspect',
      disabled: (resource: DockerNetworkDetailsView) => !hasCapability(resource, 'canRead'),
      Content: ({ resource }) => <InspectNetworkWrapper resource={resource} />,
    },
  ],
  useData: (platformId: string, resourceId: string) => {
    const [searchParams] = useSearchParams();
    const dockerNodeId = searchParams.get('dockerNodeId') ?? undefined;
    const { data, isLoading, error } = useRead(`inspectNetwork`, {
      platformId,
      networkId: resourceId,
      query: { dockerNodeId },
    });
    return { resource: data?.data, isLoading, error };
  },
};

const InspectNetworkWrapper = ({ resource }: { resource: DockerNetworkDetailsView }) => {
  return (
    <div className="flex flex-col gap-(--section-gap)">
      <DetailSection title="Network configuration" description="Driver, scope, and connection settings." icon={Info}>
        <NetworkInfoTable network={resource} />
      </DetailSection>
      <DetailSection title={`Connected containers (${Object.keys(resource.containers ?? {}).length})`} icon={Box}>
        {Object.keys(resource.containers ?? {}).length ? (
          <ContainerInfoTable network={resource} />
        ) : (
          <p className="text-sm text-muted-foreground">No containers are connected to this network.</p>
        )}
      </DetailSection>
      <DetailMetadata title="Driver options" items={resource.options} />
      {!!resource.ipam?.config?.length && (
        <DetailSection
          title="IP address management"
          description="Subnets, gateways, and address ranges configured for this network."
          icon={Share2}>
          <IPAMInfoTable ipam={resource?.ipam ?? undefined} />
        </DetailSection>
      )}
      <DetailMetadata items={resource.labels} />
    </div>
  );
};
