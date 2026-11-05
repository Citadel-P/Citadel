import type { ComponentProps, ReactNode } from 'react';
import * as RadioGroupPrimitive from '@radix-ui/react-radio-group';
import { cn } from '@/lib/utils';

type SegmentedControlProps = ComponentProps<typeof RadioGroupPrimitive.Root> & {
  options: readonly { value: string; label: ReactNode }[];
};

export function SegmentedControl({ options, className, ...props }: SegmentedControlProps) {
  return (
    <RadioGroupPrimitive.Root
      orientation="horizontal"
      className={cn('inline-flex h-7 w-fit items-center gap-0.5 rounded-md border bg-muted/40 p-px', className)}
      {...props}>
      {options.map(({ value, label }) => (
        <RadioGroupPrimitive.Item
          key={value}
          value={value}
          className="inline-flex h-full items-center justify-center rounded-sm px-2 text-[11px] font-medium leading-none whitespace-nowrap text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 data-[state=checked]:bg-background data-[state=checked]:text-foreground data-[state=checked]:shadow-xs motion-reduce:transition-none">
          {label}
        </RadioGroupPrimitive.Item>
      ))}
    </RadioGroupPrimitive.Root>
  );
}
