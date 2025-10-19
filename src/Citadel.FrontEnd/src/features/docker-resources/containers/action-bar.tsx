import { ContainerView } from '@/api/generated/api.types';
import { ActionBarButtons } from './action-bar-buttons';
import { GenericActionBar } from '@/components/custom/action-bar';
import { useDeleteDialog } from '@/lib/hooks';
import { useSelectedResources } from '@/lib/atoms';

export const ActionBar = ({ items }: { items: ContainerView[] }) => {
  const type = 'Container';
  const { openDialog } = useDeleteDialog<ContainerView>({ type });
  const [selectedRows, _] = useSelectedResources<ContainerView>(type);

  return (
    <GenericActionBar
      selectedRows={selectedRows}
      allItems={items}
      resource={type}
      actionButtons={<ActionBarButtons selectedRows={selectedRows} openDialog={openDialog} />}
    />
  );
};
