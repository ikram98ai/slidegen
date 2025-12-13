import { create } from 'zustand';
import { DocType } from '../types';

interface SubjectState {
  file: File | null;
  fileBase64: string | null;
  docType: DocType;
  isAnalyzing: boolean;
  error: string | null;

  setFile: (file: File | null) => void;
  setFileBase64: (base64: string | null) => void;
  setDocType: (type: DocType) => void;
  setIsAnalyzing: (isAnalyzing: boolean) => void;
  setError: (error: string | null) => void;
  reset: () => void;
}

export const useSubjectStore = create<SubjectState>((set) => ({
  file: null,
  fileBase64: null,
  docType: DocType.BOOK,
  isAnalyzing: false,
  error: null,

  setFile: (file) => set({ file }),
  setFileBase64: (base64) => set({ fileBase64: base64 }),
  setDocType: (type) => set({ docType: type }),
  setIsAnalyzing: (isAnalyzing) => set({ isAnalyzing }),
  setError: (error) => set({ error }),
  reset: () => set({ 
    file: null, 
    fileBase64: null, 
    docType: DocType.BOOK, 
    isAnalyzing: false, 
    error: null 
  }),
}));
