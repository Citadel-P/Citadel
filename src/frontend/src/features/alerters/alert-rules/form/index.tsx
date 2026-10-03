import { StateIndicator } from '@/components/custom/state-indicator';
import { AlertRuleForm } from './form';
import { StateBadge } from '@/components/custom/state-badge';
import { RequiredFormComponents, ResourceFormDataHookResult, RequiredFormFields } from '@/pages/types';
import { useRead } from '@/lib/hooks';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { ActivitiesTab } from '@/features/activities';
import { AlertRuleView } from '@/api/generated/api.types';
import { AlertRuleActions } from './actions';
import { hasCapability } from '@/lib/resource-capabilities';

const title = 'Rule';
export const AlertRuleFormComponents: RequiredFormComponents = {
  AddForm: {
    Header: {
      title,
    },
    Content: () => {
      return <AlertRuleForm mode="add" />;
    },
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }: { resource: RequiredFormFields }) => {
        return (
          <StateBadge
            indicator={<StateIndicator value={resource.status as any} className="mr-0" />}
            value={resource.status as any}
          />
        );
      },
      ActionButtons: ({ resource }) => {
        return <GenericActionBarButtons resource={resource} actions={Object.values(AlertRuleActions)} />;
      },
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource }) => {
          return <AlertRuleForm mode="edit" resource={resource} disabled={!hasCapability(resource, 'canWrite')} />;
        },
      },
      {
        label: 'Activities',
        Content: ({ resource }: { resource: AlertRuleView }) => {
          return <ActivitiesTab resourceId={resource.id} resourceType="AlertRule" />;
        },
      },
    ],
    useData: function (id?: string): ResourceFormDataHookResult {
      const { data, isLoading, error, refetch, isFetching } = useRead('getAlertRuleConfig', { id });
      return { error, refetch, isFetching, item: data?.data as any, isLoading };
    },
  },
};
