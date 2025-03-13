'use client';

import React, { useState } from 'react';
import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  useReactTable,
  type ExpandedState,
  getExpandedRowModel,
} from '@tanstack/react-table';
import { ChevronRight } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { cn } from '@/lib/utils';
import { useGETExternalImages } from './hooks/useGETExternalImages';
import Loader from '@/components/ui/loader';

// Types for our data structure
type PackageVersion = {
  id: string;
  name: string;
  status: 'not-started' | 'in-progress' | 'completed' | 'on-hold';
  startDate: string;
  endDate: string;
  priority: 'low' | 'medium' | 'high';
};

type Package = {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
  versions: PackageVersion[];
  url: string;
};

// Sample data
const data: Package[] = [
  {
    id: '1',
    name: 'Alex Johnson',
    createdAt: 'alex@example.com',
    updatedAt: 'Developer',
    url: 'Engineering',
    versions: [
      {
        id: 'p1',
        name: 'Website Redesign',
        status: 'in-progress',
        startDate: '2023-01-10',
        endDate: '2023-04-30',
        priority: 'high',
      },
      {
        id: 'p2',
        name: 'API Integration',
        status: 'not-started',
        startDate: '2023-05-01',
        endDate: '2023-06-15',
        priority: 'medium',
      },
    ],
  },
  {
    id: '2',
    name: 'Sam Taylor',
    createdAt: 'sam@example.com',
    updatedAt: 'Designer',
    url: 'Design',
    versions: [
      {
        id: 'p3',
        name: 'Mobile App UI',
        status: 'completed',
        startDate: '2022-10-01',
        endDate: '2023-01-20',
        priority: 'high',
      },
      {
        id: 'p4',
        name: 'Brand Guidelines',
        status: 'in-progress',
        startDate: '2023-02-01',
        endDate: '2023-03-31',
        priority: 'medium',
      },
      {
        id: 'p5',
        name: 'Marketing Materials',
        status: 'on-hold',
        startDate: '2023-03-15',
        endDate: '2023-05-30',
        priority: 'low',
      },
    ],
  },
  {
    id: '3',
    name: 'Jordan Lee',
    createdAt: 'jordan@example.com',
    updatedAt: 'Manager',
    url: 'Product',
    versions: [
      {
        id: 'p6',
        name: 'Q4 Roadmap',
        status: 'completed',
        startDate: '2022-09-01',
        endDate: '2022-10-15',
        priority: 'high',
      },
      {
        id: 'p7',
        name: 'Feature Prioritization',
        status: 'in-progress',
        startDate: '2023-01-05',
        endDate: '2023-02-28',
        priority: 'high',
      },
    ],
  },
];

// Column helpers
const personColumnHelper = createColumnHelper<Package>();
const projectColumnHelper = createColumnHelper<PackageVersion>();

// Nested table component
function NestedProjectsTable({ person }: { person: Package }) {
  const projectColumns = React.useMemo(
    () => [
      projectColumnHelper.accessor('name', {
        header: 'Project Name',
        cell: (info) => info.getValue(),
      }),
      projectColumnHelper.accessor('status', {
        header: 'Status',
        cell: (info) => <span>Not started</span>,
      }),
      projectColumnHelper.accessor('startDate', {
        header: 'Start Date',
        cell: (info) => info.getValue(),
      }),
      projectColumnHelper.accessor('endDate', {
        header: 'End Date',
        cell: (info) => info.getValue(),
      }),
      projectColumnHelper.accessor('priority', {
        header: 'Priority',
        cell: (info) => <span>PP</span>,
      }),
    ],
    [],
  );

  const projectsTable = useReactTable({
    data: person.versions,
    columns: projectColumns,
    getCoreRowModel: getCoreRowModel(),
  });

  return (
    <Table className="bg-background">
      <TableHeader>
        {projectsTable.getHeaderGroups().map((headerGroup) => (
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
        {projectsTable.getRowModel().rows.length ? (
          projectsTable.getRowModel().rows.map((row) => (
            <TableRow key={row.id}>
              {row.getVisibleCells().map((cell) => (
                <TableCell key={cell.id}>{flexRender(cell.column.columnDef.cell, cell.getContext())}</TableCell>
              ))}
            </TableRow>
          ))
        ) : (
          <TableRow>
            <TableCell colSpan={projectColumns.length} className="h-24 text-center">
              No projects found.
            </TableCell>
          </TableRow>
        )}
      </TableBody>
    </Table>
  );
}

export default function GhcrImageTable({registryName}: {registryName: string}) {
  const {isLoading, isSuccess, data: externalImagesData } = useGETExternalImages(registryName);
  const [expanded, setExpanded] = useState<ExpandedState>({});

  const columns = React.useMemo(
    () => [
      personColumnHelper.display({
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
        header: () => null,
        size: 50,
      }),
      personColumnHelper.accessor('name', {
        cell: (info) => info.getValue(),
        header: 'Package Name',
      }),
      personColumnHelper.accessor('url', {
        cell: (info) => info.getValue(),
        header: 'Url',
      }),
      personColumnHelper.accessor('createdAt', {
        cell: (info) => info.getValue(),
        header: 'Create dAt',
      }),
      personColumnHelper.accessor('updatedAt', {
        cell: (info) => info.getValue(),
        header: 'updated at',
      }),
    ],
    [],
  );

  const table = useReactTable({
    data,
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
                      <NestedProjectsTable person={row.original} />
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
