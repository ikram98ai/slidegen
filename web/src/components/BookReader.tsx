import React from "react";
import { ChevronLeft } from "lucide-react";
import type { SubjectResponse } from "../types";
import { SlidesViewer } from "./SlidesViewer";
import { useSubjectLessons } from "../hooks/useAppQueries";

interface BookReaderProps {
  subject: SubjectResponse;
  activeViewerChapterId: number | null;
  setActiveViewerChapterId: (id: number) => void;
  viewMode: "vertical" | "horizontal";
  setViewMode: (mode: "vertical" | "horizontal") => void;
  currentHorizontalIndex: number;
  setCurrentHorizontalIndex: (index: number) => void;
  onClose: () => void;
}

export const BookReader: React.FC<BookReaderProps> = ({
  subject,
  activeViewerChapterId,
  setActiveViewerChapterId,
  viewMode,
  currentHorizontalIndex,
  setCurrentHorizontalIndex,
  onClose,
}) => {
  const {data: chapters} = useSubjectLessons(subject.id)

  const activeChapter = chapters?.find(c => c.id === activeViewerChapterId);

 

  return (
    <div className="flex h-[calc(100vh-4rem)] overflow-hidden bg-gray-50">
      {/* Sidebar */}
      <div className="w-80 bg-white border-r border-gray-200 shrink-0 flex flex-col h-full z-10">
        <div className="p-6 border-b border-gray-100 flex items-center space-x-3">
          <button
            onClick={onClose}
            className="p-2 hover:bg-gray-100 rounded-full transition-colors text-gray-500"
          >
            <ChevronLeft size={20} />
          </button>
          <div>
            <h2 className="text-sm font-bold text-gray-900 uppercase tracking-wide">
              Table of Contents
            </h2>
            <p className="text-xs text-gray-500 truncate max-w-[180px]">
              {subject.title}
            </p>
          </div>
        </div>
        <div className="overflow-y-auto flex-1 p-4 space-y-2">
          {chapters?.map((chapter, index) => (
            <button
              key={chapter.id}
              onClick={() => {
                setActiveViewerChapterId(chapter.id);
                setCurrentHorizontalIndex(0);
              }}
              className={`
                 w-full text-left p-4 rounded-xl transition-all duration-200 flex items-start space-x-3
                 ${
                   activeViewerChapterId === chapter.id
                     ? "bg-blue-50 border-blue-200 shadow-sm"
                     : "hover:bg-gray-50 border border-transparent"
                 }
               `}
            >
              <div
                className={`
                  mt-1 shrink-0 w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold
                  ${
                    activeViewerChapterId === chapter.id
                      ? "bg-blue-500 text-white"
                      : "bg-gray-200 text-gray-500"
                  }
               `}
              >
                {index + 1}
              </div>
              <div>
                <h3
                  className={`font-semibold text-sm ${
                    activeViewerChapterId === chapter.id
                      ? "text-blue-900"
                      : "text-gray-700"
                  }`}
                >
                  {chapter.title}
                </h3>

              </div>
            </button>
          ))}
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex-1 overflow-y-auto p-8 md:p-12 scroll-smooth bg-gray-50/50">
        <div className="max-w-6xl mx-auto h-full flex flex-col">
          {activeChapter ? (
            <div className="animate-fade-in flex-1 flex flex-col">
              <div className="flex justify-between items-end mb-8">
                <div>
                  <h1 className="text-3xl font-bold text-gray-900">
                    {activeChapter.title}
                  </h1>
               
                </div>

              </div>

             
                <SlidesViewer
                  viewMode={viewMode}
                  currentHorizontalIndex={currentHorizontalIndex}
                  setCurrentHorizontalIndex={setCurrentHorizontalIndex}
                  lessonId={activeChapter.id}
                />
            
            </div>
          ) : (
            <div className="flex items-center justify-center h-full text-gray-400">
              Select a chapter to begin
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
