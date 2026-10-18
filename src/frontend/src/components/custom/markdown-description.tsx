import { ComponentProps } from 'react';
import { CircleHelp } from 'lucide-react';
import Markdown from 'react-markdown';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';

const examples = [
  ['Bold', '**Important**'],
  ['Italic', '*Optional*'],
  ['Link', '[Runbook](https://example.com)'],
  ['Inline code', '`docker ps`'],
  ['Bullet list', '- First item\n- Second item'],
  ['Numbered list', '1. First step\n2. Second step'],
  ['Quote', '> Maintenance note', 'Start a new line with > followed by a space.'],
];

export function MarkdownDescription({ children, className }: { children: string; className?: string }) {
  return (
    <div
      className={cn(
        'min-w-0 space-y-2 text-sm leading-relaxed [overflow-wrap:anywhere] [&_p]:whitespace-pre-wrap [&_ul]:list-disc [&_ul]:pl-5 [&_ol]:list-decimal [&_ol]:pl-5 [&_li>p]:my-1 [&_blockquote]:border-l-2 [&_blockquote]:pl-3 [&_code]:rounded-sm [&_code]:bg-muted [&_code]:px-1 [&_code]:font-mono [&_code]:text-[0.9em] [&_pre]:overflow-x-auto [&_pre]:rounded-md [&_pre]:bg-muted [&_pre]:p-2',
        className,
      )}>
      <Markdown
        allowedElements={['p', 'strong', 'em', 'a', 'ul', 'ol', 'li', 'blockquote', 'code', 'pre', 'br']}
        unwrapDisallowed
        components={{
          a: ({ href, children, title }) =>
            href ? (
              <a
                href={href}
                title={title}
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary underline underline-offset-2 hover:opacity-80">
                {children}
              </a>
            ) : (
              <span>{children}</span>
            ),
        }}>
        {children}
      </Markdown>
    </div>
  );
}

export function MarkdownDescriptionEditor({ className, ...props }: ComponentProps<typeof Textarea>) {
  return (
    <div className="relative min-w-0 flex-1">
      <Textarea {...props} className={cn('resize-y pr-10 pb-9', className)} />
      <div className="absolute right-3 bottom-2">
        <Dialog>
          <Tooltip>
            <TooltipTrigger asChild>
              <DialogTrigger asChild>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  className="size-6 text-muted-foreground"
                  aria-label="Markdown formatting help">
                  <CircleHelp className="size-3.5" />
                </Button>
              </DialogTrigger>
            </TooltipTrigger>
            <TooltipContent>Markdown formatting help</TooltipContent>
          </Tooltip>
          <DialogContent className="max-h-[85vh] overflow-y-auto">
            <DialogHeader>
              <DialogTitle>Formatting descriptions</DialogTitle>
              <DialogDescription>
                Type Markdown in your description. Formatting appears when you save.
              </DialogDescription>
            </DialogHeader>
            <Table className="table-fixed">
              <TableHeader>
                <TableRow>
                  <TableHead scope="col" className="w-28">
                    Format
                  </TableHead>
                  <TableHead scope="col">What to type</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {examples.map(([label, syntax, note]) => (
                  <TableRow key={label}>
                    <TableHead scope="row" className="text-sm font-normal tracking-normal whitespace-normal">
                      {label}
                    </TableHead>
                    <TableCell className="whitespace-normal [overflow-wrap:anywhere]">
                      <code className="whitespace-pre-wrap text-xs">{syntax}</code>
                      {note && <p className="mt-1 text-xs text-muted-foreground">{note}</p>}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
            <p className="text-sm text-muted-foreground">
              Escape formatting with a backslash, such as {'\\*literal asterisks\\*'}. Images and HTML formatting are
              not supported.
            </p>
          </DialogContent>
        </Dialog>
      </div>
    </div>
  );
}
