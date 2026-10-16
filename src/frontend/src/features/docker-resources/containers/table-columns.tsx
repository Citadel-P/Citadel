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

const options = [
  { id: 'containerId', label: 'ID' },
  { id: 'image', label: 'Image' },
  { id: 'health', label: 'Health' },
  { id: 'CPU', label: 'CPU' },
  { id: 'memory', label: 'Memory' },
  { id: 'ports', label: 'Ports' },
] as const;
export const containerColumnsAtom = atomWithStorage<Record<string, boolean>>('containers-columns', {
  containerId: false,
  image: true,
  health: true,
  CPU: true,
  memory: true,
  ports: true,
});

export function ContainerColumnsButton() {
  const [visible, setVisible] = useAtom(containerColumnsAtom);
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline">
          <Columns3 className="size-3.5" />
          Columns
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {options.map(({ id, label }) => (
          <DropdownMenuCheckboxItem
            key={id}
            checked={visible[id] !== false}
            onSelect={(event) => event.preventDefault()}
            onCheckedChange={(checked) => setVisible((current) => ({ ...current, [id]: checked }))}>
            {label}
          </DropdownMenuCheckboxItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
