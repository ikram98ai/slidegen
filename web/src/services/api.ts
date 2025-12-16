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
    const response = await api.post<UserResponse>("/auth/register", data);
    return response.data;
  },
  login: async (formData: URLSearchParams): Promise<Token> => {
    const response = await api.post<Token>("/auth/token", formData, {
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
    });
    return response.data;
  },
  refreshToken: async (refreshToken: string): Promise<Token> => {
    const response = await api.post<Token>("/auth/refresh", null, {
      params: { refresh_token: refreshToken },
    });
    return response.data;
  },

  me: async (): Promise<UserResponse> => {
    const response = await api.get<UserResponse>(`/auth/me`);
    return response.data;
  },
};

export const usersApi = {
  getUser: async (userId: number): Promise<UserResponse> => {
    const response = await api.get<UserResponse>(`/users/${userId}`);
    return response.data;
  },
  updateUser: async (data: FormData): Promise<UserResponse> => {
    const response = await api.patch<UserResponse>(`/users/me`, data, {
      headers: { "Content-Type": "multipart/form-data" },
    });
    return response.data;
  },
  deleteUser: async (): Promise<void> => {
    await api.delete(`/users/me`);
  },
  getUserSubjects: async (
    userId: number,
    skip = 0,
    limit = 100
  ): Promise<SubjectResponse[]> => {
    const response = await api.get<SubjectResponse[]>(
      `/users/${userId}/subjects`,
      {
        params: { skip, limit },
      }
    );
    return response.data;
  },
};

export const subjectsApi = {
  listSubjects: async (skip = 0, limit = 100): Promise<SubjectResponse[]> => {
    const response = await api.get<SubjectResponse[]>("/subjects/", {
      params: { skip, limit },
    });
    return response.data;
  },

  getSubject: async (subjectId: number): Promise<SubjectResponse> => {
    const response = await api.get<SubjectResponse>(
      `/subjects/${subjectId}`
    );
    return response.data;
  },
  uploadSubject: async (formData: FormData): Promise<SubjectResponse> => {
    const response = await api.post<SubjectResponse>(
      "/subjects/upload",
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
      `/subjects/${subjectId}/chapters`
    );
    return response.data;
  },
  updateSubject: async (
    subjectId: number,
    data: SubjectUpdate
  ): Promise<SubjectResponse> => {
    const response = await api.patch<SubjectResponse>(
      `/subjects/${subjectId}`,
      data
    );
    return response.data;
  },
  deleteSubject: async (subjectId: number): Promise<void> => {
    await api.delete(`/subjects/${subjectId}`);
  },
};

export const chaptersApi = {
  createChapter: async (data: ChapterCreate): Promise<ChapterResponse> => {
    const response = await api.post<ChapterResponse>(`/chapters`, data);
    return response.data;
  },

  updateChapter: async (
    chapterId: number,
    data: ChapterUpdate
  ): Promise<ChapterResponse> => {
    const response = await api.patch<ChapterResponse>(
      `/chapters/${chapterId}`,
      data
    );
    return response.data;
  },

  deleteChapter: async (chapterId: number): Promise<void> => {
    await api.delete(`/chapters/${chapterId}`);
  },
  generateSlides: async (chapterId: number): Promise<void> => {
    await api.post(`/chapters/${chapterId}/slides/generate`);
  },
  getChapterSlides: async (chapterId: number): Promise<SlideResponse[]> => {
    const response = await api.get<SlideResponse[]>(
      `/chapters/${chapterId}/slides`
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
      `/slides/${slideId}`,
      data
    );
    return response.data;
  },
  deleteSlide: async (slideId: number): Promise<void> => {
    await api.delete(`/slides/${slideId}`);
  },
};
