import React, { useEffect, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { SlideCard } from "./SlideCard";
import { ChapterViewer } from "./ChapterViewer";
import { useChapterSlides } from "../hooks/useAppQueries";
import type { ChapterResponse } from "../types";

interface SlidesViewerProps {
  viewMode: "vertical" | "horizontal";
  currentHorizontalIndex: number;
  setCurrentHorizontalIndex: (index: number) => void;
  subjectId: string;
  chapter: ChapterResponse;
}

export const SlidesViewer: React.FC<SlidesViewerProps> = ({
  viewMode,
  currentHorizontalIndex,
  setCurrentHorizontalIndex,
  subjectId,
  chapter,
}) => {
  const queryClient = useQueryClient();
  const chapterId = chapter.id;
  const isProcessing = chapter.processing_status === "processing";
  const { data: slides } = useChapterSlides(chapterId, false);

  const wasProcessing = useRef(isProcessing);
  useEffect(() => {
    if (wasProcessing.current && !isProcessing) {
      queryClient.invalidateQueries({ queryKey: ["chapters", chapterId] });
      queryClient.invalidateQueries({
        queryKey: ["chapter-embed", subjectId, chapterId],
      });
    }
    wasProcessing.current = isProcessing;
  }, [isProcessing, chapterId, subjectId, queryClient]);

  if (chapter.package_key || isProcessing || (slides && slides.length === 0)) {
    return <ChapterViewer subjectId={subjectId} chapter={chapter} />;
  }

  if (!slides) {
    return null;
  }

  if (viewMode === "vertical") {
    return (
      <div className="grid grid-cols-1 gap-8">
        {slides.map((slide, idx) => (
          <SlideCard
            key={slide.id ?? idx}
            chapterId={chapterId}
            slide={slide}
            index={idx}
            total={slides.length}
          />
        ))}
      </div>
    );
  }

  const currentSlide = slides[currentHorizontalIndex];

  return (
    <div className="flex flex-col h-full items-center justify-center">
      <div className="w-full max-w-4xl min-h-[500px] relative">
        <SlideCard
          key={currentSlide.id ?? currentHorizontalIndex}
          chapterId={chapterId}
          slide={currentSlide}
          index={currentHorizontalIndex}
          total={slides.length}
        />

        <button
          onClick={() =>
            setCurrentHorizontalIndex(Math.max(0, currentHorizontalIndex - 1))
          }
          disabled={currentHorizontalIndex === 0}
          className="absolute -left-16 top-1/2 -translate-y-1/2 p-3 rounded-full bg-white shadow-lg text-gray-700 disabled:opacity-30 disabled:cursor-not-allowed hover:scale-110 transition-transform"
        >
          <ChevronLeft size={24} />
        </button>

        <button
          onClick={() =>
            setCurrentHorizontalIndex(
              Math.min(slides.length - 1, currentHorizontalIndex + 1)
            )
          }
          disabled={currentHorizontalIndex === slides.length - 1}
          className="absolute -right-16 top-1/2 -translate-y-1/2 p-3 rounded-full bg-white shadow-lg text-gray-700 disabled:opacity-30 disabled:cursor-not-allowed hover:scale-110 transition-transform"
        >
          <ChevronRight size={24} />
        </button>
      </div>

      <div className="flex space-x-2 mt-8">
        {slides.map((_, idx) => (
          <button
            key={idx}
            onClick={() => setCurrentHorizontalIndex(idx)}
            className={`w-2 h-2 rounded-full transition-all duration-300 ${
              idx === currentHorizontalIndex
                ? "bg-blue-600 w-6"
                : "bg-gray-300"
            }`}
          />
        ))}
      </div>
    </div>
  );
};
