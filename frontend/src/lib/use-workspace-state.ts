'use client';
import { useContext, useState, type Dispatch, type SetStateAction } from 'react';
import { WorkspaceContext } from './experience';

export function useWorkspaceState<T>(key: string, initial: T): [T, Dispatch<SetStateAction<T>>] {
  const memory = useContext(WorkspaceContext);
  const [value, setValue] = useState<T>(() => memory.values.has(key) ? memory.values.get(key) as T : initial);
  const update: Dispatch<SetStateAction<T>> = next => {
    const previous = memory.values.has(key) ? memory.values.get(key) as T : initial;
    const resolved = typeof next === 'function' ? (next as (v: T) => T)(previous) : next;
    memory.values.set(key, resolved);
    setValue(resolved);
  };
  return [value, update];
}
