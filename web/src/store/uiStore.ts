import { create } from 'zustand';
import { Tab } from '../types';

interface UIState {
  activeTab: Tab;
  showAuthModal: boolean;
  viewingDocId: string | null;
  
  setActiveTab: (tab: Tab) => void;
  setShowAuthModal: (show: boolean) => void;
  setViewingDocId: (id: string | null) => void;
  reset: () => void;
}

export const useUIStore = create<UIState>((set) => ({
  activeTab: Tab.FILES,
  showAuthModal: false,
  viewingDocId: null,

  setActiveTab: (tab) => set({ activeTab: tab }),
  setShowAuthModal: (show) => set({ showAuthModal: show }),
  setViewingDocId: (id) => set({ viewingDocId: id }),
  reset: () => set({ activeTab: Tab.FILES, showAuthModal: false, viewingDocId: null }),
}));
