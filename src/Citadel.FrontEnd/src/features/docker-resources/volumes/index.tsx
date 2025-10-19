import { HardDrive } from 'lucide-react';
import { ActionBar } from './action-bar';
import { VolumesTable } from './table';
import { RequiredDockerComponents } from '@/pages/types';
import { DeleteDialog } from './delete-dialog';

export const VolumeComponents: RequiredDockerComponents = {
  Icon: <HardDrive className="h-4 w-4" />,
  Table: ({ items, isLoading }) => {
    return <VolumesTable items={items} isLoading={isLoading} />;
  },
  ActionBar: ({ items }) => {
    return <ActionBar items={items} />;
  },
  DeleteDialog: () => {
    return <DeleteDialog />;
  },
};
