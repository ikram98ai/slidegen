import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { authApi, subjectsApi, slidesApi, usersApi, lessonsApi } from "../services/api";
import type {
  SlideUpdate,
  SubjectResponse,
  LessonResponse,
  SubjectCreate,
  SlideResponse,
} from "../types";
import { DocType } from "../types";
import { useAuthStore } from "../store/authStore";

// --- Auth Hooks ---

export const useLogin = () => {
  const login = useAuthStore((state) => state.login);
  return useMutation({
    mutationFn: async (credentials: { email: string; password?: string }) => {
      const formData = new URLSearchParams();
      formData.append("username", credentials.email);
      formData.append("password", credentials.password || "");
      return await authApi.login(formData);
    },
    onSuccess: (_, variables) => {
      return login(variables.email, variables.password || "");
    },
  });
};

export const useRegister = () => {
  const login = useAuthStore((state) => state.login);
  return useMutation({
    mutationFn: async (details: {
      name: string;
      email: string;
      password?: string;
    }) => {
      return await authApi.register({
        full_name: details.name,
        email: details.email,
        password: details.password || "",
      });
    },
    onSuccess: async (_, variables) => {
      // Auto login
      await login(variables.email, variables.password || "");
    },
  });
};

// --- Subject Hooks ---

export const useSubjects = () => {
  return useQuery<SubjectResponse[]>({
    queryKey: ["subjects"],
    queryFn: async () => {
      const subjects = await subjectsApi.listSubjects();
      return subjects;
    },
  });
};

export const useSubject = (subjectId:number | undefined) => {
  return useQuery<SubjectResponse>({
    queryKey: ["subjects"],
    queryFn: async () => {
      const subject = await subjectsApi.getSubject(subjectId);
      return subject;
    },
  });
};
export const useUserSubjects = (userId: number | undefined) => {
  return useQuery<SubjectResponse[]>({
    queryKey: ["userSubjects", userId],
    queryFn: async () => {
      if (!userId) return [];
      const subjects = await usersApi.getUserSubjects(userId);
      return subjects;
    },
    enabled: !!userId,
  });
};

export const useSubjectLessons = (subjectId: number | null) => {
  return useQuery<LessonResponse[]>({
    queryKey: ["subject", subjectId],
    queryFn: async () => {
      if (!subjectId) return [];

      const lessons = await subjectsApi.getSubjectLessons(subjectId);
      return lessons;
    },
    enabled: !!subjectId,
  });
};

export const useAddSubject = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (doc: SubjectCreate) => {
      // We need to upload the file.
      // The `doc` object has `file` (File object).
      if (!doc.file) throw new Error("No file to upload");

      const formData = new FormData();
      formData.append("title", doc.title);
      formData.append("type", doc.type === DocType.BOOK ? "book" : "report");
      formData.append("is_public", "false");
      formData.append("file", doc.file);

      return await subjectsApi.uploadSubject(formData);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["subjects"] });
    },
  });
};

export const useUpdateSubject = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (payload: {
      id: number;
      data: { title?: string; is_public?: boolean };
    }) => {
      return await subjectsApi.updateSubject(payload.id, payload.data);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["subjects"] });
    },
  });
};

export const useDeleteSubject = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: number) => {
      return await subjectsApi.deleteSubject(id);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["subjects"] });
    },
  });
};

export const useLessonSlides = (lessonId: number) => {
  return useQuery<SlideResponse[]>({
    queryKey: ["lessons", lessonId],
    queryFn: async () => {
      if (!lessonId) return [];

      const slides = await lessonsApi.getLessonSlides(lessonId);
      return slides;
    },
    enabled: !!lessonId,
  });
};


export const useUpdateSlide = () => {
  return useMutation({
    mutationFn: async (payload: { slideId: number; slide: SlideUpdate }) => {
      if (!payload.slideId) throw new Error("Slide ID is missing");

      return await slidesApi.updateSlide(payload.slideId, {
        title: payload.slide.title,
        points: payload.slide.points,
        explanation: payload.slide.explanation,
      });
    },
  });
};
