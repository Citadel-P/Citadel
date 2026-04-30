import { TabbedResourceComponents } from '@/pages/types';
import { Users } from './users';
import { Teams } from './teams';
import { Roles } from './roles';
import { UserKey } from 'lucide-react';

export const AccessComponents: TabbedResourceComponents = {
  Icon: <UserKey className="h-4 w-4" />,
  header: {
    subtitle: 'Manage users, teams, and their access.',
    showSearch: false,
    showAdd: false,
  },
  Tabs: [
    {
      label: 'Users',
      slug: 'users',
      Content: () => {
        return <Users />;
      },
      Header: {
        showAdd: true,
        showSearch: true,
        addButtonTitle: 'Add User',
        addButtonUrl: '/users/add',
      },
    },
    {
      label: 'Teams',
      slug: 'teams',
      Content: () => {
        return <Teams />;
      },
      Header: {
        showAdd: true,
        showSearch: true,
        addButtonTitle: 'Add Team',
        addButtonUrl: '/teams/add',
      },
    },
    {
      label: 'Roles',
      slug: 'roles',
      Content: () => {
        return <Roles />;
      },
      Header: {
        showAdd: true,
        showSearch: false,
        addButtonTitle: 'Add Role',
        addButtonUrl: '/roles/add',
      },
    },
  ],
};
