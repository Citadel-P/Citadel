import { NewTag, TagPatch, TagView } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from '@/components/ui/command';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { useMutate } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { Check, ChevronsUpDown, LoaderCircle } from 'lucide-react';
import { useMemo, useState } from 'react';
import { toast } from 'sonner';
import { invalidateTagQueries } from './actions';
import { DEFAULT_TAG_COLOR, getTagColorOption, TAG_COLOR_OPTIONS } from './tag-colors';

type TagDraft = NewTag;

const emptyDraft = (): TagDraft => ({
  name: '',
  color: DEFAULT_TAG_COLOR.value,
});

export const TagCreateDialog = ({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) => {
  return <TagFormDialog mode="create" open={open} onOpenChange={onOpenChange} />;
};

export function TagFormDialog({
  mode,
  tag,
  open,
  onOpenChange,
}: {
  mode: 'create' | 'edit';
  tag?: TagView;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const queryClient = useQueryClient();
  const createTag = useMutate('createTag');
  const patchTag = useMutate('patchTag');
  const initialDraft = useMemo(
    () =>
      tag ? { name: tag.name, color: getTagColorOption(tag.color)?.value ?? DEFAULT_TAG_COLOR.value } : emptyDraft(),
    [tag],
  );
  const draftKey = `${mode}:${tag?.id ?? 'new'}:${open ? 'open' : 'closed'}`;
  const [draftState, setDraftState] = useState(() => ({ key: draftKey, draft: initialDraft }));
  const draft = draftState.key === draftKey ? draftState.draft : initialDraft;
  const setDraft = (update: (prev: TagDraft) => TagDraft) => {
    setDraftState({ key: draftKey, draft: update(draft) });
  };

  const isSaving = createTag.isPending || patchTag.isPending;
  const isSaveDisabled = isSaving || !draft.name.trim() || !getTagColorOption(draft.color);
  const title = mode === 'create' ? 'Create Tag' : 'Edit Tag';

  const save = async () => {
    if (isSaveDisabled) return;

    if (mode === 'create') {
      await createTag.mutateAsync({ data: normalizeDraft(draft) } as any);
      toast.success('Tag created');
    } else if (tag) {
      const payload: TagPatch = normalizeDraft(draft);
      await patchTag.mutateAsync({ id: tag.id, data: payload } as any);
      toast.success('Tag updated');
    }

    await invalidateTagQueries(queryClient);
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        <div className="grid gap-4 py-2">
          <div className="grid gap-1.5">
            <Label htmlFor="tag-name">Name</Label>
            <Input
              id="tag-name"
              value={draft.name}
              placeholder="production"
              onChange={(event) => setDraft((prev) => ({ ...prev, name: event.target.value }))}
            />
          </div>
          <ColorField value={draft.color} onChange={(color) => setDraft((prev) => ({ ...prev, color }))} />
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isSaving}>
            Cancel
          </Button>
          <Button onClick={save} disabled={isSaveDisabled}>
            Save
            {isSaving && <LoaderCircle className="size-3.5 animate-spin" />}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function ColorField({ value, onChange }: { value: string; onChange: (color: string) => void }) {
  const [open, setOpen] = useState(false);
  const selected = getTagColorOption(value) ?? DEFAULT_TAG_COLOR;

  return (
    <div className="grid gap-1.5">
      <Label htmlFor="tag-color">Color</Label>
      <Popover open={open} onOpenChange={setOpen} modal>
        <PopoverTrigger asChild>
          <Button
            id="tag-color"
            type="button"
            variant="outline"
            role="combobox"
            aria-expanded={open}
            className="h-10 w-full justify-between font-normal">
            <ColorOptionLabel color={selected.value} label={selected.label} />
            <ChevronsUpDown className="size-4 opacity-50" />
          </Button>
        </PopoverTrigger>
        <PopoverContent align="start" className="w-(--radix-popover-trigger-width) p-0 bg-background">
          <Command filter={colorSearchFilter}>
            <CommandInput placeholder="Search colors..." />
            <CommandList className="max-h-72 overscroll-contain" onWheel={(event) => event.stopPropagation()}>
              <CommandEmpty>No colors found.</CommandEmpty>
              <CommandGroup>
                {TAG_COLOR_OPTIONS.map((option) => (
                  <CommandItem
                    key={`${option.family}-${option.shade}`}
                    value={`${option.label} ${option.family} ${option.shade}`}
                    onSelect={() => {
                      onChange(option.value);
                      setOpen(false);
                    }}>
                    <ColorOptionLabel color={option.value} label={option.label} />
                    {option.value === selected.value && <Check className="ml-auto size-4" />}
                  </CommandItem>
                ))}
              </CommandGroup>
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>
    </div>
  );
}

function ColorOptionLabel({ color, label }: { color: string; label: string }) {
  return (
    <span className="flex min-w-0 items-center gap-2">
      <span className="size-3 shrink-0 rounded-full border border-border" style={{ backgroundColor: color }} />
      <span className="truncate">{label}</span>
    </span>
  );
}

const colorSearchFilter = (value: string, search: string) => {
  if (!search.trim()) return 1;
  return value.toLowerCase().includes(search.toLowerCase()) ? 1 : 0;
};

function normalizeDraft(draft: TagDraft): NewTag {
  return {
    name: draft.name.trim(),
    color: draft.color.trim(),
  };
}
