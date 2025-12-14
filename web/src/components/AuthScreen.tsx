import React, { useState } from "react";
import { useLogin, useRegister } from "../hooks/useAppQueries";
import { BookOpen, Eye, EyeOff } from "lucide-react"; // Keep Eye and EyeOff for password toggle
import { Button } from "./ui/Button";

export const AuthScreen: React.FC = () => {
  const [isLogin, setIsLogin] = useState(true);

  // Form State
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [name, setName] = useState("");

  const loginMutation = useLogin();
  const registerMutation = useRegister();

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    loginMutation.mutate({ email, password });
  };

  const handleRegister = async (e: React.FormEvent) => {
    e.preventDefault();
    // Avatar is handled by backend default generation if empty
    registerMutation.mutate({ name, email, password });
  };

  const error = isLogin ? loginMutation.error : registerMutation.error;
  const isLoading = isLogin
    ? loginMutation.isPending
    : registerMutation.isPending;

  return (
    <div className="min-h-screen bg-apple-gray flex flex-col justify-center items-center p-6">
      <div className="w-full max-w-md">
        <div className="text-center mb-10">
          <div className="inline-flex items-center justify-center p-3 bg-black rounded-2xl mb-6 shadow-xl">
            <BookOpen className="text-white h-8 w-8" />{" "}
            {/* Changed Sparkles to BookOpen */}
          </div>
          <h1 className="text-3xl font-bold text-gray-900 tracking-tight">
            {isLogin ? "Welcome back." : "Create your account."}
          </h1>
          <p className="text-gray-500 mt-2 text-lg">
            {isLogin
              ? "Enter your credentials to access your library."
              : "Start your learning journey with Lumina."}
          </p>
        </div>

        <div className="bg-white rounded-4xl shadow-apple-xl p-8 md:p-10 border border-white/40">
          <form
            onSubmit={isLogin ? handleLogin : handleRegister}
            className="space-y-6"
          >
            {!isLogin && (
              <div>
                <label className="block text-sm font-semibold text-gray-700 mb-2">
                  Full Name
                </label>
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="w-full px-4 py-3 rounded-xl bg-gray-50 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 transition-all outline-none"
                  placeholder="John Doe"
                  required
                />
              </div>
            )}

            <div>
              <label className="block text-sm font-semibold text-gray-700 mb-2">
                Email Address
              </label>
              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                className="w-full px-4 py-3 rounded-xl bg-gray-50 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 transition-all outline-none"
                placeholder="name@example.com"
                required
              />
            </div>

            <div>
              <label className="block text-sm font-semibold text-gray-700 mb-2">
                Password
              </label>
              <div className="relative">
                <input
                  type={showPassword ? "text" : "password"}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  className="w-full px-4 py-3 rounded-xl bg-gray-50 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 transition-all outline-none pr-12"
                  placeholder="••••••••"
                  required
                />
                <button
                  type="button"
                  onClick={() => setShowPassword(!showPassword)}
                  className="absolute right-4 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 transition-colors"
                >
                  {showPassword ? <EyeOff size={20} /> : <Eye size={20} />}
                </button>
              </div>
            </div>

            {error && (
              <div className="p-4 rounded-xl bg-red-50 text-red-600 text-sm font-medium animate-fade-in">
                {(error as { message?: string })?.message ||
                  "An error occurred."}
              </div>
            )}

            <Button
              type="submit"
              className="w-full py-4 text-lg"
              isLoading={isLoading}
            >
              {isLogin ? "Sign In" : "Create Account"}
            </Button>
          </form>

          <div className="mt-8 text-center">
            <button
              onClick={() => {
                setIsLogin(!isLogin);
                setEmail("");
                setPassword("");
                setName("");
              }}
              className="text-gray-500 hover:text-blue-600 font-medium transition-colors text-sm"
            >
              {isLogin
                ? "Don't have an account? Sign up"
                : "Already have an account? Sign in"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
