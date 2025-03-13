import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { ImagesContext } from './ImagesProvider';
import { useContextSelector } from 'use-context-selector';

export default function SelectRegistryInput() {
  const registries = useContextSelector(ImagesContext, (v) => v?.registries) ?? [];
  const selectedRegistry = useContextSelector(ImagesContext, (v) => v?.selectedRegistry);
  const setSelectionChange = useContextSelector(ImagesContext, (v) => v?.setSelectionChange);

  return (
    <Select defaultValue={selectedRegistry?.name ?? ''} onValueChange={setSelectionChange}>
      <SelectTrigger className="w-[200px]">
        <SelectValue placeholder="Select a registry" />
      </SelectTrigger>
      <SelectContent className="bg-background">
        <SelectGroup>
          <SelectLabel>Registries</SelectLabel>
          {registries &&
            registries.map((registry) => (
              <SelectItem key={registry.id} value={registry.name ?? ''}>
                {registry.name}
              </SelectItem>
            ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  );
}
