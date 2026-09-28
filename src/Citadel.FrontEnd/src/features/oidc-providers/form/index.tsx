import { OidcProviderView, ResourceCapabilities } from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { StateIndicator } from '@/components/custom/state-indicator';
import { ActivitiesTab } from '@/features/activities';
import { useRead } from '@/lib/hooks';
import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { OidcProviderInfoActions } from '../actions';
import { OidcProviderForm } from './form';

type OidcProviderFormResource = OidcProviderView & RequiredFormFields & { capabilities: ResourceCapabilities };

export const OidcProviderFormComponents: RequiredFormComponents<OidcProviderFormResource> = {
  AddForm: {
    Header: {
      title: 'OIDC Provider',
    },
    Content: () => <OidcProviderForm mode="add" />,
  },
  EditForm: {
    Header: {
      canEditTitle: true,
      canEditDescription: true,
      Indicator: ({ resource }) => (
        <StateIndicator value={Boolean((resource as OidcProviderView).enabled)} enableLabel />
      ),
      ActionButtons: ({ resource }) => {
        const { edit: _edit, ...actions } = OidcProviderInfoActions;
        return <GenericActionBarButtons resource={resource} actions={Object.values(actions)} />;
      },
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource }) => <OidcProviderForm mode="edit" resource={resource as OidcProviderFormResource} />,
      },
      {
        label: 'Activities',
        Content: ({ resource }) => <ActivitiesTab resourceId={resource.id} resourceType="OidcProvider" />,
      },
    ],
    useData(id: string) {
      const { data, isLoading, error, refetch, isFetching } = useRead('getOidcProvider', { id });
      return {
        error,
        refetch,
        isFetching,
        item: data?.data
          ? ({
              ...data.data,
              status: data.data.enabled,
              description: data.data.description,
              capabilities: { canRead: true, canWrite: true, canExecute: false },
            } satisfies OidcProviderFormResource)
          : undefined,
        isLoading,
      };
    },
  },
};
