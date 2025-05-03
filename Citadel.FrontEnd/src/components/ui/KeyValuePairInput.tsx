import { FormField, FormItem, FormControl } from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Plus, Trash2 } from 'lucide-react';

interface KeyValuePairInputProps {
  name: string;
  fields: any[];
  control: any;
  append: (value: { key: string; value: string }) => void;
  remove: (index: number) => void;
  label?: string;
  addButtonLabel?: string;
  keyPlaceHolder?: string;
  valuePlaceHolder?: string;
}

const KeyValuePairInput = ({
  name,
  fields,
  control,
  append,
  remove,
  label = 'Options',
  addButtonLabel = 'Add option',
  keyPlaceHolder = 'com.docker.network.driver.mtu',
  valuePlaceHolder = 'true',
}: KeyValuePairInputProps) => (
  <div className="col-span-2">
    <Label className="flex-none text-xs">{label}</Label>
    <div className="space-y-2">
      {fields.map((field, idx: number) => (
        <div key={field.id} className="flex gap-2 items-center">
          <FormField
            control={control}
            name={`${name}.${idx}.key`}
            render={({ field }) => (
              <FormItem className="flex-1">
                <FormControl>
                  <div className="flex items-stretch">
                    <span className="flex z-10 items-center px-6 bg-accent-foreground/5 border-l rounded-l-sm border-y border-border text-xs h-full">
                      key
                    </span>
                    <Input
                      placeholder={keyPlaceHolder}
                      className="rounded-l-none focus-visible:ring-transparent"
                      {...field}
                    />
                  </div>
                </FormControl>
              </FormItem>
            )}
          />
          <FormField
            control={control}
            name={`${name}.${idx}.value`}
            render={({ field }) => (
              <FormItem className="flex-1">
                <FormControl>
                  <div className="flex items-stretch">
                    <span className="flex z-10 items-center px-6 bg-accent-foreground/5 border-l rounded-l-sm border-y border-border text-xs h-full">
                      value
                    </span>
                    <Input placeholder={valuePlaceHolder} className="rounded-l-none focus-visible:ring-transparent" {...field} />
                  </div>
                </FormControl>
              </FormItem>
            )}
          />
          <Button type="button" variant="destructive" size="icon" onClick={() => remove(idx)}>
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      ))}
      <Button
        type="button"
        variant="ghost"
        size="sm"
        className="text-xs text-foreground/70"
        onClick={() => append({ key: '', value: '' })}>
        <Plus className="h-2.5 w-2.5" /> {addButtonLabel}
      </Button>
    </div>
  </div>
);

export default KeyValuePairInput;
