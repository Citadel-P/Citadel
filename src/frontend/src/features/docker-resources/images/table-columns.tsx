import { useAtom } from 'jotai';
import { atomWithStorage } from 'jotai/utils';
import { Columns3 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuCheckboxItem,
} from '@/components/ui/dropdown-menu';

export const imageShowRegistryAtom = atomWithStorage('images-show-registry', false);
export const imageShowCreatedAtom = atomWithStorage('images-show-created', false);

export function ImageColumnsButton() {
  const [showRegistry, setShowRegistry] = useAtom(imageShowRegistryAtom);
  const [showCreated, setShowCreated] = useAtom(imageShowCreatedAtom);
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline">
          <Columns3 className="size-4" />
          Columns
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuCheckboxItem checked={showRegistry} onCheckedChange={setShowRegistry}>
          Registry
        </DropdownMenuCheckboxItem>
        <DropdownMenuCheckboxItem checked={showCreated} onCheckedChange={setShowCreated}>
          Created
        </DropdownMenuCheckboxItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
