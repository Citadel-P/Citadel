import { RequiredComponents } from '@/pages/types';
import { Users } from './users';
import { Teams } from './teams';
import { Roles } from './roles';
import { UserKey } from 'lucide-react';

export const AccessComponents: RequiredComponents = {
  Icon: <UserKey className="h-4 w-4" />,
  header: {
    subtitle: 'Manage users, teams, and their access.',
    showSearch: false,
    showAdd: false,
  },
  Tabs: [
    {
      label: 'Users',
      Content: () => {
        return <Users />;
      },
      Header: {
        showAdd: true,
        showSearch: true,
        addButtonTitle: 'Add User',
        addButtonUrl: './add-user',
      },
    },
    {
      label: 'Teams',
      Content: () => {
        return <Teams />;
      },
      Header: {
        showAdd: true,
        showSearch: true,
        addButtonTitle: 'Add Team',
        addButtonUrl: './add-team',
      },
    },
    {
      label: 'Roles',
      Content: () => {
        return <Roles />;
      },
      Header: {
        showAdd: true,
        showSearch: false,
        addButtonTitle: 'Add Role',
        addButtonUrl: './add-role',
      },
    },
  ],
};
