import { useCallback } from 'react';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import { StateIndicator } from '@/components/custom/state-indicator';
import { DetailSection, DetailMetadata } from '@/components/custom/resource-detail';
import { ImageInspectionView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { StateBadge } from '@/components/custom/state-badge';
import { RequiredDockerInfoComponents } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { Box, Info, Layers } from 'lucide-react';
import { ImageInfoTable } from './image-info-table';
import { ContainerInfoTable } from './container-info-table';
import { ImageLayerTable } from './image-layer-table';
import { ImageInfoActions, type ImageInfoActionTarget } from './actions';
import { hasCapability } from '@/lib/resource-capabilities';
import { useSearchParams } from 'react-router';
import { ImageUsage } from '../image-usage';

export const ImageInfoComponents: RequiredDockerInfoComponents<ImageInspectionView> = {
  Header: {
    Icon: Layers,
    Status: ({ resource }) => (
      <StateBadge
        indicator={<StateIndicator value={Boolean(resource.containers?.length)} className="mr-0" />}
        value={Boolean(resource.containers?.length)}
        label={resource.containers?.length ? 'Used by containers' : 'No containers'}
      />
    ),
    ActionButtons: ({ resource }) => {
      return (
        <GenericActionBarButtons
          resource={
            {
              dockerImageId: resource.id,
              tags: resource.repoTags,
              registry: resource.registry,
              name: resource.name,
              dockerNodeId: resource.dockerNodeId ?? null,
              capabilities: resource.capabilities ?? null,
              isInUse: resource.containers.length > 0,
              isStale: false,
            } satisfies ImageInfoActionTarget
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
    const refresh = useCallback(() => {
      void refetch({ cancelRefetch: false });
    }, [refetch]);
    const setupEventListeners = useCallback(
      (connection: RealtimeConnection) => {
        connection.on('ImagesInfoUpdated', refresh);
      },
      [refresh],
    );
    const removeEventListeners = useCallback(
      (connection: RealtimeConnection) => {
        connection.off('ImagesInfoUpdated', refresh);
      },
      [refresh],
    );
    useRealtimeGroup({
      groupName: `images:${platformId}`,
      setupEventListeners,
      removeEventListeners,
      skip: !platformId,
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

      <div id="workloads">
        <ImageUsage
          key={`${resource.id}-${resource.dockerNodeId}`}
          imageId={resource.id}
          dockerNodeId={resource.dockerNodeId}
          managedOnly>
          {(references, refresh) => (
            <DetailSection title="Workloads using this image" icon={Box} actions={refresh}>
              {!!resource.containers?.length && (
                <>
                  <h3 className="text-sm font-medium">Containers ({resource.containers.length})</h3>
                  <ContainerInfoTable image={resource} />
                </>
              )}
              {references}
            </DetailSection>
          )}
        </ImageUsage>
      </div>

      <DetailSection title={`Layers (${resource.layers?.length ?? 0})`} icon={Layers}>
        <ImageLayerTable image={resource} />
      </DetailSection>

      <DetailMetadata items={resource.labels} />
    </div>
  );
};
