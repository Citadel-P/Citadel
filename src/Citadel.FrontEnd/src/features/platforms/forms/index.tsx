import { PlatformView } from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { StateIndicator } from '@/components/custom/state-indicator';
import { ActivitiesTab } from '@/features/activities';
import { ResourceHeaderTagsEditor } from '@/features/tags/components';
import { hasCapability } from '@/lib/resource-capabilities';
import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { PlatformInfoActions } from '../actions';
import { PlatformForm } from './form';
import { usePlatformGroup } from './hooks/usePlatformGroup';
import { PlatformResourceSummary, PlatformStatsTab } from './platform-stats';

type PlatformFormResource = PlatformView & RequiredFormFields;

const toFormResource = (platform: PlatformView): PlatformFormResource =>
  ({
    ...platform,
    description: platform.description ?? null,
  }) as PlatformFormResource;

export const PlatformFormComponents: RequiredFormComponents<PlatformFormResource> = {
  AddForm: {
    Content: () => <PlatformForm mode="add" />,
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }) => <StateIndicator value={resource.status} kind="platform" />,
      ActionButtons: ({ resource }) => (
        <GenericActionBarButtons resource={resource} actions={[PlatformInfoActions.delete]} />
      ),
      Tags: ({ resource }) => (
        <ResourceHeaderTagsEditor
          resourceType="Platform"
          resourceId={resource.id}
          tags={resource.tags}
          disabled={!hasCapability(resource, 'canWrite')}
        />
      ),
    },
    SubHeader: ({ resource }) => <PlatformResourceSummary platform={resource} />,
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource }) => (
          <PlatformForm mode="edit" resource={resource} disabled={!hasCapability(resource, 'canWrite')} />
        ),
      },
      {
        label: 'Stats',
        Content: ({ resource }) => <PlatformStatsTab platform={resource} />,
      },
      {
        label: 'Activity',
        Content: ({ resource }) => <ActivitiesTab resourceId={resource.id} resourceType="Platform" />,
      },
    ],
    useData: function (id: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { platform, isLoading } = usePlatformGroup(id);
      return { item: platform ? toFormResource(platform) : undefined, isLoading };
    },
  },
};
