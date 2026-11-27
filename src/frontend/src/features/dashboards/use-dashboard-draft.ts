import { useReducer, useRef } from 'react';
import type { DashboardConfiguration } from '@/api/generated/api.types';

type Draft = { name: string; config: DashboardConfiguration };
type History = { past: Draft[]; current: Draft; future: Draft[] };
type Action = { type: 'change'; patch: Partial<Draft>; coalesce: boolean } | { type: 'undo' | 'redo' };

function reduce(state: History, action: Action): History {
  const { past, current, future } = state;
  if (action.type === 'undo') {
    const previous = past.at(-1);
    return previous ? { current: previous, past: past.slice(0, -1), future: [current, ...future] } : state;
  }
  if (action.type === 'redo') {
    return future.length ? { current: future[0], past: [...past, current], future: future.slice(1) } : state;
  }
  if (action.type !== 'change') return state;
  const next = { ...current, ...action.patch };
  if (JSON.stringify(next) === JSON.stringify(current)) return state;
  return { past: action.coalesce ? past : [...past.slice(-49), current], current: next, future: [] };
}

export function useDashboardDraft(initial: Draft) {
  const [state, dispatch] = useReducer(reduce, { past: [], current: initial, future: [] });
  const textField = useRef<Element | null>(null);
  const endEdit = () => {
    textField.current = null;
  };
  return {
    ...state.current,
    canUndo: state.past.length > 0,
    canRedo: state.future.length > 0,
    endEdit,
    change(patch: Partial<Draft>) {
      const el = document.activeElement;
      const field = el?.matches('textarea, input[type=text], input:not([type])') ? el : null;
      dispatch({ type: 'change', patch, coalesce: !!field && field === textField.current });
      textField.current = field;
    },
    undo() {
      endEdit();
      dispatch({ type: 'undo' });
    },
    redo() {
      endEdit();
      dispatch({ type: 'redo' });
    },
  };
}
