import React from 'react';
import { ChevronLeft, LayoutTemplate, Rows } from 'lucide-react';
import type { StoredSubject } from '../types';
import { SlidesViewer } from './SlidesViewer';

interface ReportReaderProps {
  doc: StoredSubject;
  viewMode: 'vertical' | 'horizontal';
  setViewMode: (mode: 'vertical' | 'horizontal') => void;
  currentHorizontalIndex: number;
  setCurrentHorizontalIndex: (index: number) => void;
  onClose: () => void;
}

export const ReportReader: React.FC<ReportReaderProps> = ({
  doc,
  viewMode,
  setViewMode,
  currentHorizontalIndex,
  setCurrentHorizontalIndex,
  onClose
}) => {
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
    <div className="min-h-[calc(100vh-4rem)] bg-gray-50 p-8 md:p-12 overflow-y-auto">
      <div className="max-w-6xl mx-auto min-h-full flex flex-col">
        <div className="mb-10 flex justify-between items-center">
            <div className="flex items-center space-x-4">
                <button 
                    onClick={onClose} 
                    className="p-2 hover:bg-gray-200 rounded-full transition-colors text-gray-600"
                >
                    <ChevronLeft size={24} />
                </button>
                <div>
                    <h1 className="text-3xl font-bold text-gray-900">{doc.title}</h1>
                    <p className="text-gray-500 mt-1">Executive Report Summary</p>
                </div>
            </div>
            {doc.reportSlides && doc.reportSlides.length > 0 && renderViewToggle()}
        </div>
        
        <div className="flex-1">
            <SlidesViewer 
                slides={doc.reportSlides}
                viewMode={viewMode}
                currentHorizontalIndex={currentHorizontalIndex}
                setCurrentHorizontalIndex={setCurrentHorizontalIndex}
                subjectId={doc.id}
            />
        </div>
      </div>
    </div>
  );
};
