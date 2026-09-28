import { ServiceAccountView } from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useRead } from '@/lib/hooks';
import { hasCapability } from '@/lib/resource-capabilities';
import { RequiredFormComponents, ResourceFormDataHookResult } from '@/pages/types';
import { ServiceAccountInfoActions } from '../actions';
import { ServiceAccountForm, ServiceAccountTokens } from './form';
import { ActivitiesTab } from '@/features/activities';

export const ServiceAccountFormComponents: RequiredFormComponents = {
  AddForm: {
    Header: { title: 'Service Account' },
    Content: () => <ServiceAccountForm mode="add" />,
  },
  EditForm: {
    skipMetadataUpdate: true,
    supportsHeaderRename: true,
    Header: {
      canEditDescription: false,
      Indicator: ({ resource }: { resource: ServiceAccountView }) => (
        <StateIndicator value={!resource.archivedAtUtc && resource.isEnabled} enableLabel={true} />
      ),
      ActionButtons: ({ resource }) => (
        <GenericActionBarButtons resource={resource} actions={[ServiceAccountInfoActions.archive]} />
      ),
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource }) => (
          <ServiceAccountForm
            mode="edit"
            resource={resource}
            disabled={!!resource.archivedAtUtc || !hasCapability(resource, 'canWrite')}
          />
        ),
      },
      {
        label: 'Tokens',
        Content: ({ resource }) => <ServiceAccountTokens resource={resource as ServiceAccountView} />,
      },
      {
        label: 'Activities',
        Content: ({ resource }) => <ActivitiesTab resourceId={resource.id} resourceType="ServiceAccount" />,
      },
    ],
    useData: (id?: string): ResourceFormDataHookResult => {
      const { data, isLoading, error, refetch, isFetching } = useRead('getServiceAccount', { id });
      return { error, refetch, isFetching, item: data?.data as any, isLoading };
    },
  },
};
