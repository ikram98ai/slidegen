import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { UserResponse } from "../types";
import { authApi } from "../services/api";

interface AuthState {
  user: UserResponse | null;
  isAuthenticated: boolean;
  token: string | null;
  isLoading: boolean;
  error: string | null;

  login: (email: string, password: string) => Promise<void>;
  register: (name: string, email: string, password: string) => Promise<void>;
  logout: () => void;
  updateUser: (user: UserResponse) => void;
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      user: null,
      isAuthenticated: false,
      token: null,
      isLoading: false,
      error: null,

      login: async (email, password) => {
        set({ isLoading: true, error: null });
        try {
          const formData = new URLSearchParams();
          formData.append("username", email);
          formData.append("password", password);
          const { access_token, refresh_token } = await authApi.login(formData);

          localStorage.setItem("access_token", access_token);
          localStorage.setItem("refresh_token", refresh_token);

          const user = await authApi.me();
          set({ isAuthenticated: true, token: access_token, user: user });

          // Fetch user details immediately after login
        } catch (error: any) {
          console.error("Login failed:", error);
          set({ error: "Login failed. Please check your credentials." });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      register: async (name, email, password) => {
        set({ isLoading: true, error: null });
        try {
          await authApi.register({ full_name: name, email, password });
          // After register, we can auto-login
          await get().login(email, password);
        } catch (error: any) {
          console.error("Registration failed:", error);
          set({ error: "Registration failed. Please try again." });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      logout: () => {
        localStorage.removeItem("access_token");
        localStorage.removeItem("refresh_token");
        set({ user: null, isAuthenticated: false, token: null, error: null });
      },

      updateUser: (user: UserResponse) => {
        set({ user });
      },
    }),
    {
      name: "auth-storage",
      partialize: (state) => ({
        token: state.token,
        isAuthenticated: state.isAuthenticated,
        user: state.user,
      }),
    }
  )
);
