import { FormField, FormItem, FormControl } from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Plus, Trash2, HelpCircle } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

interface ValueInputProps {
  name: string;
  fields: any[];
  control: any;
  append: (value: { value: string }) => void;
  remove: (index: number) => void;
  label?: string;
  addButtonLabel?: string;
  valuePlaceHolder?: string;
  helpText?: string;
}

const ValueInput = ({
  name,
  fields,
  control,
  append,
  remove,
  label = 'Options',
  addButtonLabel = 'Add option',
  valuePlaceHolder = 'Value',
  helpText,
}: ValueInputProps) => (
  <div className="col-span-2">
    <div className="flex items-center gap-1">
      <Label className="flex-none text-xs mb-1">{label}</Label>
      {helpText && (
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <HelpCircle className="w-3.5 h-3.5 text-muted-foreground cursor-pointer" />
            </TooltipTrigger>
            <TooltipContent side="top">{helpText}</TooltipContent>
          </Tooltip>
        </TooltipProvider>
      )}
    </div>
    <div className="space-y-2">
      {fields.map((field, idx: number) => (
        <div key={field.id} className="flex flex-col sm:flex-row gap-2 items-start sm:items-center">
          <FormField
            control={control}
            name={`${name}.${idx}.value`}
            render={({ field }) => (
              <FormItem className="flex-1 w-full">
                <FormControl>
                  <Input
                    placeholder={valuePlaceHolder}
                    className="focus-visible:ring-transparent"
                    {...field}
                  />
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
        onClick={() => append({ value: '' })}>
        <Plus className="h-2.5 w-2.5" /> {addButtonLabel}
      </Button>
    </div>
  </div>
);

export default ValueInput;
