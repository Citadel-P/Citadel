import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

export type RuntimeTargetOption = {
  value: string;
  label: string;
  disabled?: boolean;
};

export const RuntimeTargetSelect = ({
  value,
  options,
  placeholder,
  ariaLabel,
  onValueChange,
}: {
  value?: string;
  options: RuntimeTargetOption[];
  placeholder: string;
  ariaLabel: string;
  onValueChange: (value: string) => void;
}) => (
  <Select value={value} onValueChange={onValueChange}>
    <SelectTrigger aria-label={ariaLabel} className="h-8 min-w-42 rounded-sm bg-background shadow-xs">
      <SelectValue placeholder={placeholder} />
    </SelectTrigger>
    <SelectContent className="bg-background">
      <SelectGroup>
        {options.map((option) => (
          <SelectItem key={option.value} value={option.value} disabled={option.disabled}>
            {option.label}
          </SelectItem>
        ))}
      </SelectGroup>
    </SelectContent>
  </Select>
);
