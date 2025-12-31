import { Button } from '@/components/ui/button';
import { Download, Minus, Plus } from 'lucide-react';
import { useInlineSubHeader, useResourceFilter, useTaskSheet } from '@/lib/atoms';
import { useEffect, useRef, useState } from 'react';
import autoAnimate from '@formkit/auto-animate';
import { cn } from '@/lib/utils';
import { Input } from '@/components/ui/input';
import { RegistryView } from '@/api/generated/api.types';
import { ResourceSelectorField } from '@/components/custom/common';
import { AlertMessage } from '@/components/custom/alert-message';

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
        imageTag: image,
        registryId: registry.id,
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
              <ResourceSelectorField
                type={'Registry'}
                selected={registry}
                onSelect={setRegistry}
                placeholder="Select Registry"
                className="text-sm"
              />
            </div>

            <div className="flex flex-col sm:flex-row sm:items-baseline">
              <div className="flex-none w-full sm:w-36 text-sm font-medium">Image</div>
              <Input
                placeholder="e.g. nginx:latest or my-app:1.0"
                value={image}
                onChange={(e) => setImage(e.target.value)}
                className="text-sm w-full max-w-[400px]"
              />
            </div>

            <div className="flex flex-col sm:flex-row sm:items-baseline">
              <div className="flex-none w-full sm:w-36 text-sm" />
              <Button type="submit" className="gap-2 text-sm" variant="outline" disabled={!image || !registry}>
                Pull image
                <Download className="w-4 h-4" />
              </Button>
            </div>

            <AlertMessage type="info" title="">
              <div className="flex flex-row gap-1">
                <div className="font-semibold text-sm">Tip:</div>
                <div className="font-normal">
                  Looking to run containers? Create a Deployment (single) or Stack (multiple) instead.
                </div>
              </div>
            </AlertMessage>
          </form>
        </div>
      )}
    </div>
  );
}
