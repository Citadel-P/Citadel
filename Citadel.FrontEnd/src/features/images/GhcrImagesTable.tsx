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
import { useGETExternalRepositories } from './hooks/useGETExternalRepositories';
import Loader from '@/components/ui/loader';
import { GitHubCrPackageVersion, IImageRepositoryGitHubPackageResponse } from '@/api/_generated';
import { useGETPackageVersions } from './hooks/useGETPackageVersions';
import { useContextSelector } from 'use-context-selector';
import { ImagesContext } from './ImagesProvider';
import { fromNow } from '@/lib/dayjs.helper';
import { Badge } from '@/components/ui/badge';
import { truncate } from '@/lib/truncate';
import PullProgressSheetContent from './PullProgressSheetContent';
import { Sheet } from '@/components/ui/sheet';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { useSheetState } from './hooks/useSheetState';
import { PullImageBadge } from '@/components/ui/PullImageBadge';

// Column helpers
const packageColumnHelper = createColumnHelper<IImageRepositoryGitHubPackageResponse>();
const versionColumnHelper = createColumnHelper<GitHubCrPackageVersion>();

// Nested table component
function NestedVersionsTable({ ghPackage }: { ghPackage: IImageRepositoryGitHubPackageResponse }) {
  const selectedRegistry = useContextSelector(ImagesContext, (v) => v?.selectedRegistry);
  const { isLoading, data } = useGETPackageVersions(selectedRegistry?.name, ghPackage?.name);
  const { sheetState, openSheet, closeSheet } = useSheetState<GitHubCrPackageVersion>();

  const versionColumns = useMemo(
    () => [
      versionColumnHelper.accessor('name', {
        header: 'Version',
        cell: (info) => <VersionRow version={info.getValue() ?? ''} />,
      }),
      versionColumnHelper.accessor('htmlUrl', {
        header: 'Url',
        cell: (info) => (
          <a
            className="hover:underline text-blue-600 text-[13px]"
            target="_blank"
            rel="noreferrer"
            href={info.row.original.htmlUrl ?? ''}>
            {info.row.original.id}
          </a>
        ),
      }),
      {
        header: 'Tags',
        cell: ({ row }: { row: Row<GitHubCrPackageVersion> }) =>
          row.original.metadata?.container?.tags?.map((s) => (
            <Badge className="mr-1 text-[13px] font-normal" variant="outline" key={s}>
              {truncate(s, 15, 'left')}
            </Badge>
          )),
      },
      versionColumnHelper.accessor('createdAt', {
        header: 'Created At',
        cell: (info) => <span className="text-[13px]">{fromNow(new Date(info.getValue() ?? 0 * 1000).getTime())}</span>,
      }),
      versionColumnHelper.accessor('updatedAt', {
        header: 'Updated At',
        cell: (info) => <span className="text-[13px]">{fromNow(new Date(info.getValue() ?? 0 * 1000).getTime())}</span>,
      }),
      {
        id: 'select',
        cell: ({ row }: { row: Row<GitHubCrPackageVersion> }) => (
          <PullImageBadge onClick={() => openSheet(row.original)} className="group-hover/versionrow:visible" />
        ),
      },
    ],
    [openSheet],
  );

  const versionsTable = useReactTable({
    data: data?.data ?? [],
    columns: versionColumns,
    getCoreRowModel: getCoreRowModel(),
  });

  if (isLoading) return <Loader />;

  return (
    <>
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
              <TableCell colSpan={versionColumns.length} className="h-24 text-center">
                No results found.
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
      {sheetState.image && (
        <Sheet open={sheetState.isOpen} onOpenChange={(open) => (open ? openSheet(sheetState.image!) : closeSheet())}>
          <PullProgressSheetContent
            sheetProps={{ repository: ghPackage.name ?? '', imageTag: sheetState.image.name ?? '' }}
          />
        </Sheet>
      )}
    </>
  );
}

// Reusable VersionRow Component
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

export default function GhcrImagesTable({ registryName }: { registryName: string }) {
  const { isLoading, data } = useGETExternalRepositories(registryName);
  const [expanded, setExpanded] = useState<ExpandedState>({});
  const columns = useMemo(
    () => [
      packageColumnHelper.display({
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
      packageColumnHelper.accessor('name', {
        cell: (info) => info.getValue(),
        header: 'Package Name',
      }),
      packageColumnHelper.accessor('url', {
        cell: (info) => (
          <a
            className="hover:underline text-blue-600 "
            target="_blank"
            rel="noreferrer"
            href={info.row.original.htmlUrl ?? ''}>
            {info.row.original.htmlUrl}
          </a>
        ),
        header: 'Url',
      }),
      packageColumnHelper.accessor('createdAt', {
        cell: (info) => fromNow(new Date(info.getValue() ?? 0 * 1000).getTime()),
        header: 'Created At',
      }),
      packageColumnHelper.accessor('updatedAt', {
        cell: (info) => fromNow(new Date(info.getValue() ?? 0 * 1000).getTime()),
        header: 'Updated At',
      }),
    ],
    [],
  );

  const table = useReactTable({
    data: (data?.data ?? []) as IImageRepositoryGitHubPackageResponse[],
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
                    <NestedVersionsTable ghPackage={row.original} />
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
