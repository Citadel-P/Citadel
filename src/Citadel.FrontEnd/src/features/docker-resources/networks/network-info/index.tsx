import { DockerNetworkDetails, DockerNetworkResult } from '@/api/generated/api.types';
import { Box, Info, Share2 } from 'lucide-react';
import { ContainerInfoTable } from './container-info-table';
import { NetworkInfoTable } from './network-info-table';
import { IPAMInfoTable } from './ipam-info-table';
import { useRead } from '@/lib/hooks';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredDockerInfoComponents } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { NetworkInfoActions } from './actions';
import { DockerLabelsSection, KeyPairEntries, Section } from '@/components/custom/common';

export const NetworkInfoComponents: RequiredDockerInfoComponents<DockerNetworkDetails> = {
  Header: {
    Indicator: ({ resource }) => {
      return <StateIndicator value={Object.keys(resource.containers ?? {}).length > 0} />;
    },
    ActionButtons: ({ resource }) => {
      return (
        <GenericActionBarButtons
          resource={
            {
              id: resource.id,
              name: resource.name,
              inUse: Object.keys(resource.containers ?? {}).length > 0,
            } as DockerNetworkResult
          }
          actions={Object.values(NetworkInfoActions)}
        />
      );
    },
  },

  Tabs: [
    {
      label: 'Inspect',
      Content: ({ resource }) => <InspectNetworkWrapper resource={resource} />,
    },
  ],
  useData: (platformId: string, resourceId: string) => {
    const { data, isLoading, error } = useRead(`inspectNetwork`, { platformId, networkId: resourceId });
    return { resource: data?.data, isLoading, error };
  },
};

const InspectNetworkWrapper = ({ resource }: { resource: DockerNetworkDetails }) => {
  return (
    <div className="flex flex-col gap-8">
      <Section title="Details" Icon={Info}>
        <NetworkInfoTable network={resource} />
      </Section>
      {Object.keys(resource?.containers ?? {}).length !== 0 && (
        <Section title="Containers in this network" Icon={Box}>
          <ContainerInfoTable network={resource} />
          <KeyPairEntries items={resource?.options} />
        </Section>
      )}
      {resource?.ipam?.config?.length !== 0 && (
        <Section title="IPAM" Icon={Share2}>
          <IPAMInfoTable ipam={resource?.ipam ?? undefined} />
        </Section>
      )}
      <DockerLabelsSection labels={resource?.labels} />
    </div>
  );
};
