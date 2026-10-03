import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { HelpCircle, Plus, Trash2 } from 'lucide-react';

export interface KVPair {
  key: string;
  value: string;
}

export function KeyValuePairInput({
  label = 'Options',
  value,
  onChange,
  addButtonLabel = 'Add option',
  keyPlaceHolder = 'com.example.key',
  valuePlaceHolder = 'value',
  helpText,
}: {
  label?: string;
  value: KVPair[];
  onChange: (next: KVPair[]) => void;
  addButtonLabel?: string;
  keyPlaceHolder?: string;
  valuePlaceHolder?: string;
  helpText?: string;
}) {
  const updateField = (index: number, field: 'key' | 'value', val: string) => {
    const next = [...value];
    next[index] = { ...next[index], [field]: val };
    onChange(next);
  };

  const remove = (index: number) => {
    const next = [...value];
    next.splice(index, 1);
    onChange(next);
  };

  const append = () => {
    onChange([...(value ?? []), { key: '', value: '' }]);
  };

  return (
    <div className="col-span-2">
      <div className="flex items-center gap-1 mb-2">
        <Label className="text-sm font-normal">{label}</Label>

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
        {value.map((pair, idx) => (
          <div key={idx} className="flex flex-col sm:flex-row gap-2 items-start sm:items-center">
            <div className="flex-1 w-full">
              <div className="flex items-stretch">
                <span className="flex items-center justify-center px-4 bg-accent-foreground/5 border border-r-0 border-input rounded-l-sm text-sm shadow-xs text-muted-foreground whitespace-nowrap min-w-fit">
                  key
                </span>
                <Input
                  placeholder={keyPlaceHolder}
                  className="rounded-l-none focus-visible:ring-transparent"
                  value={pair.key}
                  onChange={(e) => updateField(idx, 'key', e.target.value)}
                />
              </div>
            </div>

            <div className="flex-1 w-full">
              <div className="flex items-stretch">
                <span className="flex items-center justify-center px-4 bg-accent-foreground/5 border border-r-0 border-input rounded-l-sm text-sm text-muted-foreground shadow-xs whitespace-nowrap min-w-fit">
                  value
                </span>
                <Input
                  placeholder={valuePlaceHolder}
                  className="rounded-l-none focus-visible:ring-transparent"
                  value={pair.value}
                  onChange={(e) => updateField(idx, 'value', e.target.value)}
                />
              </div>
            </div>

            <Button type="button" variant="destructive" size="icon" onClick={() => remove(idx)}>
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        ))}

        <Button type="button" variant="ghost" size="sm" className="text-xs text-foreground/70 h-8" onClick={append}>
          <Plus className="h-3 w-3 mr-1" /> {addButtonLabel}
        </Button>
      </div>
    </div>
  );
}
