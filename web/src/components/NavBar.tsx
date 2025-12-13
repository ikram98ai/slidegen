import React from 'react';
import { LayoutGrid, UploadCloud, UserCircle } from 'lucide-react';
import { Tab } from '../types';
import { useAuthStore } from '../store/authStore';
import { useUIStore } from '../store/uiStore';
import { useLocation, useNavigate } from 'react-router-dom';

export const NavBar: React.FC = () => {
  const { setShowAuthModal } = useUIStore();
  const { isAuthenticated } = useAuthStore();
  const location = useLocation();
  const navigate = useNavigate();

  const getActiveTab = (path: string) => {
    if (path === '/' || path.startsWith('/subject')) return Tab.FILES;
    if (path === '/upload') return Tab.UPLOAD;
    if (path === '/profile') return Tab.PROFILE;
    return Tab.FILES;
  };

  const activeTab = getActiveTab(location.pathname);

  const handleTabChange = (tab: Tab) => {
    if ((tab === Tab.UPLOAD || tab === Tab.PROFILE) && !isAuthenticated) {
      setShowAuthModal(true);
      return;
    }
    if (tab === Tab.FILES) navigate('/');
    if (tab === Tab.UPLOAD) navigate('/upload');
    if (tab === Tab.PROFILE) navigate('/profile');
  };

  const tabs = [
    { id: Tab.FILES, label: 'Explore', icon: LayoutGrid },
    { id: Tab.UPLOAD, label: 'Upload', icon: UploadCloud },
    { id: Tab.PROFILE, label: 'Profile', icon: UserCircle },
  ];

  return (
    <div className="px-2 flex justify-center w-full mb-8 relative">
      <div className="max-w-7xl bg-white/80 backdrop-blur-md p-1.5 rounded-full shadow-apple-sm border border-gray-200/50 flex space-x-1">
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