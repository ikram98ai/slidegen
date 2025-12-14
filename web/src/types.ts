export const DocType = {
  BOOK: "book",
  REPORT: "report",
} as const;
export type DocType = (typeof DocType)[keyof typeof DocType];

export const Tab = {
  FILES: "FILES",
  UPLOAD: "UPLOAD",
  PROFILE: "PROFILE",
} as const;
export type Tab = (typeof Tab)[keyof typeof Tab];

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
  dp: string;
  is_active: boolean;
}

export interface SubjectCreate {
  title: string;
  file: File;
  type: DocType;
}

export interface SubjectResponse {
  title: string;
  is_public: boolean;
  type: DocType;
  id: number;
  user_id: number;
  file_path: string;
  processing_status: string;
  created_at: string;
  updated_at?: string | null;
}

export interface SubjectDetailResponse {
  title: string;
  is_public: boolean;
  type: DocType;
  id: number;
  user_id: number;
  file_path: string;
  processing_status: string;
  chapters?: ChapterResponse[];
  created_at: string;
  updated_at?: string | null;
}

export interface SubjectUpdate {
  title?: string | null;
  is_public?: boolean | null;
}

export interface ChapterResponse {
  title: string;
  page_start: number;
  page_end: number;
  order_index: number;
  id: number;
  subject_id: number;
  created_at: string;
  updated_at?: string | null;
}

export interface ChapterUpdate {
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
  chapter_id: number;
  voice_url?: string | null;
  created_at: string;
  updated_at?: string | null;
}

export interface SlideUpdate {
  id?: number;
  title?: string | null;
  points?: string[] | null;
  explanation?: string | null;
  order_index?: number | null;
}
