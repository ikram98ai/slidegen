import React from 'react';
import { useNavigate } from 'react-router-dom';
import { NavBar } from '../components/NavBar';
import { SubjectList } from '../components/SubjectList';
import { useSubjects } from '../hooks/useAppQueries';

export const FilesPage: React.FC = () => {
  const navigate = useNavigate();
  const { data: subjectsList, isLoading: isLoadingSubjects } = useSubjects();

  const handleSubjectClick = (subject: any) => {
     navigate(`/subject/${subject.id}`);
  };

  return (
    <>
      <div className="pt-8">
        <NavBar />
      </div>
      <SubjectList
        subjects={subjectsList}
        isLoading={isLoadingSubjects}
        onSubjectClick={handleSubjectClick}
      />
    </>
  );
};
