import React from "react";
import { Sparkles, LogIn, LogOut } from "lucide-react";
import { useAuthStore } from '../store/authStore';

import { Button } from "./Button";

interface HeaderProps {
  isAuthenticated: boolean;
  userName?: string;
  dp?:string
  onLogoClick: () => void;
  onSignInClick: () => void;
  onProfileClick: () => void;
}

export const Header: React.FC<HeaderProps> = ({
  isAuthenticated,
  userName,
  dp,
  onLogoClick,
  onSignInClick,
  onProfileClick,
}) => {
    const { logout } = useAuthStore();
  
  return (
    <header className="sticky top-0 z-50 bg-white/80 backdrop-blur-md border-b border-gray-200">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
        <div
          className="flex items-center space-x-2 cursor-pointer"
          onClick={onLogoClick}
        >
          <div className="bg-black text-white p-1.5 rounded-lg">
            <Sparkles size={18} />
          </div>
          <span className="text-lg font-bold tracking-tight text-gray-900">
            Lumina Learn
          </span>
        </div>

        {isAuthenticated ? (
          <div className="flex items-center space-x-4">
            <div
              className="w-8 h-8 rounded-full border border-gray-300 overflow-hidden bg-gray-100 flex items-center justify-center cursor-pointer"
              onClick={onProfileClick}
            >
            <img
              src={dp || `https://ui-avatars.com/api/?name=${userName}&background=random`}
              alt="Profile"
              className=" rounded-full object-cover"
            />
            </div>
            <button
              onClick={logout}
              className="p-2 text-gray-400 hover:text-red-500 transition-colors"
              title="Sign Out"
            >
              <LogOut size={20} />
            </button>
          </div>
        ) : (
          <Button variant="ghost" onClick={onSignInClick} className="text-sm">
            <LogIn size={16} className="mr-2" />
            Sign In
          </Button>
        )}
      </div>
    </header>
  );
};
