import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { ArrowDown } from 'lucide-react';

interface PullBadgeProps {
  onClick: () => void;
  className?: string;
}

export const PullImageBadge: React.FC<PullBadgeProps> = ({ onClick, className }) => {
  return (
    <Badge
      onClick={onClick}
      className={cn(
        'flex text-right cursor-pointer invisible group/versionrowdown group-hover/versionrow:visible truncate rounded-full hover:bg-primary/90',
        className,
      )}>
      <span>Pull</span>
      <ArrowDown className="ml-1 h-3.5 w-3.5 dark:text-foreground group-hover/versionrowdown:animate-bounce" />
    </Badge>
  );
};
