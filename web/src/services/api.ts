import { api } from "../lib/axios";
import type {
  UserCreate,
  UserResponse,
  Token,
  SubjectResponse,
  SubjectUpdate,
  ChapterResponse,
  ChapterUpdate,
  SlideResponse,
  SlideUpdate,
  SubjectDetailResponse,
  ChapterCreate,
} from "../types";

export const authApi = {
  register: async (data: UserCreate): Promise<UserResponse> => {
    const response = await api.post<UserResponse>("/api/auth/register", data);
    return response.data;
  },
  login: async (formData: URLSearchParams): Promise<Token> => {
    const response = await api.post<Token>("/api/auth/token", formData, {
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
    });
    return response.data;
  },
  refreshToken: async (refreshToken: string): Promise<Token> => {
    const response = await api.post<Token>("/api/auth/refresh", null, {
      params: { refresh_token: refreshToken },
    });
    return response.data;
  },

  me: async (): Promise<UserResponse> => {
    const response = await api.get<UserResponse>(`/api/auth/me`);
    return response.data;
  },
};

export const usersApi = {
  getUser: async (userId: number): Promise<UserResponse> => {
    const response = await api.get<UserResponse>(`/api/users/${userId}`);
    return response.data;
  },
  updateUser: async (data: FormData): Promise<UserResponse> => {
    const response = await api.patch<UserResponse>(`/api/users/me`, data, {
      headers: { "Content-Type": "multipart/form-data" },
    });
    return response.data;
  },
  deleteUser: async (): Promise<void> => {
    await api.delete(`/api/users/me`);
  },
  getUserSubjects: async (
    userId: number,
    skip = 0,
    limit = 100
  ): Promise<SubjectResponse[]> => {
    const response = await api.get<SubjectResponse[]>(
      `/api/users/${userId}/subjects`,
      {
        params: { skip, limit },
      }
    );
    return response.data;
  },
};

export const subjectsApi = {
  listSubjects: async (skip = 0, limit = 100): Promise<SubjectResponse[]> => {
    const response = await api.get<SubjectResponse[]>("/api/subjects/", {
      params: { skip, limit },
    });
    return response.data;
  },

  getSubject: async (subjectId: number): Promise<SubjectResponse> => {
    const response = await api.get<SubjectResponse>(
      `/api/subjects/${subjectId}`
    );
    return response.data;
  },
  uploadSubject: async (formData: FormData): Promise<SubjectResponse> => {
    const response = await api.post<SubjectResponse>(
      "/api/subjects/upload",
      formData,
      {
        headers: { "Content-Type": "multipart/form-data" },
      }
    );
    return response.data;
  },
  getSubjectChapters: async (
    subjectId: number
  ): Promise<SubjectDetailResponse> => {
    const response = await api.get<SubjectDetailResponse>(
      `/api/subjects/${subjectId}/chapters`
    );
    return response.data;
  },
  updateSubject: async (
    subjectId: number,
    data: SubjectUpdate
  ): Promise<SubjectResponse> => {
    const response = await api.patch<SubjectResponse>(
      `/api/subjects/${subjectId}`,
      data
    );
    return response.data;
  },
  deleteSubject: async (subjectId: number): Promise<void> => {
    await api.delete(`/api/subjects/${subjectId}`);
  },
};

export const chaptersApi = {
  createChapter: async (data: ChapterCreate): Promise<ChapterResponse> => {
    const response = await api.post<ChapterResponse>(`/api/chapters`, data);
    return response.data;
  },

  updateChapter: async (
    chapterId: number,
    data: ChapterUpdate
  ): Promise<ChapterResponse> => {
    const response = await api.patch<ChapterResponse>(
      `/api/chapters/${chapterId}`,
      data
    );
    return response.data;
  },

  deleteChapter: async (chapterId: number): Promise<void> => {
    await api.delete(`/api/chapters/${chapterId}`);
  },
  generateSlides: async (chapterId: number): Promise<void> => {
    await api.post(`/api/chapters/${chapterId}/slides/generate`);
  },
  getChapterSlides: async (chapterId: number): Promise<SlideResponse[]> => {
    const response = await api.get<SlideResponse[]>(
      `/api/chapters/${chapterId}/slides`
    );
    return response.data;
  },
};

export const slidesApi = {
  updateSlide: async (
    slideId: number,
    data: SlideUpdate
  ): Promise<SlideResponse> => {
    const response = await api.patch<SlideResponse>(
      `/api/slides/${slideId}`,
      data
    );
    return response.data;
  },
  deleteSlide: async (slideId: number): Promise<void> => {
    await api.delete(`/api/slides/${slideId}`);
  },
};
