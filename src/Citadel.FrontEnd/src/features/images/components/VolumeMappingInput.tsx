import { Control, UseFieldArrayAppend, UseFieldArrayRemove, FieldArrayWithId } from 'react-hook-form';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { FormField, FormItem, FormControl, FormMessage } from '@/components/ui/form';
import { Label } from '@/components/ui/label';
import { Plus, Trash2 } from 'lucide-react';
import { truncate } from '@/lib/truncate';

interface VolumeMappingInputProps {
  control: Control<any>;
  name: string;
  fields: FieldArrayWithId<any, 'volumes'>[];
  append: UseFieldArrayAppend<any, 'volumes'>;
  remove: UseFieldArrayRemove;
  containerVolumes: string[];
}

const VolumeMappingInput = ({ control, name, fields, append, remove, containerVolumes }: VolumeMappingInputProps) => {
  return (
    <div className="space-y-2">
      <Label>Volumes</Label>
      <div className="space-y-2">
        {fields.map((field, index) => (
          <div key={field.id} className="flex items-start space-x-2">
            <div className="grid flex-1 grid-cols-1 gap-2 md:grid-cols-2">
              <FormField
                control={control}
                name={`${name}.${index}.hostPath`}
                render={({ field }) => (
                  <FormItem>
                    <FormControl>
                      <Input {...field} placeholder="Host path" className="rounded-sm focus-visible:ring-transparent" />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={control}
                name={`${name}.${index}.containerPath`}
                render={({ field }) => (
                  <FormItem>
                    <Select onValueChange={field.onChange} defaultValue={field.value}>
                      <FormControl>
                        <SelectTrigger className="w-full h-10">
                          <SelectValue placeholder="Container path" />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent className="bg-background">
                        <SelectGroup>
                          <SelectLabel>Container path</SelectLabel>
                          {containerVolumes.map((vol) => (
                            <SelectItem key={vol} value={vol}>
                              {truncate(vol ?? '', 24)}
                            </SelectItem>
                          ))}
                        </SelectGroup>
                      </SelectContent>
                    </Select>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>
            <Button type="button" variant="destructive" size="icon" onClick={() => remove(index)}>
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        ))}
      </div>
      <Button
        variant="ghost"
        size="sm"
        className="text-xs text-foreground/70"
        onClick={() => append({ hostPath: '', containerPath: '' })}>
        <Plus className="h-2.5 w-2.5" /> Add volume mapping
      </Button>
    </div>
  );
};

export default VolumeMappingInput;
