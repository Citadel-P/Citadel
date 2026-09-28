import type { ReactNode } from 'react';
import { cn } from '@/lib/utils';
import { Surface } from './surface';

export const ContentCard = ({ children, className }: { children: ReactNode; className?: string }) => (
  <Surface className={cn('p-0', className)}>{children}</Surface>
);
