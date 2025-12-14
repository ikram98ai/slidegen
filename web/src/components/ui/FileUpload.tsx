import React, { useCallback, useState } from 'react';
import { Upload, CheckCircle2, AlertCircle } from 'lucide-react';

interface FileUploadProps {
  onFileSelect: (file: File) => void;
  selectedFile: File | null;
}

export const FileUpload: React.FC<FileUploadProps> = ({ onFileSelect, selectedFile }) => {
  const [isDragging, setIsDragging] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const validateAndSelect = React.useCallback((file: File) => {
    setError(null); // Clear previous errors
    if (file.type !== 'application/pdf') {
      setError("Please upload a PDF file.");
      return;
    }
    onFileSelect(file);
  }, [onFileSelect]);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    
    const file = e.dataTransfer.files[0];
    validateAndSelect(file);
  }, [validateAndSelect]);

  const handleFileInput = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      validateAndSelect(e.target.files[0]);
    }
  };

  return (
    <div className="w-full max-w-2xl mx-auto">
      <div
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
        className={`
          relative group cursor-pointer
          rounded-3xl border-2 border-dashed transition-all duration-300 ease-out
          flex flex-col items-center justify-center p-12 text-center
          bg-white/50 backdrop-blur-sm
          ${isDragging 
            ? 'border-blue-500 bg-blue-50/50 scale-[1.02]' 
            : 'border-gray-200 hover:border-blue-400 hover:bg-white/80'
          }
          ${selectedFile ? 'border-green-400 bg-green-50/30' : ''}
        `}
      >
        <input
          type="file"
          accept="application/pdf"
          className="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
          onChange={handleFileInput}
        />
        
        <div className="transition-transform duration-300 group-hover:scale-110 mb-6">
          {selectedFile ? (
            <div className="w-20 h-20 bg-green-100 rounded-2xl flex items-center justify-center text-green-600 shadow-sm">
               <CheckCircle2 size={40} />
            </div>
          ) : (
            <div className="w-20 h-20 bg-blue-100 rounded-2xl flex items-center justify-center text-blue-600 shadow-sm">
               <Upload size={40} />
            </div>
          )}
        </div>

        <h3 className="text-xl font-semibold text-gray-900 mb-2">
          {selectedFile ? selectedFile.name : 'Upload your PDF'}
        </h3>
        
        <p className="text-gray-500 text-sm max-w-xs mx-auto leading-relaxed">
          {selectedFile 
            ? 'File ready for analysis. Click "Analyze" below.' 
            : 'Drag and drop or click to browse. We support educational materials and reports.'
          }
        </p>

        {error && (
            <div className="absolute -bottom-12 left-0 right-0 flex items-center justify-center text-red-500 text-sm animate-fade-in">
                <AlertCircle size={16} className="mr-2" />
                {error}
            </div>
        )}
      </div>
    </div>
  );
};