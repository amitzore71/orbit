'use client';
import { createContext, useContext } from 'react';
import type { Density, MotionLevel } from './protocol';

export const ExperienceContext = createContext<{motion: MotionLevel; density: Density; visible: boolean}>({motion: 'standard', density: 'comfortable', visible: true});
export const useExperience = () => useContext(ExperienceContext);

// Navigation state belongs to this workspace instance, never to the database or localStorage.
export interface WorkspaceMemory {values: Map<string, unknown>}
export const WorkspaceContext = createContext<WorkspaceMemory>({values: new Map()});
