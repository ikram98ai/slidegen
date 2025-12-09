export enum DocType {
  BOOK = 'BOOK',
  REPORT = 'REPORT'
}

export enum Tab {
  FILES = 'FILES',
  UPLOAD = 'UPLOAD',
  PROFILE = 'PROFILE'
}

export interface User {
  id: string;
  name: string;
  email: string;
  avatar: string;
}

export interface Slide {
  title: string;
  bullets: string[];
  explanation: string; // Detailed text explanation for the slide
  audioBase64?: string; // Base64 encoded audio data (WAV format)
  isLoadingAudio?: boolean; // UI state for audio generation
}

export interface Chapter {
  id: string; // usually the chapter number or title
  title: string;
  description: string;
  slides?: Slide[]; // Optional because we might lazy load them
  isLoadingSlides?: boolean;
}

export interface StoredDocument {
  id: string;
  title: string;
  type: DocType;
  uploadDate: Date;
  fileBase64?: string | null; // Optional, might not exist for demo docs
  chapters: Chapter[];
  reportSlides: Slide[];
  isUserOwner: boolean; // True if uploaded by current user
  author: string;
  userId?: string;
}

export interface ProjectState {
  file: File | null;
  fileBase64: string | null;
  docType: DocType;
  isAnalyzing: boolean;
  chapters: Chapter[];
  reportSlides: Slide[];
  activeChapterId: string | null;
  error: string | null;
}

export interface AuthResponse {
  user: User;
  token: string;
}

export interface UpdateSlideRequest {
  documentId: string;
  chapterId?: string; // Optional, only for books
  slideIndex: number;
  slide: Slide;
}