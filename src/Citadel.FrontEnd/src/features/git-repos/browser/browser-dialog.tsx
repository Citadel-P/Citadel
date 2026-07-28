import { GitRepositoryView } from '@/api/generated/api.types';
import { ActionButton } from '@/components/custom/action-with-dialog';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { ButtonActionComponent } from '@/pages/types';
import { useQueryClient } from '@tanstack/react-query';
import { FolderGit2 } from 'lucide-react';
import { useState } from 'react';
import { RepositoryBrowser } from './repository-browser';

export const GitRepositoryBrowseAction: ButtonActionComponent<GitRepositoryView> = ({ resource }) => {
  const [open, setOpen] = useState(false);
  const queryClient = useQueryClient();

  const handleOpenChange = (nextOpen: boolean) => {
    setOpen(nextOpen);
    if (nextOpen) return;

    queryClient.removeQueries({
      predicate: (query) => {
        const args = query.queryKey[1] as { id?: string } | string | undefined;
        return (
          (query.queryKey[0] === 'getGitRepositoryFileContent' &&
            typeof args === 'object' &&
            args?.id === resource.id) ||
          (query.queryKey[0] === 'git-repository-browser' && args === resource.id)
        );
      },
    });
  };

  return (
    <>
      <ActionButton
        title="Browse Repo"
        iconPosition="left"
        variant="outline"
        icon={<FolderGit2 className="size-4" />}
        onClick={() => setOpen(true)}
      />
      <Dialog open={open} onOpenChange={handleOpenChange}>
        <DialogContent
          className="flex h-[90dvh] max-h-[900px] w-[calc(100vw-1rem)] max-w-[1200px] flex-col gap-0 overflow-hidden p-0 sm:max-w-[1200px]"
          overlayClassName="backdrop-blur-[2px]">
          <DialogHeader className="border-b px-4 py-3 pr-12">
            <DialogTitle>Browse repository</DialogTitle>
            <DialogDescription>{resource.name}</DialogDescription>
          </DialogHeader>
          <div className="min-h-0 flex-1 p-2 sm:p-4">
            {open && <RepositoryBrowser repository={resource} className="h-full min-h-0" />}
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
};
