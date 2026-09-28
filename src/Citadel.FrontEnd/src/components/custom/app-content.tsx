import type { ComponentProps } from 'react';
import { cn } from '@/lib/utils';

export function AppContent({ className, ...props }: ComponentProps<'div'>) {
  return <div className={cn('app-content flex-1', className)} {...props} />;
}
