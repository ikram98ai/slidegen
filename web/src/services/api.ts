import { api } from '../lib/axios';
import type {
  UserCreate,
  UserResponse,
  Token,
  SubjectResponse,
  SubjectUpdate,
  LessonResponse,
  LessonUpdate,
  SlideResponse,
  SlideUpdate,
} from '../types';

export const authApi = {
  register: async (data: UserCreate): Promise<UserResponse> => {
    const response = await api.post<UserResponse>('/api/auth/register', data);
    return response.data;
  },
  login: async (formData: URLSearchParams): Promise<Token> => {
    const response = await api.post<Token>('/api/auth/token', formData, {
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    });
    return response.data;
  },
  refreshToken: async (refreshToken: string): Promise<Token> => {
    const response = await api.post<Token>('/api/auth/refresh', null, {
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
      headers: { 'Content-Type': 'multipart/form-data' },
    });
    return response.data;
  },
  deleteUser: async (): Promise<void> => {
    await api.delete(`/api/users/me`);
  },
  getUserSubjects: async (userId: number, skip = 0, limit = 100): Promise<SubjectResponse[]> => {
    const response = await api.get<SubjectResponse[]>(`/api/users/${userId}/subjects`, {
      params: { skip, limit },
    });
    return response.data;
  },
};

export const subjectsApi = {
  listSubjects: async (skip = 0, limit = 100): Promise<SubjectResponse[]> => {
    const response = await api.get<SubjectResponse[]>('/api/subjects/', {
      params: { skip, limit },
    });
    return response.data;
  },
  uploadSubject: async (formData: FormData): Promise<SubjectResponse> => {
    const response = await api.post<SubjectResponse>('/api/subjects/upload', formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
    });
    return response.data;
  },
  getSubjectLessons: async (subjectId: number): Promise<LessonResponse[]> => {
    const response = await api.get<LessonResponse[]>(`/api/subjects/${subjectId}/lessons`);
    return response.data;
  },
  updateSubject: async (subjectId: number, data: SubjectUpdate): Promise<SubjectResponse> => {
    const response = await api.patch<SubjectResponse>(`/api/subjects/${subjectId}`, data);
    return response.data;
  },
  deleteSubject: async (subjectId: number): Promise<void> => {
    await api.delete(`/api/subjects/${subjectId}`);
  },
};

export const lessonsApi = {
  updateLesson: async (lessonId: number, data: LessonUpdate): Promise<LessonResponse> => {
    const response = await api.patch<LessonResponse>(`/api/lessons/${lessonId}`, data);
    return response.data;
  },
  deleteLesson: async (lessonId: number): Promise<void> => {
    await api.delete(`/api/lessons/${lessonId}`);
  },
  generateSlides: async (lessonId: number): Promise<void> => {
    await api.post(`/api/lessons/${lessonId}/slides/generate`);
  },
  getLessonSlides: async (lessonId: number): Promise<SlideResponse[]> => {
    const response = await api.get<SlideResponse[]>(`/api/lessons/${lessonId}/slides`);
    return response.data;
  },
};

export const slidesApi = {
  updateSlide: async (slideId: number, data: SlideUpdate): Promise<SlideResponse> => {
    const response = await api.patch<SlideResponse>(`/api/slides/${slideId}`, data);
    return response.data;
  },
  deleteSlide: async (slideId: number): Promise<void> => {
    await api.delete(`/api/slides/${slideId}`);
  },
};
