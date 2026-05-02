import { ImageView, InspectImageView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredDockerInfoComponents } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { Box, Info, Layers } from 'lucide-react';
import { ImageInfoTable } from './image-info-table';
import { ContainerInfoTable } from './container-info-table';
import { ImageLayerTable } from './image-layer-table';
import { ImageInfoActions } from './actions';
import { DockerLabelsSection, Section } from '@/components/custom/common';

export const ImageInfoComponents: RequiredDockerInfoComponents<InspectImageView> = {
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
              dockerImageId: resource.id,
              name: resource.name,
              tag: resource.tag,
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
      Content: ({ resource }) => <InspectImageWrapper resource={resource} />,
    },
  ],

  useData: (platformId: string, resourceId: string) => {
    const { data, isLoading, error } = useRead('inspectImage', { platformId, imageId: resourceId });
    return { resource: data?.data, isLoading, error };
  },
};

const InspectImageWrapper = ({ resource }: { resource: InspectImageView }) => {
  return (
    <div className="flex flex-col gap-8">
      <Section title="Details" Icon={Info}>
        <ImageInfoTable image={resource} />
      </Section>

      {Object.keys(resource?.containers ?? {}).length !== 0 && (
        <Section title="Containers from this image" Icon={Box}>
          <ContainerInfoTable image={resource} />
        </Section>
      )}

      <Section title={`Layers (${resource.layers?.length ?? 0})`} Icon={Layers}>
        <ImageLayerTable image={resource} />
      </Section>

      <DockerLabelsSection labels={resource?.labels} />
    </div>
  );
};
