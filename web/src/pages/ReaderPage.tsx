import React, { useEffect } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { useViewerStore } from "../store/viewerStore";
import { BookReader } from "../components/BookReader";
import { ReportReader } from "../components/ReportReader";
import { DocType } from "../types";
import { useSubject } from "../hooks/useAppQueries";

export const ReaderPage: React.FC = () => {
  const { id } = useParams<{ id: string }>();

  const navigate = useNavigate();
  const { data: subject } = useSubject(parseInt(id));
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

  const closeSubject = () => {
    navigate("/");
  };
  
  if (!subject) return <div>No Subject found</div>;

  return subject?.type === DocType.BOOK ? (
    <BookReader
      subject={subject}
      activeViewerChapterId={activeChapterId}
      setActiveViewerChapterId={setActiveChapterId}
      viewMode={viewMode}
      setViewMode={setViewMode}
      currentHorizontalIndex={currentHorizontalIndex}
      setCurrentHorizontalIndex={setCurrentHorizontalIndex}
      onClose={closeSubject}
    />
  ) : (
    <ReportReader
      subject={subject}
      viewMode={viewMode}
      setViewMode={setViewMode}
      currentHorizontalIndex={currentHorizontalIndex}
      setCurrentHorizontalIndex={setCurrentHorizontalIndex}
      onClose={closeSubject}
    />
  );
};
