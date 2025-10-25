import { ActionBarButtons } from './action-bar-buttons';
import { GenericActionBar } from '@/components/custom/action-bar';
import { ImageView } from '@/api/generated/api.types';
import { useDeleteDialog } from '@/lib/hooks';
import { useSelectedResources } from '@/lib/atoms';

export const ActionBar = ({ items }: { items: ImageView[] }) => {
  const type = 'Image';
  const { openDialog } = useDeleteDialog<ImageView>({ type });
  const [selectedRows, _] = useSelectedResources<ImageView>(type);

  if (!selectedRows?.length) return null;

  return (
    <GenericActionBar
      selectedItems={selectedRows}
      allItems={items}
      resource={type}
      actionButtons={<ActionBarButtons selectedRows={selectedRows} openDialog={openDialog} />}
    />
  );
};
