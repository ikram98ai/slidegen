import React from 'react';
import { useNavigate } from 'react-router-dom';
import { NavBar } from '../components/NavBar';
import { UploadTab } from '../components/UploadTab';
import { Tab } from '../types';

export const UploadPage: React.FC = () => {
  const navigate = useNavigate();

  return (
    <>
      <div className="pt-8">
        <NavBar />
      </div>
      <UploadTab onSetActiveTab={(tab) => {
          if (tab === Tab.FILES) navigate('/');
          if (tab === Tab.PROFILE) navigate('/profile');
      }} />
    </>
  );
};
