import { Button } from '@/components/ui/button';
import { Download, Minus, Plus } from 'lucide-react';
import { useInlineSubHeader, useTaskSheet } from '@/lib/atoms';
import { useState } from 'react';
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

  const handleRegistrySelect = (newRegistry: RegistryView | undefined) => {
    setRegistry(newRegistry);
  };

  const submit = (e?: React.SubmitEvent) => {
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
    <div className="mb-1">
      <div
        className={cn(
          'grid transition-all duration-300 ease-in-out',
          open ? 'grid-rows-[1fr] opacity-100' : 'grid-rows-[0fr] opacity-0 invisible',
        )}>
        <div className="overflow-hidden">
          <div className="rounded-sm border p-1 px-2 bg-background">
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
                  sourceType="Image"
                  targetType='Registry'
                  selected={registry}
                  onSelect={handleRegistrySelect}
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
                  className="text-sm w-full max-w-100"
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
                    Looking to run containers? Create a Deployment for a single container or a Stack for multiple
                    containers.
                  </div>
                </div>
              </AlertMessage>
            </form>
          </div>
        </div>
      </div>
    </div>
  );
}
