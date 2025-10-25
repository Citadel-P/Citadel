import { GenericActionBar } from '@/components/custom/action-bar';
import { useRegistriesContext } from './RegistriesContext';
import { ActionBarButtons } from './action-bar-buttons';

export const ActionBar = () => {
  const { selectedRows, registries, setDialogData } = useRegistriesContext();

  return (
    <GenericActionBar
      selectedItems={selectedRows}
      allItems={registries}
      resource="Registry"
      actionButtons={<ActionBarButtons selectedRows={selectedRows} setDialogData={setDialogData} />}
    />
  );
};
