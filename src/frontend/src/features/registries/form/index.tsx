import { RegistryForm } from './form';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RegistryActions } from './actions';
import { RequiredFormComponents, ResourceFormDataHookResult, RequiredFormFields } from '@/pages/types';
import { useRead } from '@/lib/hooks';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import {
  RegistryConfigResponse,
  AuthorizedRegistryView,
} from '@/api/generated/api.types';
import { ActivitiesTab } from '@/features/activities';
import { hasCapability } from '@/lib/resource-capabilities';
import { ResourceHeaderTagsEditor } from '@/features/tags/components';

export const RegistryFormComponents: RequiredFormComponents = {
  AddForm: {
    Content: () => {
      return <RegistryForm mode="add" />;
    },
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }: { resource: RequiredFormFields }) => {
        return <StateIndicator variant="badge" value={resource.status as any} />;
      },
      ActionButtons: ({ resource }) => {
        return <GenericActionBarButtons resource={resource} actions={Object.values(RegistryActions)} />;
      },
      Tags: ({ resource }: { resource: RegistryConfigResponse }) => (
        <ResourceHeaderTagsEditor
          resourceType="Registry"
          resourceId={resource.id}
          tags={resource.tags}
          disabled={!hasCapability(resource, 'canWrite')}
        />
      ),
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource }) => {
          return <RegistryForm mode="edit" resource={resource} disabled={!hasCapability(resource, 'canWrite')} />;
        },
      },
      {
        label: 'Activities',
        Content: ({ resource }: { resource: AuthorizedRegistryView }) => {
          return <ActivitiesTab resourceId={resource.id} resourceType="Registry" />;
        },
      },
    ],
    useData: function (id?: string): ResourceFormDataHookResult {
      const { data, isLoading, error, refetch, isFetching } = useRead('getRegistryConfig', { id });
      return { error, refetch, isFetching, item: data?.data, isLoading };
    },
  },
};
