import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import api from '../lib/axios';
import type { StoredDocument, User, UpdateSlideRequest, Slide } from '../types';
import { useAuthStore } from '../store/authStore';

// --- Auth Hooks ---

export const useLogin = () => {
  const login = useAuthStore(state => state.login);
  return useMutation({
    mutationFn: async (credentials: { email: string; password?: string }) => {
      const { data } = await api.post('/auth/login', credentials);
      return data;
    },
    onSuccess: (data) => {
      login(data.user, data.token);
    },
  });
};

export const useRegister = () => {
  const login = useAuthStore(state => state.login);
  return useMutation({
    mutationFn: async (details: { name: string; email: string; avatar: string; password?: string }) => {
      const { data } = await api.post('/auth/register', details);
      return data;
    },
    onSuccess: (data) => {
      login(data.user, data.token);
    },
  });
};

// --- Document Hooks ---

export const useDocuments = () => {
  return useQuery<StoredDocument[]>({
    queryKey: ['documents'],
    queryFn: async () => {
      const { data } = await api.get('/documents');
      return data;
    },
  });
};

export const useAddDocument = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (doc: StoredDocument) => {
      const { data } = await api.post('/documents', doc);
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    },
  });
};

export const useUpdateSlide = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (payload: UpdateSlideRequest) => {
      await api.put('/documents/slide', payload);
      return payload;
    },
    onSuccess: (data) => {
      // Optimistic update or invalidation could happen here.
      // For simplicity, we invalidate 'documents'.
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    },
  });
};