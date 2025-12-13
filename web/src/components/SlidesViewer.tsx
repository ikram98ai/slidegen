import React from 'react';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import type { Slide } from '../types';
import { SlideCard } from './SlideCard';

interface SlidesViewerProps {
  slides: Slide[];
  viewMode: 'vertical' | 'horizontal';
  currentHorizontalIndex: number;
  setCurrentHorizontalIndex: (index: number) => void;
  documentId: string;
  chapterId?: string;
}

export const SlidesViewer: React.FC<SlidesViewerProps> = ({
  slides,
  viewMode,
  currentHorizontalIndex,
  setCurrentHorizontalIndex,
  documentId,
  chapterId
}) => {
  if (viewMode === 'vertical') {
    return (
      <div className="grid grid-cols-1 gap-8">
        {slides.map((slide, idx) => (
          <SlideCard 
            key={idx} 
            slide={slide} 
            index={idx} 
            total={slides.length} 
            documentId={documentId}
            chapterId={chapterId}
          />
        ))}
      </div>
    );
  } else {
    // Horizontal (Presentation) Mode
    const currentSlide = slides[currentHorizontalIndex];
    return (
      <div className="flex flex-col h-full items-center justify-center">
          <div className="w-full max-w-4xl min-h-[500px] relative">
            <SlideCard 
              key={currentSlide.id ?? currentHorizontalIndex}
              slide={currentSlide} 
              index={currentHorizontalIndex} 
              total={slides.length} 
              documentId={documentId}
              chapterId={chapterId}
            />
            
            {/* Navigation Buttons */}
            <button 
              onClick={() => setCurrentHorizontalIndex(Math.max(0, currentHorizontalIndex - 1))}
              disabled={currentHorizontalIndex === 0}
              className="absolute -left-16 top-1/2 -translate-y-1/2 p-3 rounded-full bg-white shadow-lg text-gray-700 disabled:opacity-30 disabled:cursor-not-allowed hover:scale-110 transition-transform"
            >
              <ChevronLeft size={24} />
            </button>
            
            <button 
              onClick={() => setCurrentHorizontalIndex(Math.min(slides.length - 1, currentHorizontalIndex + 1))}
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
                      className={`w-2 h-2 rounded-full transition-all duration-300 ${idx === currentHorizontalIndex ? 'bg-blue-600 w-6' : 'bg-gray-300'}`}
                  />
              ))}
          </div>
      </div>
    );
  }
};
