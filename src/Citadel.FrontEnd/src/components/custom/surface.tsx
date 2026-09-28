import type { ComponentProps } from 'react';
import { cn } from '@/lib/utils';
export function Surface({ className, ...props }: ComponentProps<'section'>) {
  return (
    <section
      className={cn('rounded-lg border bg-card text-card-foreground p-(--surface-padding) shadow-xs', className)}
      {...props}
    />
  );
}
