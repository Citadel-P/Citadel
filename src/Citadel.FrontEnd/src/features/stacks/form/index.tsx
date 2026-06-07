import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { StackForm } from './form';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { StackActions } from './actions';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useStackGroup } from './hooks/useStackGroup';
import { StackView, ResourceControlState } from '@/api/generated/api.types';
import { ActivitiesTab } from '@/features/activities';
import { hasCapability } from '@/lib/resource-capabilities';

export const StackFormComponents: RequiredFormComponents = {
  AddForm: {
    Content: () => {
      return <StackForm mode="add" />;
    },
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }: { resource: RequiredFormFields }) => {
        return (
          <StateIndicator
            value={(resource as StackView).status}
            isProcessing={(resource as StackView).controlState === ResourceControlState.Processing}
          />
        );
      },
      ActionButtons: ({ resource }) => {
        return <GenericActionBarButtons resource={resource} actions={Object.values(StackActions)} />;
      },
    },
    SubHeader: ({ resource: _resource }: { resource: StackView }) => {
      return <></>; //<DeploymentSubHeader latestActivity={resource.latestActivityView ?? null} deployment={resource} />;
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource, metadataChanged }: { resource: StackView; metadataChanged?: boolean }) => {
          return (
            <StackForm mode="edit" metadataChanged={metadataChanged} disabled={!hasCapability(resource, 'canWrite')} />
          );
        },
      },
      {
        label: 'Activities',
        Content: ({ resource }: { resource: StackView }) => {
          return <ActivitiesTab resourceId={resource.id} resourceType="Stack" />;
        },
      },
    ],
    useData: function (id: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { stack, isLoading } = useStackGroup(id);
      return { item: stack, isLoading };
    },
  },
};
