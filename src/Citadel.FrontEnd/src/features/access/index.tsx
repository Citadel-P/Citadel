import { TabbedResourceComponents } from '@/pages/types';
import { Users } from './users';
import { Teams } from './teams';
import { Roles } from './roles';
import { UserKey } from 'lucide-react';
import { useUsersList } from './users/hooks/useUsersList';
import { ActionBar } from '@/components/custom/action-bar';
import { UserDropdownActions, UserGroupActions } from './users/actions';

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
      Content: ({ items, isLoading, actions }) => {
        return <Users items={items} isLoading={isLoading} actions={actions} />;
      },
      Header: {
        showAdd: true,
        showSearch: false,
        addButtonTitle: 'Add User',
        addButtonUrl: '/access/users/add',
      },
      DropdownActions: UserDropdownActions,
      GroupActions: ({ items }) => {
        return <ActionBar type="User" items={items} actions={Object.values(UserGroupActions)} />;
      },
      useData: () => {
        const { pagedUsers, isLoading } = useUsersList();
        return {
          items: pagedUsers as any,
          isLoading,
        };
      },
    },
    {
      label: 'Teams',
      slug: 'teams',
      Content: ({ items, isLoading, actions }) => {
        return <Teams items={items} isLoading={isLoading} actions={actions}/>;
      },
      Header: {
        showAdd: true,
        showSearch: false,
        addButtonTitle: 'Add Team',
        addButtonUrl: '/access/teams/add',
      },
      useData: () => {
        const { pagedUsers, isLoading } = useUsersList();
        return {
          items: pagedUsers as any,
          isLoading,
        };
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
        addButtonUrl: '/access/roles/add',
      },
    },
  ],
};
