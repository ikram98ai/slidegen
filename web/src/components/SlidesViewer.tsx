import React from "react";
import { ChevronLeft, ChevronRight, Layout } from "lucide-react";
import { SlideCard } from "./SlideCard";
import { useChapterSlides } from "../hooks/useAppQueries";
import { Button } from "./ui/Button";
import { chaptersApi } from "../services/api";

interface SlidesViewerProps {
  viewMode: "vertical" | "horizontal";
  currentHorizontalIndex: number;
  setCurrentHorizontalIndex: (index: number) => void;
  chapterId: number;
}

export const SlidesViewer: React.FC<SlidesViewerProps> = ({
  viewMode,
  currentHorizontalIndex,
  setCurrentHorizontalIndex,
  chapterId,
}) => {
  const { data: slides } = useChapterSlides(chapterId);

  const generateSlidesForActiveChapter = async () => {
    if (isNaN(chapterId)) return;
    try {
      await chaptersApi.generateSlides(chapterId);
      alert("Slides generation started. Please check back in a few moments.");
    } catch {
      alert("Failed to start slides generation.");
    }
  };

  if (viewMode === "vertical") {
    return (
      <div className="grid grid-cols-1 gap-8">
        {slides?.map((slide, idx) => (
          <SlideCard
            key={idx}
            slide={slide}
            index={idx}
            total={slides.length}
          />
        ))}
      </div>
    );
  } else {
    // Horizontal (Presentation) Mode
    if (slides === undefined)
      return (
        <div className="flex flex-col items-center justify-center flex-1 bg-white rounded-3xl shadow-sm border border-gray-100 min-h-[400px]">
          <Layout className="w-16 h-16 text-gray-300 mb-4" />
          <p className="text-gray-500 mb-6">
            Ready to create content for this section.
          </p>
          <Button onClick={generateSlidesForActiveChapter}>
            Generate Slides
          </Button>
        </div>
      );

    const currentSlide = slides[currentHorizontalIndex];

    return (
      <div className="flex flex-col h-full items-center justify-center">
        <div className="w-full max-w-4xl min-h-[500px] relative">
          <SlideCard
            key={currentSlide.id ?? currentHorizontalIndex}
            slide={currentSlide}
            index={currentHorizontalIndex}
            total={slides.length}
          />

          {/* Navigation Buttons */}
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

        {/* Dots Indicator */}
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
  }
};
