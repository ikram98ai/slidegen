import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  authApi,
  subjectsApi,
  slidesApi,
  usersApi,
  chaptersApi,
} from "../services/api";
import type {
  SlideUpdate,
  SubjectResponse,
  SubjectCreate,
  SlideResponse,
  SubjectDetailResponse,
} from "../types";
import { DocType } from "../types";
import { useAuthStore } from "../store/authStore";

// --- Auth Hooks ---

export const useLogin = () => {
  const login = useAuthStore((state) => state.login);
  return useMutation({
    mutationFn: async (credentials: { email: string; password: string }) => {
      return await authApi.login(credentials);
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
    refetchInterval: (query) => {
      const data = query.state.data;
      if (data && data.some(s => s.processing_status === "processing")) {
        return 3000;
      }
      return false;
    },
  });
};

export const useSubject = (subjectId: string | undefined) => {
  return useQuery<SubjectResponse | null>({
    queryKey: ["subjects", subjectId],
    queryFn: async () => {
      if (!subjectId) return null;
      const subject = await subjectsApi.getSubject(subjectId);
      return subject;
    },
  });
};
export const useUserSubjects = (userId: string | undefined) => {
  return useQuery<SubjectResponse[]>({
    queryKey: ["userSubjects", userId],
    queryFn: async () => {
      if (!userId) return [];
      const subjects = await usersApi.getUserSubjects(userId);
      return subjects;
    },
    enabled: !!userId,
    refetchInterval: (query) => {
      const data = query.state.data;
      if (data && data.some(s => s.processing_status === "processing")) {
        return 3000;
      }
      return false;
    },
  });
};

export const useSubjectChapters = (subjectId: string | undefined) => {
  return useQuery<SubjectDetailResponse | null>({
    queryKey: ["subject", subjectId],
    queryFn: async () => {
      if (!subjectId) return null;

      const chapters = await subjectsApi.getSubjectChapters(subjectId);
      return chapters;
    },
    enabled: !!subjectId,
    refetchInterval: (query) => {
      const data = query.state.data;
      if (data && data.chapters && data.chapters.some(c => c.processing_status === "processing")) {
        return 3000;
      }
      return false;
    },
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
      id: string;
      data: { title?: string; is_public?: boolean };
    }) => {
      return await subjectsApi.updateSubject(payload.id, payload.data);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["subjects"] });
    },
  });
};

export const useReprocessSubject = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string) => {
      return await subjectsApi.reprocessSubject(id);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["subjects"] });
      queryClient.invalidateQueries({ queryKey: ["userSubjects"] });
    },
  });
};

export const useDeleteSubject = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string) => {
      return await subjectsApi.deleteSubject(id);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["subjects"] });
    },
  });
};

export const useChapterSlides = (chapterId: string, pollWhileGenerating = false) => {
  return useQuery<SlideResponse[]>({
    queryKey: ["chapters", chapterId],
    queryFn: async () => {
      if (!chapterId) return [];

      const slides = await chaptersApi.getChapterSlides(chapterId);
      return slides;
    },
    enabled: !!chapterId,
    // While slides are being generated, keep fetching so finished slides
    // stream in as the background job saves them.
    refetchInterval: pollWhileGenerating ? 3000 : false,
  });
};

export const useUpdateSlide = () => {
  return useMutation({
    mutationFn: async (payload: {
      chapterId: string;
      slideId: string;
      slide: SlideUpdate;
    }) => {
      if (!payload.chapterId)
        throw new Error("Chapter ID is missing");

      if (!payload.slideId)
        throw new Error("Slide ID is missing");

      return await slidesApi.updateSlide(payload.chapterId, payload.slideId, {
        title: payload.slide.title,
        points: payload.slide.points,
        explanation: payload.slide.explanation,
      });
    },
  });
};
