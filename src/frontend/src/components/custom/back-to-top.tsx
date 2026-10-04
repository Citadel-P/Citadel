import { useEffect, useState } from 'react';
import { ArrowUp } from 'lucide-react';
import { Button } from '@/components/ui/button';

export function BackToTop() {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const container = document.getElementById('main-scroll-container');
    if (!container) return;

    const updateVisibility = () => setVisible(container.scrollTop > 400);
    updateVisibility();
    container.addEventListener('scroll', updateVisibility, { passive: true });
    return () => container.removeEventListener('scroll', updateVisibility);
  }, []);

  if (!visible) return null;

  return (
    <Button
      type="button"
      variant="outline"
      size="icon-lg"
      aria-label="Back to top"
      title="Back to top"
      className="fixed right-4 bottom-24 z-30 rounded-full border-border/70 bg-background text-muted-foreground shadow-sm hover:text-foreground sm:right-6 xl:bottom-6 dark:bg-background motion-reduce:transition-none"
      onClick={() => {
        document.getElementById('main-scroll-container')?.scrollTo({
          top: 0,
          behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'instant' : 'smooth',
        });
      }}>
      <ArrowUp className="size-4" aria-hidden="true" />
    </Button>
  );
}
