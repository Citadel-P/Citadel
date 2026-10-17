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

export const networkOptionalColumns = [
  { id: 'properties', label: 'Properties' },
  { id: 'gateway', label: 'Gateway' },
  { id: 'ipRange', label: 'IP range' },
  { id: 'ipamDriver', label: 'IPAM driver' },
  { id: 'created', label: 'Created' },
  { id: 'networkId', label: 'Network ID' },
] as const;

export type NetworkOptionalColumn = (typeof networkOptionalColumns)[number]['id'];
export const networkColumnsAtom = atomWithStorage<NetworkOptionalColumn[]>('networks-optional-columns', []);

export function NetworkColumnsButton() {
  const [visible, setVisible] = useAtom(networkColumnsAtom);
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline">
          <Columns3 className="size-3.5" />
          Columns
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {networkOptionalColumns.map(({ id, label }) => (
          <DropdownMenuCheckboxItem
            key={id}
            checked={visible.includes(id)}
            onSelect={(event) => event.preventDefault()}
            onCheckedChange={(checked) =>
              setVisible((current) =>
                checked ? [...new Set([...current, id])] : current.filter((value) => value !== id),
              )
            }>
            {label}
          </DropdownMenuCheckboxItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
