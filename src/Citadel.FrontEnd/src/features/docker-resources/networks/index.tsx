import { Network } from 'lucide-react';
import { NetworksTable } from './table';
import { ActionBar } from './action-bar';
import { RequiredDockerComponents } from '@/pages/types';
import { DeleteDialog } from './delete-dialog';

export const NetworkComponents: RequiredDockerComponents = {
  Icon: <Network className="h-4 w-4" />,
  Table: ({ items, isLoading }) => {
    return <NetworksTable items={items} isLoading={isLoading} />;
  },
  ActionBar: ({ items }) => {
    return <ActionBar items={items} />;
  },
  DeleteDialog: () => {
    return <DeleteDialog />;
  },
};
