import { RegularResourceComponents } from '@/pages/types';
import { getTaskName } from '@/lib/utils';
import { ListTodo } from 'lucide-react';
import { useTasksGroup } from './hooks/useTasksGroup';
import { TasksTable } from './table';

export const TaskComponents: RegularResourceComponents = {
  Icon: ListTodo,
  header: {
    title: 'Tasks',
    subtitle: 'Current scheduler-owned tasks. Tasks are read-only.',
    showAdd: false,
    showSearch: true,
  },
  Content: ({ items, isLoading }) => <TasksTable items={items} isLoading={isLoading} />,
  useData: (platformId) => {
    const { items, isLoading } = useTasksGroup(platformId);
    return { items, isLoading, capabilities: undefined };
  },
  filterItems: (items, search) => {
    const value = search.trim().toLowerCase();
    return value
      ? items.filter(
          (item) =>
            getTaskName(item).toLowerCase().includes(value) ||
            item.serviceName?.toLowerCase().includes(value) ||
            item.nodeHostname?.toLowerCase().includes(value) ||
            item.id?.toLowerCase().includes(value),
        )
      : items;
  },
};
