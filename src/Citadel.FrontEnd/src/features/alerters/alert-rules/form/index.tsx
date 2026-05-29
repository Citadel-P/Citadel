import { AlertRuleForm } from './form';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
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
        return <StateIndicator value={resource.status as any} />;
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
    useData: function (id?: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { data, isLoading } = useRead('getAlertRuleConfig', { id });
      return { item: data?.data as any, isLoading };
    },
  },
};
