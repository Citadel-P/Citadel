import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { DeploymentForm } from './form';

export const DeploymentFormComponents: RequiredFormComponents = {
  AddForm: {
    Content: () => {
      return <DeploymentForm mode="add" />;
    },
  },
  EditForm: {
    Header: {
      Indicator: undefined,
      ActionButtons: undefined,
    },
    Tabs: [],
    useData: function (id: string): { item?: RequiredFormFields; isLoading: boolean } {
      throw new Error('Function not implemented.');
    },
  },
};
