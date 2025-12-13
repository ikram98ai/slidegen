import React from 'react';
import { BookOpen, FileText } from 'lucide-react';
import { DocType } from '../types';
import { useDocumentStore } from '../store/documentStore';
import { FileUpload } from './FileUpload';
import { Button } from './Button';

interface UploadTabProps {
  onFileSelect: (file: File) => void;
  onStartAnalysis: () => void;
  isAnalyzing: boolean;
}

export const UploadTab: React.FC<UploadTabProps> = ({
  onFileSelect,
  onStartAnalysis,
  isAnalyzing
}) => {
  const { docType, setDocType, file, isAnalyzing: storeIsAnalyzing } = useDocumentStore();
  
  // Use passed isAnalyzing (from mutation) or store's isAnalyzing
  const isLoading = isAnalyzing || storeIsAnalyzing;

  return (
    <div className="flex flex-col items-center justify-center min-h-[calc(100vh-8rem)] p-6 animate-fade-in-up">
      <div className="text-center max-w-2xl mx-auto mb-12">
        <h1 className="text-4xl md:text-5xl font-bold text-gray-900 tracking-tight mb-6">
          Turn your reading into <br/>
          <span className="text-transparent bg-clip-text bg-gradient-to-r from-blue-600 to-purple-600">
            interactive mastery.
          </span>
        </h1>
        <p className="text-lg text-gray-500 font-medium leading-relaxed">
          Upload a textbook or report. Lumina analyzes the content, structures it, and designs beautiful presentation slides instantly with voice narrations.
        </p>
      </div>

      <div className="w-full max-w-3xl bg-white rounded-[2rem] shadow-apple-xl p-8 md:p-12 border border-white/20">
        <div className="flex justify-center mb-10">
          <div className="bg-gray-100/80 p-1.5 rounded-full flex space-x-1 shadow-inner">
            <button
              onClick={() => setDocType(DocType.BOOK)}
              className={`
                px-6 py-2 rounded-full text-sm font-semibold transition-all duration-200
                ${docType === DocType.BOOK 
                  ? 'bg-white text-gray-900 shadow-sm' 
                  : 'text-gray-500 hover:text-gray-700'
                }
              `}
            >
              <div className="flex items-center space-x-2">
                <BookOpen size={16} />
                <span>Book</span>
              </div>
            </button>
            <button
              onClick={() => setDocType(DocType.REPORT)}
              className={`
                px-6 py-2 rounded-full text-sm font-semibold transition-all duration-200
                ${docType === DocType.REPORT 
                  ? 'bg-white text-gray-900 shadow-sm' 
                  : 'text-gray-500 hover:text-gray-700'
                }
              `}
            >
              <div className="flex items-center space-x-2">
                <FileText size={16} />
                <span>Report</span>
              </div>
            </button>
          </div>
        </div>

        <FileUpload onFileSelect={onFileSelect} selectedFile={file} />

        <div className="mt-10 flex justify-center">
          <Button 
            disabled={!file} 
            isLoading={isLoading}
            onClick={onStartAnalysis}
            className="w-full sm:w-auto px-12 py-4 text-base"
          >
            {isLoading ? 'Uploading...' : 'Create Learning Material'}
          </Button>
        </div>
      </div>
    </div>
  );
};
