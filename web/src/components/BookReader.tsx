import React from 'react';
import { ChevronLeft, Layout, LayoutTemplate, Rows } from 'lucide-react';
import type { StoredDocument } from '../types';
import { Button } from './Button';
import { SlidesViewer } from './SlidesViewer';

interface BookReaderProps {
  doc: StoredDocument;
  activeViewerChapterId: string | null;
  setActiveViewerChapterId: (id: string) => void;
  viewMode: 'vertical' | 'horizontal';
  setViewMode: (mode: 'vertical' | 'horizontal') => void;
  currentHorizontalIndex: number;
  setCurrentHorizontalIndex: (index: number) => void;
  onClose: () => void;
  onGenerateSlides: () => void;
}

export const BookReader: React.FC<BookReaderProps> = ({
  doc,
  activeViewerChapterId,
  setActiveViewerChapterId,
  viewMode,
  setViewMode,
  currentHorizontalIndex,
  setCurrentHorizontalIndex,
  onClose,
  onGenerateSlides
}) => {
  const activeChapter = doc.chapters.find(c => c.id === activeViewerChapterId);

  const renderViewToggle = () => (
    <div className="flex items-center bg-gray-100 p-1 rounded-lg">
      <button 
        onClick={() => setViewMode('vertical')}
        className={`p-2 rounded-md transition-all ${viewMode === 'vertical' ? 'bg-white shadow text-gray-900' : 'text-gray-500 hover:text-gray-700'}`}
        title="List View"
      >
        <Rows size={18} />
      </button>
      <button 
        onClick={() => setViewMode('horizontal')}
        className={`p-2 rounded-md transition-all ${viewMode === 'horizontal' ? 'bg-white shadow text-gray-900' : 'text-gray-500 hover:text-gray-700'}`}
        title="Presentation View"
      >
        <LayoutTemplate size={18} />
      </button>
    </div>
  );

  return (
    <div className="flex h-[calc(100vh-4rem)] overflow-hidden bg-gray-50">
      {/* Sidebar */}
      <div className="w-80 bg-white border-r border-gray-200 flex-shrink-0 flex flex-col h-full z-10">
         <div className="p-6 border-b border-gray-100 flex items-center space-x-3">
           <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-full transition-colors text-gray-500">
              <ChevronLeft size={20} />
           </button>
           <div>
              <h2 className="text-sm font-bold text-gray-900 uppercase tracking-wide">Table of Contents</h2>
              <p className="text-xs text-gray-500 truncate max-w-[180px]">{doc.title}</p>
           </div>
         </div>
         <div className="overflow-y-auto flex-1 p-4 space-y-2">
           {doc.chapters.map((chapter, index) => (
             <button
               key={chapter.id}
               onClick={() => {
                   setActiveViewerChapterId(chapter.id);
                   setCurrentHorizontalIndex(0);
               }}
               className={`
                 w-full text-left p-4 rounded-xl transition-all duration-200 flex items-start space-x-3
                 ${activeViewerChapterId === chapter.id 
                   ? 'bg-blue-50 border-blue-200 shadow-sm' 
                   : 'hover:bg-gray-50 border border-transparent'
                 }
               `}
             >
               <div className={`
                  mt-1 flex-shrink-0 w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold
                  ${activeViewerChapterId === chapter.id ? 'bg-blue-500 text-white' : 'bg-gray-200 text-gray-500'}
               `}>
                 {index + 1}
               </div>
               <div>
                 <h3 className={`font-semibold text-sm ${activeViewerChapterId === chapter.id ? 'text-blue-900' : 'text-gray-700'}`}>
                   {chapter.title}
                 </h3>
                 <p className="text-xs text-gray-500 mt-1 line-clamp-2 leading-relaxed">
                   {chapter.description}
                 </p>
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
                      <h1 className="text-3xl font-bold text-gray-900">{activeChapter.title}</h1>
                      <p className="text-gray-600 mt-2 text-lg">{activeChapter.description}</p>
                  </div>
                  {activeChapter.slides && activeChapter.slides.length > 0 && renderViewToggle()}
               </div>

               {activeChapter.isLoadingSlides ? (
                 <div className="flex flex-col items-center justify-center flex-1 bg-white/50 rounded-3xl border border-dashed border-gray-300 min-h-[400px]">
                   <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-blue-500 mb-4"></div>
                   <p className="text-gray-500 font-medium">Generating slides for this chapter...</p>
                 </div>
               ) : activeChapter.slides && activeChapter.slides.length > 0 ? (
                  <SlidesViewer 
                    slides={activeChapter.slides}
                    viewMode={viewMode}
                    currentHorizontalIndex={currentHorizontalIndex}
                    setCurrentHorizontalIndex={setCurrentHorizontalIndex}
                    documentId={doc.id}
                    chapterId={activeChapter.id}
                  />
               ) : (
                 <div className="flex flex-col items-center justify-center flex-1 bg-white rounded-3xl shadow-sm border border-gray-100 min-h-[400px]">
                   <Layout className="w-16 h-16 text-gray-300 mb-4" />
                   <p className="text-gray-500 mb-6">Ready to create content for this section.</p>
                   <Button onClick={onGenerateSlides}>
                     Generate Slides
                   </Button>
                 </div>
               )}
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
