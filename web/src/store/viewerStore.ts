import { create } from 'zustand';

type ViewMode = 'vertical' | 'horizontal';

interface ViewerState {
  activeChapterId: string | null;
  viewMode: ViewMode;
  currentHorizontalIndex: number;

  setActiveChapterId: (id: string | null) => void;
  setViewMode: (mode: ViewMode) => void;
  setCurrentHorizontalIndex: (index: number) => void;
  reset: () => void;
}

export const useViewerStore = create<ViewerState>((set) => ({
  activeChapterId: null,
  viewMode: 'vertical',
  currentHorizontalIndex: 0,

  setActiveChapterId: (id) => set({ activeChapterId: id }),
  setViewMode: (mode) => set({ viewMode: mode }),
  setCurrentHorizontalIndex: (index) => set({ currentHorizontalIndex: index }),
  reset: () => set({ 
    activeChapterId: null, 
    viewMode: 'vertical', 
    currentHorizontalIndex: 0 
  }),
}));
