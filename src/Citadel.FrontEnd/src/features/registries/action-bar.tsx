import { ActionBar } from '@/components/custom/action-bar';
import { useRegistriesContext } from './RegistriesContext';
import { RegistriesActionBarButtons } from './action-bar-buttons';

export const RegistriesActionBar = () => {
  const { selectedRows, registries, setDialogData } = useRegistriesContext();

  return (
    <ActionBar
      selectedRows={selectedRows}
      allItems={registries}
      resource="Registry"
      actionButtons={<RegistriesActionBarButtons selectedRows={selectedRows} setDialogData={setDialogData} />}
    />
  );
};
