import { PagedResultViewOfTeamView } from '@/api/generated/api.types';
import { DropdownActionComponent, RequiredFormComponents } from '@/pages/types';
import { UsersTable } from '../users';

export const Teams = ({
  items,
  actions,
  isLoading,
}: {
  items: PagedResultViewOfTeamView;
  actions: Record<string, DropdownActionComponent>;
  isLoading: boolean;
}) => <UsersTable pagedResult={items} isLoading={isLoading} actions={actions} />;

export const TeamFormComponents: RequiredFormComponents = {
  AddForm: {
    Header: {
      title: 'Team',
    },
    Content: () => <></>,
  },
};
