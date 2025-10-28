import React, { useState, useMemo } from 'react';
import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  useReactTable,
  type ExpandedState,
  getExpandedRowModel,
  Row,
} from '@tanstack/react-table';
import { CheckCheck, ChevronRight, Clipboard } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { cn } from '@/lib/utils';
import Loader from '@/components/ui/loader';
import { DockerHubTagView, IImageRepositoryDockerHubRepositoryResponse } from '@/api/generated/api.types';
import { fromNow } from '@/lib/dayjs.helper';
import { truncate } from '@/lib/truncate';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { byteTransform } from '@/lib/bytes.helper';
import { PullImageBadge } from '@/components/ui/PullImageBadge';
import { useRead } from '@/lib/hooks';
import { useTaskSheet } from '@/lib/atoms';

// Column helpers
const repositoryColumnHelper = createColumnHelper<IImageRepositoryDockerHubRepositoryResponse>();
const tagColumnHelper = createColumnHelper<DockerHubTagView>();

// Nested table component
function NestedImagesTable({
  dockerhubRepo,
  registryName,
}: {
  dockerhubRepo: IImageRepositoryDockerHubRepositoryResponse;
  registryName: string;
}) {
  const { isLoading, data } = useRead('getDockerHubRepositoryTags', {
    registryName,
    repositoryName: dockerhubRepo?.name,
  });
  const { open } = useTaskSheet('Image');

  const tagColumns = useMemo(
    () => [
      tagColumnHelper.accessor('name', {
        header: 'Tag',
        cell: (info) => info.getValue(),
      }),
      tagColumnHelper.accessor('image.digest', {
        header: 'Digest',
        cell: (info) => <VersionRow version={info.getValue() ?? ''} />,
      }),
      tagColumnHelper.accessor('lastPulled', {
        header: 'Last Pulled',
        cell: (info) => <span className="text-[13px]">{fromNow(new Date(info.getValue() ?? 0 * 1000).getTime())}</span>,
      }),
      tagColumnHelper.accessor('lastUpdated', {
        header: 'Last Pushed',
        cell: (info) => <span className="text-[13px]">{fromNow(new Date(info.getValue() ?? 0 * 1000).getTime())}</span>,
      }),
      {
        header: 'Os/Arch',
        cell: ({ row }: { row: Row<DockerHubTagView> }) => (
          <span className="text-[13px]">{row.original.image?.os + '/' + row.original.image?.architecture}</span>
        ),
      },
      tagColumnHelper.accessor('image.size', {
        header: 'Size',
        cell: (info) => <span className="text-[13px]">{byteTransform(info.getValue() ?? 0, 2)}</span>,
      }),
      {
        id: 'select',
        cell: ({ row }: { row: Row<DockerHubTagView> }) => (
          <PullImageBadge
            onClick={() =>
              open({
                kind: 'pull',
                payload: { repository: dockerhubRepo.name ?? '', imageTag: row.original.name ?? '', registryName },
              })
            }
            className="group-hover/versionrow:visible"
          />
        ),
      },
    ],
    [open, dockerhubRepo.name, registryName],
  );

  const versionsTable = useReactTable({
    data: data?.data ?? [],
    columns: tagColumns,
    getCoreRowModel: getCoreRowModel(),
  });

  if (isLoading) return <Loader />;

  return (
    <Table className="bg-background">
      <TableHeader>
        {versionsTable.getHeaderGroups().map((headerGroup) => (
          <TableRow key={headerGroup.id}>
            {headerGroup.headers.map((header) => (
              <TableHead key={header.id}>
                {header.isPlaceholder ? null : flexRender(header.column.columnDef.header, header.getContext())}
              </TableHead>
            ))}
          </TableRow>
        ))}
      </TableHeader>
      <TableBody>
        {versionsTable.getRowModel().rows.length ? (
          versionsTable.getRowModel().rows.map((row) => (
            <TableRow key={row.id} className="group/versionrow">
              {row.getVisibleCells().map((cell, index) => (
                <TableCell
                  key={cell.id}
                  className={cn({
                    'justify-items-center': index === row.getVisibleCells().length - 1,
                  })}>
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </TableCell>
              ))}
            </TableRow>
          ))
        ) : (
          <TableRow>
            <TableCell colSpan={tagColumns.length} className="h-24 text-center">
              No results found.
            </TableCell>
          </TableRow>
        )}
      </TableBody>
    </Table>
  );
}

const VersionRow = ({ version }: { version: string }) => {
  const [copiedWinCmd, copyWinCmdToClipboard] = useCopyToClipboard(5000);
  const v = version?.split(':').at(1) ?? '';
  return (
    <div className="flex gap-0.5 items-center">
      <div>{truncate(v, 12, 'right', true)}</div>
      <button
        className="rounded-full invisible group-hover/versionrow:visible ml-1 px-1.5 py-1.5 bg-foreground/5 hover:bg-foreground/10 text-sm font-semibold"
        onClick={() => copyWinCmdToClipboard(version ?? '')}>
        {copiedWinCmd ? <CheckCheck className="w-3 h-3 text-green-500" /> : <Clipboard className="w-3 h-3 " />}
      </button>
    </div>
  );
};

export default function PrivateDockerHubImagesTable({ registryName }: { registryName: string }) {
  const { isLoading, data } = useRead('getExternalRepositories', { registryName });
  const [expanded, setExpanded] = useState<ExpandedState>({});
  const columns = useMemo(
    () => [
      repositoryColumnHelper.display({
        id: 'expander',
        cell: ({ row }) => (
          <Button
            variant="ghost"
            size="icon"
            onClick={() => row.toggleExpanded()}
            aria-label={row.getIsExpanded() ? 'Collapse row' : 'Expand row'}
            className="h-8 w-8 p-0">
            <div className={cn('transition-transform duration-200', row.getIsExpanded() ? 'rotate-90' : '')}>
              <ChevronRight className="h-4 w-4" />
            </div>
          </Button>
        ),
        size: 50,
      }),
      repositoryColumnHelper.accessor('name', {
        cell: (info) => info.getValue(),
        header: 'Repository',
      }),
      repositoryColumnHelper.accessor('namespace', {
        cell: (info) => info.getValue(),
        header: 'Namespace',
      }),
      repositoryColumnHelper.accessor('pullCount', {
        cell: (info) => info.getValue(),
        header: 'Pull Count',
      }),
      repositoryColumnHelper.accessor('lastUpdated', {
        cell: (info) => fromNow(new Date(info.getValue() ?? 0 * 1000).getTime()),
        header: 'Last Pushed',
      }),
    ],
    [],
  );

  const table = useReactTable({
    data: (data?.data ?? []) as IImageRepositoryDockerHubRepositoryResponse[],
    columns,
    state: {
      expanded,
    },
    onExpandedChange: setExpanded,
    getRowCanExpand: () => true,
    getCoreRowModel: getCoreRowModel(),
    getExpandedRowModel: getExpandedRowModel(),
  });

  if (isLoading) return <Loader />;
  return (
    <Table>
      <TableHeader>
        {table.getHeaderGroups().map((headerGroup) => (
          <TableRow key={headerGroup.id}>
            {headerGroup.headers.map((header) => (
              <TableHead key={header.id} style={{ width: header.getSize() !== 150 ? header.getSize() : undefined }}>
                {header.isPlaceholder ? null : flexRender(header.column.columnDef.header, header.getContext())}
              </TableHead>
            ))}
          </TableRow>
        ))}
      </TableHeader>
      <TableBody>
        {table.getRowModel().rows.map((row) => (
          <React.Fragment key={row.id}>
            <TableRow>
              {row.getVisibleCells().map((cell) => (
                <TableCell key={cell.id}>{flexRender(cell.column.columnDef.cell, cell.getContext())}</TableCell>
              ))}
            </TableRow>
            {row.getIsExpanded() && (
              <TableRow className="bg-muted/20 hover:bg-muted/20">
                <TableCell colSpan={row.getVisibleCells().length} className="p-0">
                  <div className="overflow-hidden min-h-0 pl-10 bg-background">
                    <NestedImagesTable dockerhubRepo={row.original} registryName={registryName} />
                  </div>
                </TableCell>
              </TableRow>
            )}
          </React.Fragment>
        ))}
      </TableBody>
    </Table>
  );
}
