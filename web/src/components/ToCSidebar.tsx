import { ChevronLeft, PlusCircle } from "lucide-react";
import type { ChapterResponse, SubjectResponse } from "../types";
import { AddChpater } from "./AddChapter";
import { useState } from "react";

interface ToCSidbarProps {
  subject: SubjectResponse;
  chapters: ChapterResponse[];
  activeViewerChapter: ChapterResponse | undefined;
  onSetActiveViewerChapter: (chapter: ChapterResponse | undefined) => void;
  onSetCurrentHorizontalIndex: (id: number) => void;

  onCloseSubject: () => void;
}

export const ToCSidbar: React.FC<ToCSidbarProps> = ({
  subject,
  chapters,
  activeViewerChapter,
  onSetActiveViewerChapter,
  onSetCurrentHorizontalIndex,
  onCloseSubject,
}) => {
  const [isCreating, setIsCreating] = useState(false);
  return (
    <div className="w-80 bg-white border-r border-gray-200 shrink-0 flex flex-col h-full z-10">
      <div className="p-6 border-b border-gray-100 flex items-center space-x-3">
        <button
          onClick={onCloseSubject}
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
      <button
        onClick={() => setIsCreating(true)}
        className="p-4 self-center text-gray-400  hover:text-blue-500 hover:bg-blue-50 rounded-full transition-all"
        title="Add Chapter Manually"
      >
        <PlusCircle size={30} />
      </button>

      <div className="overflow-y-auto flex-1 p-4 space-y-2">
        <AddChpater
          subjectId={subject.id}
          isCreating={isCreating}
          onSetIsCreating={setIsCreating}
        />
        {chapters?.map((chapter, index) => (
          <button
            key={chapter.id}
            onClick={() => {
              onSetActiveViewerChapter(chapter);
              onSetCurrentHorizontalIndex(0);
            }}
            className={`
                 w-full text-left p-4 rounded-xl transition-all duration-200 flex items-start space-x-3
                 ${
                   activeViewerChapter?.id === chapter.id
                     ? "bg-blue-50 border-blue-200 shadow-sm"
                     : "hover:bg-gray-50 border border-transparent"
                 }`}
          >
            <div
              className={`
                  mt-1 shrink-0 w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold
                  ${
                    activeViewerChapter?.id === chapter.id
                      ? "bg-blue-500 text-white"
                      : "bg-gray-200 text-gray-500"
                  } `}
            >
              {index + 1}
            </div>
            <div>
              <h3
                className={`font-semibold text-sm ${
                  activeViewerChapter?.id === chapter.id
                    ? "text-blue-900"
                    : "text-gray-700"
                }`}
              >
                {chapter.title}
              </h3>
              <span className="text-xs text-gray-400">page: {chapter.page_start} - {chapter.page_end}</span>
            </div>
          </button>
        ))}
      </div>
    </div>
  );
};
