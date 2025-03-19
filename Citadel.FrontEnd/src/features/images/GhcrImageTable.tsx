import React, { useState } from 'react';
import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  useReactTable,
  type ExpandedState,
  getExpandedRowModel,
  Row,
} from '@tanstack/react-table';
import { ChevronRight } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { cn } from '@/lib/utils';
import { useGETExternalImages } from './hooks/useGETExternalImages';
import Loader from '@/components/ui/loader';
import { GhcrPackageVersion, IImageResponseGitHubPackageResponse } from '@/api/_generated';
import { useGETPackageVersions } from './hooks/useGETPackageVersions';
import { useContextSelector } from 'use-context-selector';
import { ImagesContext } from './ImagesProvider';
import { fromNow } from '@/lib/dayjs.helper';
import { Badge } from '@/components/ui/badge';
import { truncate } from '@/lib/truncate';
import PullProgressSheet from './PullProgressSheet';

// Column helpers
const packageColumnHelper = createColumnHelper<IImageResponseGitHubPackageResponse>();
const versionColumnHelper = createColumnHelper<GhcrPackageVersion>();

// Nested table component
function NestedVersionsTable({ ghPackage }: { ghPackage: IImageResponseGitHubPackageResponse }) {
  const selectedRegistry = useContextSelector(ImagesContext, (v) => v?.selectedRegistry);
  const { isLoading, data } = useGETPackageVersions(selectedRegistry?.name ?? undefined, ghPackage?.name ?? '');
  const versionColumns = React.useMemo(
    () => [
      versionColumnHelper.accessor('html_url', {
        header: 'Url',
        cell: (info) => (
          <a
            className="hover:underline text-blue-600 text-[13px]"
            target="_blank"
            rel="noreferrer"
            href={info.row.original.html_url ?? ''}>
            {info.row.original.id}{' '}
          </a>
        ),
      }),
      versionColumnHelper.accessor('name', {
        header: 'Version',
        cell: (info) => <span className="text-[13px]">{truncate(info.getValue() ?? '', 50, 'left')}</span>,
      }),
      {
        header: 'Tags',
        cell: ({ row }: { row: Row<GhcrPackageVersion> }) =>
          row.original.metadata?.container?.tags?.map((s) => (
            <Badge className="mr-1 text-[13px] font-normal" variant="outline" key={s}>
              {truncate(s, 15, 'left')}
            </Badge>
          )),
      },
      versionColumnHelper.accessor('created_at', {
        header: 'Created At',
        cell: (info) => <span className="text-[13px]">{fromNow(new Date(info.getValue() ?? 0 * 1000).getTime())}</span>,
      }),
      versionColumnHelper.accessor('updated_at', {
        header: 'Updated At',
        cell: (info) => <span className="text-[13px]">{fromNow(new Date(info.getValue() ?? 0 * 1000).getTime())}</span>,
      }),
      {
        id: 'select',
        cell: ({ row }: { row: Row<GhcrPackageVersion> }) => (
          <PullProgressSheet ghPackage={ghPackage} version={row.original} />
        ),
      },
    ],
    [],
  );

  const versionsTable = useReactTable({
    data: data?.data ?? [],
    columns: versionColumns,
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
              {row.getVisibleCells().map((cell) => (
                <TableCell className="justify-items-center" key={cell.id}>
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
  );
}

export default function GhcrImageTable({ registryName }: { registryName: string }) {
  const { isLoading, data } = useGETExternalImages(registryName);
  const [expanded, setExpanded] = useState<ExpandedState>({});

  const columns = React.useMemo(
    () => [
      packageColumnHelper.display({
        id: 'expander',
        cell: ({ row }) => {
          return (
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
          );
        },
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
            {info.row.original.htmlUrl}{' '}
          </a>
        ),
        header: 'Url',
      }),
      packageColumnHelper.accessor('createdAt', {
        cell: (info) => fromNow(new Date(info.getValue() ?? 0 * 1000).getTime()),
        header: 'Create dAt',
      }),
      packageColumnHelper.accessor('updatedAt', {
        cell: (info) => fromNow(new Date(info.getValue() ?? 0 * 1000).getTime()),
        header: 'updated at',
      }),
    ],
    [],
  );

  const table = useReactTable({
    data: (data?.data ?? []) as IImageResponseGitHubPackageResponse[],
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
                  <div
                    className={cn(
                      'grid overflow-hidden transition-all duration-300 ease-in-out',
                      row.getIsExpanded() ? 'grid-rows-[1fr] opacity-100' : 'grid-rows-[0fr] opacity-0',
                    )}>
                    <div className="overflow-hidden min-h-0 pl-10 bg-background">
                      <NestedVersionsTable ghPackage={row.original} />
                    </div>
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
