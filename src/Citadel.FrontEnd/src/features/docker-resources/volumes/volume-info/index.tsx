import { StateIndicator } from '@/components/custom/state-indicator';
import { HardDrive } from 'lucide-react';
import { DetailSection, DetailMetadata } from '@/components/custom/resource-detail';
import { DockerVolumeResultView } from '@/api/generated/api.types';
import { Box, Info } from 'lucide-react';
import { ContainerInfoTable } from './container-info-table';
import { useRead } from '@/lib/hooks';
import { StateBadge } from '@/components/custom/state-badge';
import { RequiredDockerInfoComponents } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { VolumeInfoTable } from './volume-info-table';
import { VolumeInfoActions } from './actions';
import { hasCapability } from '@/lib/resource-capabilities';
import { useSearchParams } from 'react-router';

export const VolumeInfoComponents: RequiredDockerInfoComponents<DockerVolumeResultView> = {
  Header: {
    Icon: HardDrive,
    Status: ({ resource }) => (
      <StateBadge
        indicator={<StateIndicator value={resource.inUse} className="mr-0" />}
        value={resource.inUse}
        label={resource.inUse ? 'In use' : 'Unused'}
      />
    ),
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
    <div className="flex flex-col gap-(--section-gap)">
      <DetailSection
        title="Volume configuration"
        description="Storage location, driver, and capacity reported by Docker."
        icon={Info}>
        <VolumeInfoTable volume={resource} />
      </DetailSection>
      <DetailSection title={`Attached containers (${resource.containers?.length ?? 0})`} icon={Box}>
        {resource.containers?.length ? (
          <ContainerInfoTable volume={resource} />
        ) : (
          <p className="text-sm text-muted-foreground">No containers currently reference this volume.</p>
        )}
      </DetailSection>
      <DetailMetadata title="Driver options" items={resource.options} />
      <DetailMetadata items={resource.labels} />
    </div>
  );
};
