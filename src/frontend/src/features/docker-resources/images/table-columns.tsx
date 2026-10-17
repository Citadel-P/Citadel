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
export const imageShowIdAtom = atomWithStorage('images-show-id', false);

export function ImageColumnsButton() {
  const [showRegistry, setShowRegistry] = useAtom(imageShowRegistryAtom);
  const [showCreated, setShowCreated] = useAtom(imageShowCreatedAtom);
  const [showId, setShowId] = useAtom(imageShowIdAtom);
  const options = [
    { label: 'ID', checked: showId, onCheckedChange: setShowId },
    { label: 'Registry', checked: showRegistry, onCheckedChange: setShowRegistry },
    { label: 'Created', checked: showCreated, onCheckedChange: setShowCreated },
  ];
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline">
          <Columns3 className="size-3.5" />
          Columns
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {options.map(({ label, checked, onCheckedChange }) => (
          <DropdownMenuCheckboxItem
            key={label}
            checked={checked}
            onSelect={(event) => event.preventDefault()}
            onCheckedChange={onCheckedChange}>
            {label}
          </DropdownMenuCheckboxItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
