import React from 'react';
import { LayoutGrid, UploadCloud, UserCircle, LogOut } from 'lucide-react';
import { Tab } from '../types';
import { useAuthStore } from '../store/authStore';
import { useUIStore } from '../store/uiStore';

export const NavBar: React.FC = () => {
  const { activeTab, setActiveTab, setShowAuthModal } = useUIStore();
  const { isAuthenticated, logout } = useAuthStore();
  
  const handleTabChange = (tab: Tab) => {
    if ((tab === Tab.UPLOAD || tab === Tab.PROFILE) && !isAuthenticated) {
      setShowAuthModal(true);
      return;
    }
    setActiveTab(tab);
  };

  const tabs = [
    { id: Tab.FILES, label: 'Explore', icon: LayoutGrid },
    { id: Tab.UPLOAD, label: 'Upload', icon: UploadCloud },
    { id: Tab.PROFILE, label: 'Profile', icon: UserCircle },
  ];

  return (
    <div className="flex justify-center w-full mb-8 relative">
      <div className="bg-white/80 backdrop-blur-md p-1.5 rounded-full shadow-apple-sm border border-gray-200/50 flex space-x-1">
        {tabs.map((tab) => {
          const isActive = activeTab === tab.id;
          const Icon = tab.icon;
          return (
            <button
              key={tab.id}
              onClick={() => handleTabChange(tab.id)}
              className={`
                relative flex items-center space-x-2 px-6 py-2.5 rounded-full text-sm font-medium transition-all duration-300 ease-out
                ${isActive 
                  ? 'text-gray-900 shadow-apple-sm' 
                  : 'text-gray-500 hover:text-gray-900 hover:bg-gray-50'
                }
              `}
            >
              {isActive && (
                <div className="absolute inset-0 bg-white rounded-full shadow-sm -z-10 animate-fade-in" />
              )}
              <Icon size={18} className={isActive ? 'text-blue-600' : 'text-gray-400'} />
              <span>{tab.label}</span>
            </button>
          );
        })}
      </div>
      

    </div>
  );
};