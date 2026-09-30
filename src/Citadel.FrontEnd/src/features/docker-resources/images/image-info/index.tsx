import { StateIndicator } from '@/components/custom/state-indicator';
import { DetailSection, DetailMetadata } from '@/components/custom/resource-detail';
import { ImageView, ImageInspectionView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { StateBadge } from '@/components/custom/state-badge';
import { RequiredDockerInfoComponents } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { Box, Info, Layers } from 'lucide-react';
import { ImageInfoTable } from './image-info-table';
import { ContainerInfoTable } from './container-info-table';
import { ImageLayerTable } from './image-layer-table';
import { ImageInfoActions } from './actions';
import { hasCapability } from '@/lib/resource-capabilities';
import { useSearchParams } from 'react-router';

export const ImageInfoComponents: RequiredDockerInfoComponents<ImageInspectionView> = {
  Header: {
    Icon: Layers,
    Status: ({ resource }) => (
      <StateBadge
        indicator={<StateIndicator value={Boolean(resource.containers?.length)} className="mr-0" />}
        value={Boolean(resource.containers?.length)}
        label={resource.containers?.length ? 'In use' : 'Unused'}
      />
    ),
    ActionButtons: ({ resource }) => {
      return (
        <GenericActionBarButtons
          resource={
            {
              id: resource.id,
              dockerImageId: resource.id,
              name: resource.name,
              tag: resource.tag,
              dockerNodeId: resource.dockerNodeId,
              capabilities: resource.capabilities,
            } as Partial<ImageView>
          }
          actions={Object.values(ImageInfoActions)}
        />
      );
    },
  },

  Tabs: [
    {
      label: 'Inspect',
      disabled: (resource: ImageInspectionView) => !hasCapability(resource, 'canRead'),
      Content: ({ resource }: { resource: ImageInspectionView }) => <InspectImageWrapper resource={resource} />,
    },
  ],

  useData: (platformId: string, resourceId: string) => {
    const [searchParams] = useSearchParams();
    const dockerNodeId = searchParams.get('dockerNodeId') ?? undefined;
    const { data, isLoading, error, refetch, isFetching } = useRead('inspectImage', {
      platformId,
      imageId: resourceId,
      query: { dockerNodeId },
    });
    return { refetch, isFetching, resource: data?.data, isLoading, error };
  },
};

const InspectImageWrapper = ({ resource }: { resource: ImageInspectionView }) => {
  return (
    <div className="flex flex-col gap-(--section-gap)">
      <DetailSection title="Image details" description="Build platform, origin, and image size." icon={Info}>
        <ImageInfoTable image={resource} />
      </DetailSection>

      <DetailSection title={`Containers using this image (${resource.containers?.length ?? 0})`} icon={Box}>
        {resource.containers?.length ? (
          <ContainerInfoTable image={resource} />
        ) : (
          <p className="text-sm text-muted-foreground">No containers currently use this image.</p>
        )}
      </DetailSection>

      <DetailSection title={`Layers (${resource.layers?.length ?? 0})`} icon={Layers}>
        <ImageLayerTable image={resource} />
      </DetailSection>

      <DetailMetadata items={resource.labels} />
    </div>
  );
};
