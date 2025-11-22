import { Button } from '@/components/ui/button';
import { Download, Minus, Plus } from 'lucide-react';
import { useInlineSubHeader, useResourceFilter, useTaskSheet } from '@/lib/atoms';
import { useEffect, useRef, useState } from 'react';
import autoAnimate from '@formkit/auto-animate';
import { cn } from '@/lib/utils';
import { ResourceSelector } from '@/components/custom/common';
import { Input } from '@/components/ui/input';
import { RegistryView } from '@/api/generated/api.types';

export const PullButton = () => {
  const { open, toggle } = useInlineSubHeader('Image');
  return (
    <Button
      type="button"
      aria-expanded={open}
      onClick={toggle}
      className="inline-flex items-center bg-primary hover:bg-primary/80 font-medium rounded-sm text-sm px-2.5 py-2.5">
      {open ? <Minus className="h-3 w-3 ml-1" /> : <Plus className="h-3 w-3 ml-1" />} Pull Image
    </Button>
  );
};

export default function PullImageForm() {
  const { open: openSheet } = useTaskSheet('Image');
  const { open } = useInlineSubHeader('Image');
  const [image, setImage] = useState('');
  const [registry, setRegistry] = useState<RegistryView | undefined>();

  const [resourceFilter] = useResourceFilter<{ item: RegistryView }>('Registry');

  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (ref.current) {
      autoAnimate(ref.current, {
        duration: 250,
        easing: 'ease-in-out',
      });
    }
  }, []);

  useEffect(() => {
    if (resourceFilter?.item) {
      setRegistry(resourceFilter.item);
    }
  }, [resourceFilter]);

  const submit = (e?: React.FormEvent) => {
    e?.preventDefault();
    if (!image || !registry) return;

    openSheet({
      kind: 'pull',
      payload: {
        repository: '',
        imageTag: image,
        registryName: registry.name,
      },
    });
  };

  return (
    <div ref={ref} className="mb-1">
      {open && (
        <div
          className={cn(
            'overflow-hidden transition-all duration-300 ease-in-out',
            'rounded-sm border p-1 px-2 bg-background',
            open ? 'opacity-100 ' : 'opacity-0 ',
          )}>
          <form onSubmit={submit} className="sm:flex flex flex-col gap-5 p-2">
            <div className="flex flex-col items-baseline">
              <div className="font-semibold text-sm">Pull Image</div>
              <div className="text-sm text-muted-foreground">
                {' '}
                Pull an image from a connected registry to make it available on this platform.
              </div>
            </div>

            <div className="flex flex-col sm:flex-row sm:items-baseline">
              <div className="flex-none w-full sm:w-36 text-sm font-medium">Registry</div>
              <ResourceSelector
                type={'Registry'}
                selected={registry}
                placeholder="Select Registry"
                onSelect={(item) => setRegistry(item)}
                className="text-sm"
              />
            </div>

            <div className="flex flex-col sm:flex-row sm:items-baseline">
              <div className="flex-none w-full sm:w-36 text-sm font-medium">Image</div>
              <Input
                placeholder="e.g. nginx:latest or my-app:1.0"
                value={image}
                onChange={(e) => setImage(e.target.value)}
                className="text-sm w-full max-w-[600px]"
              />
            </div>

            <div className="flex flex-col sm:flex-row sm:items-baseline">
              <div className="flex-none w-full sm:w-36 text-sm" />
              <Button type="submit" className="gap-2 text-sm" variant="outline" disabled={!image || !registry}>
                Pull image
                <Download className="w-4 h-4" />
              </Button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
}
