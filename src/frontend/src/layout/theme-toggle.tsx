import { Moon, Sun } from 'lucide-react';
import { UserTheme } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { useAppearance } from '@/lib/appearance/appearance-context';
import { cn } from '@/lib/utils';

export function ThemeToggle() {
  const { effectiveMode, setMode } = useAppearance();
  const isDark = effectiveMode === 'dark';
  const label = isDark ? 'Switch to light mode' : 'Switch to dark mode';
  const iconClass =
    'absolute inset-0 size-4 transition-[rotate,scale,opacity] duration-300 ease-out motion-reduce:transition-none';

  return (
    <Button
      variant="ghost"
      size="icon-sm"
      aria-label={label}
      title={label}
      onClick={() => setMode(isDark ? UserTheme.Light : UserTheme.Dark)}>
      <span className="relative size-4" aria-hidden="true">
        <Sun className={cn(iconClass, isDark ? 'rotate-90 scale-0 opacity-0' : 'rotate-0 scale-100 opacity-100')} />
        <Moon className={cn(iconClass, isDark ? 'rotate-0 scale-100 opacity-100' : '-rotate-90 scale-0 opacity-0')} />
      </span>
    </Button>
  );
}
