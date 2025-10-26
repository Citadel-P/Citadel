import { ImageView, InspectImageView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredDockerInfoComponents } from '@/pages/types';
import { DeleteImageButton } from '../actions';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { Box, Info, Layers } from 'lucide-react';
import { ImageInfoTable } from './image-info-table';
import { ContainerInfoTable } from './container-info-table';
import { ImageLayerTable } from './image-layer-table';

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
            } as ImageView
          }
          actions={[DeleteImageButton]}
        />
      );
    },
  },

  Tabs: [
    {
      label: 'Inspect',
      Content: ({ resource }) => <InspectImageWrapper resource={resource} />,
    },
    {
      label: 'Activity',
      Content: () => <div>Activity</div>,
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
      <div className="flex flex-col gap-2">
        <div className="flex flex-row items-center gap-2">
          <Info width={14} height={14} className="text-muted-foreground" />
          <div className="text-sm font-semibold text-muted-foreground leading-none">Details</div>
        </div>
        <div className="space-y-1 rounded-sm border p-1 shadow-xs">
          <ImageInfoTable image={resource} />
        </div>
      </div>
      {Object.keys(resource?.containers ?? {}).length !== 0 && (
        <div className="flex flex-col gap-2">
          <div className="flex flex-row items-center gap-2">
            <Box width={14} height={14} className="text-muted-foreground" />
            <div className="text-sm font-semibold text-muted-foreground leading-none">Containers from this image</div>
          </div>
          <div className="space-y-1 rounded-sm border p-1 shadow-xs">
            <ContainerInfoTable image={resource} />
          </div>
        </div>
      )}

      <div className="flex flex-col gap-2">
        <div className="flex flex-row items-center gap-2">
          <Layers width={14} height={14} className="text-muted-foreground" />
          <div className="text-sm font-semibold text-muted-foreground leading-none">Layers</div>
        </div>
        <div className="space-y-1 rounded-sm border p-1 shadow-xs">
          <ImageLayerTable image={resource} />
        </div>
      </div>
    </div>
  );
};
