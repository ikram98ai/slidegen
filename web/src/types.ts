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
  dp?: string | null;
}

export interface UserResponse {
  full_name: string;
  email: string;
  id: string;
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
  id: string;
  user_id: string;
  file_path: string;
  processing_status: string;
  processed_pages?: number | null;
  total_pages?: number | null;
  job_id?: string | null;
  processing_stage?: string | null;
  extract_prefix?: string | null;
  page_offset?: number | null;
  created_at: string;
  updated_at?: string | null;
}

export interface SubjectDetailResponse {
  title: string;
  is_public: boolean;
  type: DocType;
  id: string;
  user_id: string;
  file_path: string;
  processing_status: string;
  job_id?: string | null;
  processing_stage?: string | null;
  extract_prefix?: string | null;
  page_offset?: number | null;
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
  id: string;
  subject_id: string;
  processing_status?: string | null;
  processed_slides?: number | null;
  total_slides?: number | null;
  job_id?: string | null;
  package_key?: string | null;
  created_at: string;
  updated_at?: string | null;
}

export interface JobResponse {
  id: string;
  kind: string;
  status: string;
  stage: string;
  subject_id: string;
  chapter_id?: string | null;
  tenant_id?: string | null;
  processed_pages?: number | null;
  total_pages?: number | null;
  processed_slides?: number | null;
  total_slides?: number | null;
  chapters_done?: number | null;
  chapters_total?: number | null;
  extract_prefix?: string | null;
  page_offset?: number | null;
  error?: string | null;
  created_at: string;
  updated_at: string;
}

export interface ChapterCreate {
  subject_id: string;
  title: string;
  page_start: number;
  page_end: number;
  order_index: number
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
  id: string;
  chapter_id: string;
  voice_url?: string | null;
  created_at: string;
  updated_at?: string | null;
}

export interface SlideUpdate {
  title?: string | null;
  points?: string[] | null;
  explanation?: string | null;
  order_index?: number | null;
}

export interface ChapterEmbedResponse {
  embed_url: string;
  expires_in: number;
  package_key: string;
  scene_count: number;
}
