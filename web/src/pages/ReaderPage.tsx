import React, { useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useSubjectDetails } from '../hooks/useAppQueries';
import { useViewerStore } from '../store/viewerStore';
import { BookReader } from '../components/BookReader';
import { ReportReader } from '../components/ReportReader';
import { DocType } from '../types';
import { lessonsApi } from '../services/api';

export const ReaderPage: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: subjectDetails } = useSubjectDetails(id || null);
  
  const {
    activeChapterId,
    viewMode,
    currentHorizontalIndex,
    setActiveChapterId,
    setViewMode,
    setCurrentHorizontalIndex,
    reset: resetViewer,
  } = useViewerStore();

  useEffect(() => {
    resetViewer();
    return () => resetViewer();
  }, [id, resetViewer]);

  // Set active chapter when subject loads
  useEffect(() => {
    if (
      subjectDetails &&
      subjectDetails.type === DocType.BOOK &&
      subjectDetails.chapters.length > 0 &&
      !activeChapterId
    ) {
      setActiveChapterId(subjectDetails.chapters[0].id);
    }
  }, [subjectDetails, activeChapterId, setActiveChapterId]);

  const closeSubject = () => {
    navigate('/');
  };

  const generateSlidesForActiveChapter = async () => {
    if (!subjectDetails || !activeChapterId) return;
    const lessonId = parseInt(activeChapterId);
    if (isNaN(lessonId)) return;
    try {
      await lessonsApi.generateSlides(lessonId);
      alert("Slides generation started. Please check back in a few moments.");
    } catch {
      alert("Failed to start slides generation.");
    }
  };

  if (!subjectDetails) return <div>Loading...</div>;

  return subjectDetails.type === DocType.BOOK ? (
    <BookReader
      subject={subjectDetails}
      activeViewerChapterId={activeChapterId}
      setActiveViewerChapterId={setActiveChapterId}
      viewMode={viewMode}
      setViewMode={setViewMode}
      currentHorizontalIndex={currentHorizontalIndex}
      setCurrentHorizontalIndex={setCurrentHorizontalIndex}
      onClose={closeSubject}
      onGenerateSlides={generateSlidesForActiveChapter}
    />
  ) : (
    <ReportReader
      doc={subjectDetails}
      viewMode={viewMode}
      setViewMode={setViewMode}
      currentHorizontalIndex={currentHorizontalIndex}
      setCurrentHorizontalIndex={setCurrentHorizontalIndex}
      onClose={closeSubject}
    />
  );
};
