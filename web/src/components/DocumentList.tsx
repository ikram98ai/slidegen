import React from 'react';
import { Layout } from 'lucide-react';
import type { StoredDocument } from '../types';
import { DocumentCard } from './DocumentCard';
import { Button } from './Button';

interface DocumentListProps {
  docs: StoredDocument[] | undefined;
  isLoading: boolean;
  emptyMsg: string;
  onDocumentClick: (doc: StoredDocument) => void;
  showUploadPrompt: boolean;
  onUploadClick: () => void;
}

export const DocumentList: React.FC<DocumentListProps> = ({
  docs,
  isLoading,
  emptyMsg,
  onDocumentClick,
  showUploadPrompt,
  onUploadClick
}) => {
  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
       {!docs || docs.length === 0 ? (
         <div className="text-center py-20">
            <div className="bg-gray-100 w-16 h-16 rounded-full flex items-center justify-center mx-auto mb-4">
                <Layout className="text-gray-400" />
            </div>
            <h3 className="text-lg font-medium text-gray-900">{isLoading ? 'Loading library...' : emptyMsg}</h3>
            {showUploadPrompt && (
                <Button variant="ghost" className="mt-4" onClick={onUploadClick}>
                    Upload your first document
                </Button>
            )}
         </div>
       ) : (
         <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-8 animate-fade-in">
           {docs.map(doc => (
             <DocumentCard key={doc.id} doc={doc} onClick={onDocumentClick} />
           ))}
         </div>
       )}
    </div>
  );
};
