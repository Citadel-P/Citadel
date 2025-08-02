import { HardDrive, Plus } from 'lucide-react';
import { SearchField } from '@/components/ui/SearchField';
import { Button } from '@/components/ui/button';
import { useNavigate } from 'react-router';
import { useVolumesContext } from './VolumesContext';
import { VolumesActionBar } from './VolumesActionBar';
import VolumesTable from './VolumesTable';

const Volumes = () => {
  const navigate = useNavigate();
  const { onSearch } = useVolumesContext();

  return (
    <div className="flex-col justify-between relative">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="w-full rounded-lg border-border bg-background p-4">
          {/* Header */}
          <div className="sm:flex sm:justify-between">
            <div className="mb-3 flex items-baseline gap-1">
              <div className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                <HardDrive className="h-4 w-4" />
                <span className="sr-only">Volumes</span>
              </div>
              <div className="text-md font-bold text-foreground">Volumes</div>
            </div>
            <div className="flex gap-2">
              <SearchField onSearch={onSearch} />
              <Button
                type="button"
                onClick={() => navigate('./add')}
                className="inline-flex items-center bg-primary hover:bg-primary/80 font-medium rounded-sm text-xs px-2.5 py-2.5">
                <Plus className="h-3 w-3" /> Add Volume
              </Button>
            </div>
          </div>
          <VolumesTable />
        </div>
      </div>
      <VolumesActionBar />
    </div>
  );
};

export default Volumes;
