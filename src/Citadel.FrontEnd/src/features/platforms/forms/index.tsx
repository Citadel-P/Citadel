import { RequiredFormComponents } from '@/pages/types';
import { PlatformForm } from './form';

export const PlatformFormComponents: RequiredFormComponents = {
  AddForm: {
    Content: () => <PlatformForm mode="add" />,
  },
  EditForm: undefined,
};
