import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { authApi, subjectsApi, lessonsApi, slidesApi, usersApi } from '../services/api';
import type { StoredSubject, UpdateSlideRequest, SubjectResponse } from '../types';
import { DocType } from '../types';
import { useAuthStore } from '../store/authStore';

// --- Auth Hooks ---

export const useLogin = () => {
  const login = useAuthStore(state => state.login);
  return useMutation({
    mutationFn: async (credentials: { email: string; password?: string }) => {
      const formData = new URLSearchParams();
      formData.append('username', credentials.email);
      formData.append('password', credentials.password || '');
      return await authApi.login(formData);
    },
    onSuccess: (_, variables) => {
      return login(variables.email, variables.password || '');
    },
  });
};

export const useRegister = () => {
  const login = useAuthStore(state => state.login);
  return useMutation({
    mutationFn: async (details: { name: string; email: string; password?: string }) => {
      return await authApi.register({ full_name: details.name, email: details.email, password: details.password || '' });
    },
    onSuccess: async (_, variables) => {
      // Auto login
      await login(variables.email, variables.password || '');
    },
  });
};

// --- Subject Hooks ---

const mapSubjectToStoredSubject = (subject: SubjectResponse): StoredSubject => {
  return {
    id: subject.id.toString(),
    title: subject.title,
    type: subject.type === 'book' ? DocType.BOOK : DocType.REPORT,
    uploadDate: new Date(subject.created_at),
    fileBase64: null, // Not returned by list
    chapters: [], // Fetched separately
    reportSlides: [], // Fetched separately
    isUserOwner: true, // Assuming list returns own subjects or public ones. API says "public or user's own".
    author: 'Unknown', // API doesn't return author name yet
    userId: subject.user_id.toString(),
  };
};

export const useSubjects = () => {
  return useQuery<StoredSubject[]>({
    queryKey: ['subjects'],
    queryFn: async () => {
      const subjects = await subjectsApi.listSubjects();
      return subjects.map(mapSubjectToStoredSubject);
    },
  });
};

export const useUserSubjects = (userId: number | undefined) => {
  return useQuery<StoredSubject[]>({
    queryKey: ['userSubjects', userId],
    queryFn: async () => {
      if (!userId) return [];
      const subjects = await usersApi.getUserSubjects(userId);
      return subjects.map(mapSubjectToStoredSubject);
    },
    enabled: !!userId,
  });
};

export const useSubjectDetails = (subjectId: string | null) => {
  return useQuery<StoredSubject | null>({
    queryKey: ['subject', subjectId],
    queryFn: async () => {
      if (!subjectId) return null;
      const id = parseInt(subjectId);
      
      const lessons = await subjectsApi.getSubjectLessons(id);
      
      // Now fetch slides for each lesson.
      const lessonsWithSlides = await Promise.all(lessons.map(async (lesson) => {
          const slides = await lessonsApi.getLessonSlides(lesson.id);
          return {
              ...lesson,
              slides
          };
      }));
      
      return {
          id: subjectId,
          title: "Loading...", // Placeholder if not found
          type: DocType.BOOK, // Default
          uploadDate: new Date(),
          chapters: lessonsWithSlides.map(l => ({
              id: l.id.toString(),
              title: l.title,
              description: "", // Lesson doesn't have description in API?
              slides: l.slides.map(s => ({
                  id: s.id,
                  title: s.title,
                  bullets: s.points,
                  explanation: s.explanation,
                  audioBase64: s.voice_url || undefined,
              }))
          })),
          reportSlides: [], // TODO: Handle report type
          isUserOwner: true,
          author: "",
      } as StoredSubject; 
    },
    enabled: !!subjectId,
  });
};

export const useAddSubject = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (doc: StoredSubject) => {
      // We need to upload the file.
      // The `doc` object has `file` (File object).
      if (!doc.file) throw new Error("No file to upload");
      
      const formData = new FormData();
      formData.append('title', doc.title);
      formData.append('type', doc.type === DocType.BOOK ? 'book' : 'report');
      formData.append('is_public', 'false');
      formData.append('file', doc.file);
      
      return await subjectsApi.uploadSubject(formData);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['subjects'] });
    },
  });
};

export const useUpdateSlide = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (payload: UpdateSlideRequest) => {
      const slideId = payload.slideId; 
      if (!slideId) throw new Error("Slide ID is missing");
      
      return await slidesApi.updateSlide(slideId, {
          title: payload.slide.title,
          points: payload.slide.bullets,
          explanation: payload.slide.explanation,
          voice_url: payload.slide.audioBase64
      });
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['subject', variables.subjectId] });
    },
  });
};