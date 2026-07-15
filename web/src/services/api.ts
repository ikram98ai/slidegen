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
  login: async (credentials: { email: string; password: string }): Promise<Token> => {
    const response = await api.post<Token>("/auth/token", credentials);
    return response.data;
  },
  refreshToken: async (refreshToken: string): Promise<Token> => {
    const response = await api.post<Token>("/auth/refresh", {
      refresh_token: refreshToken,
    });
    return response.data;
  },
};

export const usersApi = {
  getUser: async (userId: string): Promise<UserResponse> => {
    const response = await api.get<UserResponse>(`/users/${userId}`);
    return response.data;
  },

  getMe: async (): Promise<UserResponse> => {
    const response = await api.get<UserResponse>(`/users/me`);
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
    userId: string,
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
    const response = await api.get<SubjectResponse[]>("/subjects", {
      params: { skip, limit },
    });
    return response.data;
  },

  getSubject: async (subjectId: string): Promise<SubjectResponse> => {
    const response = await api.get<SubjectResponse>(`/subjects/${subjectId}`);
    return response.data;
  },
  uploadSubject: async (formData: FormData): Promise<SubjectResponse> => {
    const response = await api.post<SubjectResponse>(
      "/subjects",
      formData,
      {
        headers: { "Content-Type": "multipart/form-data" },
      }
    );
    return response.data;
  },
  getSubjectChapters: async (
    subjectId: string
  ): Promise<SubjectDetailResponse> => {
    const response = await api.get<SubjectDetailResponse>(
      `/subjects/${subjectId}/chapters`
    );
    return response.data;
  },
  updateSubject: async (
    subjectId: string,
    data: SubjectUpdate
  ): Promise<SubjectResponse> => {
    const response = await api.patch<SubjectResponse>(
      `/subjects/${subjectId}`,
      data
    );
    return response.data;
  },
  deleteSubject: async (subjectId: string): Promise<void> => {
    await api.delete(`/subjects/${subjectId}`);
  },
  reprocessSubject: async (subjectId: string): Promise<SubjectResponse> => {
    const response = await api.post<SubjectResponse>(
      `/subjects/${subjectId}/reprocess`
    );
    return response.data;
  },
};

export const chaptersApi = {
  createChapter: async (data: ChapterCreate): Promise<ChapterResponse> => {
    const response = await api.post<ChapterResponse>(`/chapters`, data);
    return response.data;
  },

  updateChapter: async (
    subjectId: string,
    chapterId: string,
    data: ChapterUpdate
  ): Promise<ChapterResponse> => {
    const response = await api.patch<ChapterResponse>(
      `/chapters/${subjectId}/${chapterId}`,
      data
    );
    return response.data;
  },

  deleteChapter: async (
    subjectId: string,
    chapterId: string
  ): Promise<void> => {
    await api.delete(`/chapters/${subjectId}/${chapterId}`);
  },
  generateSlides: async (
    subjectId: string,
    chapterId: string
  ): Promise<void> => {
    await api.post(`/chapters/${subjectId}/${chapterId}/slides/generate`);
  },
  getChapterSlides: async (
    chapterId: string
  ): Promise<SlideResponse[]> => {
    const response = await api.get<SlideResponse[]>(
      `/chapters/${chapterId}/slides`
    );
    return response.data;
  },
};

export const slidesApi = {
  updateSlide: async (
    chapterId: string,
    slideId: string,
    data: SlideUpdate
  ): Promise<SlideResponse> => {
    const response = await api.patch<SlideResponse>(`/chapters/${chapterId}/slides/${slideId}`, data);
    return response.data;
  },
  deleteSlide: async (
    chapterId: string,
    slideId: string
  ): Promise<void> => {
    await api.delete(`/chapters/${chapterId}/slides/${slideId}`);
  },
};
