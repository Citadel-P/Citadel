import { DockerVolumeResultView } from '@/api/generated/api.types';
import { Box, Info } from 'lucide-react';
import { ContainerInfoTable } from './container-info-table';
import { useRead } from '@/lib/hooks';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredDockerInfoComponents } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { VolumeInfoTable } from './volume-info-table';
import { VolumeInfoActions } from './actions';
import { DockerLabelsSection, KeyPairEntries, Section } from '@/components/custom/common';
import { hasCapability } from '@/lib/resource-capabilities';
import { useSearchParams } from 'react-router';

export const VolumeInfoComponents: RequiredDockerInfoComponents<DockerVolumeResultView> = {
  Header: {
    Indicator: ({ resource }) => {
      return <StateIndicator value={resource?.inUse ?? false} />;
    },
    ActionButtons: ({ resource }) => {
      return <GenericActionBarButtons resource={resource} actions={Object.values(VolumeInfoActions)} />;
    },
  },

  Tabs: [
    {
      label: 'Inspect',
      disabled: (resource: DockerVolumeResultView) => !hasCapability(resource, 'canRead'),
      Content: ({ resource }) => <InspectVolumeWrapper resource={resource} />,
    },
  ],
  useData: (platformId: string, resourceId: string) => {
    const [searchParams] = useSearchParams();
    const dockerNodeId = searchParams.get('dockerNodeId') ?? undefined;
    const { data, isLoading, error } = useRead(`inspectVolume`, {
      platformId,
      name: resourceId,
      query: { dockerNodeId },
    });
    return { resource: data?.data, isLoading, error };
  },
};

const InspectVolumeWrapper = ({ resource }: { resource: DockerVolumeResultView }) => {
  return (
    <div className="flex flex-col gap-8">
      <Section title="Details" Icon={Info}>
        <VolumeInfoTable volume={resource} />
        <KeyPairEntries items={resource?.options} />
      </Section>
      {Object.keys(resource?.containers ?? {}).length !== 0 && (
        <Section title="Containers" Icon={Box}>
          <ContainerInfoTable volume={resource} />
        </Section>
      )}
      <DockerLabelsSection labels={resource?.labels} />
    </div>
  );
};
