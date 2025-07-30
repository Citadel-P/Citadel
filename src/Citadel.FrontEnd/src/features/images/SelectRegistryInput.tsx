import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useImagesContext } from './ImagesContext';

export default function SelectRegistryInput() {
  const { selectedRegistry, setSelectionChange, registries } = useImagesContext();

  return (
    <Select defaultValue={selectedRegistry?.name ?? ''} onValueChange={setSelectionChange}>
      <SelectTrigger className="w-[200px] shadow-none">
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
