export const DocType = {
  BOOK: 'BOOK',
  REPORT: 'REPORT'
} as const;
export type DocType = typeof DocType[keyof typeof DocType];

export const Tab = {
  FILES: 'FILES',
  UPLOAD: 'UPLOAD',
  PROFILE: 'PROFILE'
} as const;
export type Tab = typeof Tab[keyof typeof Tab];

export interface User {
  id: string;
  name: string;
  email: string;
  avatar: string;
}

export interface Slide {
  id?: number; // Added for API integration
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
  file?: File; // Added for upload
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
  slideId?: number; // Added for API integration
  slide: Slide;
}

// API Types

export interface Token {
  access_token: string;
  refresh_token: string;
  token_type: string;
}

export interface UserCreate {
  full_name: string;
  email: string;
  password: string;
}

export interface UserUpdate {
  full_name?: string | null;
  email?: string | null;
  dp?: string | null;
}

export interface UserResponse {
  full_name: string;
  email: string;
  id: number;
  dp:string
  is_active: boolean;
}

export interface SubjectResponse {
  title: string;
  is_public: boolean;
  type: 'book' | 'report';
  id: number;
  user_id: number;
  file_path: string;
  processing_status: string;
  created_at: string;
  updated_at?: string | null;
}

export interface SubjectUpdate {
  title?: string | null;
  is_public?: boolean | null;
}

export interface LessonResponse {
  title: string;
  page_start: number;
  page_end: number;
  order_index: number;
  id: number;
  subject_id: number;
  created_at: string;
  updated_at?: string | null;
}

export interface LessonUpdate {
  title?: string | null;
  page_start?: number | null;
  page_end?: number | null;
  order_index?: number | null;
}

export interface SlideResponse {
  title: string;
  points: string[];
  explanation: string;
  order_index: number;
  id: number;
  lesson_id: number;
  voice_url?: string | null;
  created_at: string;
  updated_at?: string | null;
}

export interface SlideUpdate {
  title?: string | null;
  points?: string[] | null;
  explanation?: string | null;
  order_index?: number | null;
  voice_url?: string | null;
}