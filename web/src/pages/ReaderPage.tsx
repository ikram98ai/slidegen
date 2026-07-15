import React, { useEffect, useState, useCallback } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { useSubjectChapters } from "../hooks/useAppQueries";
import { LayoutTemplate, Loader2, Rows } from "lucide-react";
import { SlidesViewer } from "../components/SlidesViewer";
import { ToCSidbar } from "../components/ToCSidebar";

const ViewMode = {
  VERTICAL: "vertical",
  HORIZONTAL: "horizontal",
} as const;
type ViewMode = (typeof ViewMode)[keyof typeof ViewMode];

export const ReaderPage: React.FC = () => {
  const { id } = useParams<{ id: string }>();

  const navigate = useNavigate();
  const { data: subject, isPending } = useSubjectChapters(id);

  const [activeChapterId, setActiveChapterId] = useState<string>();
  const [viewMode, setViewMode] = useState<ViewMode>(ViewMode.HORIZONTAL);
  const [currentHorizontalIndex, setCurrentHorizontalIndex] = useState(0);

  // Resolve the active chapter from the live query data (not a stale
  // snapshot) so processing status and progress update as polling refetches.
  const activeViewerChapter = subject?.chapters?.find(
    (c) => c.id === activeChapterId
  );

  const resetViewer = useCallback(() => {
    setActiveChapterId(undefined);
    setViewMode(ViewMode.HORIZONTAL);
    setCurrentHorizontalIndex(0);
  }, []);

  useEffect(() => {
    return () => resetViewer();
  }, [id, resetViewer]);

  const closeSubject = () => {
    navigate("/");
  };

  const renderViewToggle = () => (
    <div className="flex items-center bg-gray-100 p-1 rounded-lg">
      <button
        onClick={() => setViewMode(ViewMode.VERTICAL)}
        className={`p-2 rounded-md transition-all ${
          viewMode === ViewMode.VERTICAL
            ? "bg-white shadow text-gray-900"
            : "text-gray-500 hover:text-gray-700"
        }`}
        title="List View"
      >
        <Rows size={18} />
      </button>
      <button
        onClick={() => setViewMode("horizontal")}
        className={`p-2 rounded-md transition-all ${
          viewMode === "horizontal"
            ? "bg-white shadow text-gray-900"
            : "text-gray-500 hover:text-gray-700"
        }`}
        title="Presentation View"
      >
        <LayoutTemplate size={18} />
      </button>
    </div>
  );

  if (isPending)
    return (
      <div className="flex items-center justify-center h-[calc(100vh-4rem)] text-gray-400">
        <Loader2 size={32} className="animate-spin mr-3 text-blue-500" />
        Loading...
      </div>
    );

  if (!subject)
    return (
      <div className="flex items-center justify-center h-[calc(100vh-4rem)] text-gray-500">
        No subject found
      </div>
    );

  return (
    <div className="flex h-[calc(100vh-4rem)] overflow-hidden bg-gray-50">
      {/* Sidebar */}
      <ToCSidbar
        subject={subject}
        chapters={subject.chapters || []}
        activeViewerChapter={activeViewerChapter}
        onSetActiveViewerChapter={(chapter) => setActiveChapterId(chapter?.id)}
        onSetCurrentHorizontalIndex={setCurrentHorizontalIndex}
        onCloseSubject={closeSubject}
      />

      {/* Main Content Area */}
      <div className="flex-1 overflow-y-auto p-8 md:p-12 scroll-smooth bg-gray-50/50">
        <div className="max-w-6xl mx-auto h-full flex flex-col">
          {activeViewerChapter === undefined ? (
            <div className="flex items-center justify-center h-full text-gray-400">
              Select a chapter to begin
            </div>
          ) : (
            <div className="animate-fade-in flex-1 flex flex-col">
              <div className="flex justify-between items-end mb-8">
                <div>
                  <h1 className="text-3xl font-bold text-gray-900">
                    {activeViewerChapter?.title}
                  </h1>
                </div>

                {renderViewToggle()}
              </div>

              {activeViewerChapter && (
                <SlidesViewer
                  viewMode={viewMode}
                  currentHorizontalIndex={currentHorizontalIndex}
                  setCurrentHorizontalIndex={setCurrentHorizontalIndex}
                  subjectId={subject.id}
                  chapter={activeViewerChapter}
                />
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
