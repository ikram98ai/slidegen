import React from 'react';
import { BookOpen, FileText, Clock, User } from 'lucide-react';
import type { StoredDocument } from '../types';
import { DocType } from '../types';

interface DocumentCardProps {
  doc: StoredDocument;
  onClick: (doc: StoredDocument) => void;
}

export const DocumentCard: React.FC<DocumentCardProps> = ({ doc, onClick }) => {
  return (
    <div 
      onClick={() => onClick(doc)}
      className="group bg-white rounded-3xl p-6 shadow-apple-sm hover:shadow-apple-xl border border-gray-100 transition-all duration-300 cursor-pointer flex flex-col h-full"
    >
      <div className="flex justify-between items-start mb-6">
        <div className={`
          w-12 h-12 rounded-2xl flex items-center justify-center transition-colors duration-300
          ${doc.type === DocType.BOOK 
            ? 'bg-blue-50 text-blue-600 group-hover:bg-blue-600 group-hover:text-white' 
            : 'bg-purple-50 text-purple-600 group-hover:bg-purple-600 group-hover:text-white'
          }
        `}>
          {doc.type === DocType.BOOK ? <BookOpen size={24} /> : <FileText size={24} />}
        </div>
        <div className="px-3 py-1 bg-gray-50 rounded-full text-xs font-semibold text-gray-500 uppercase tracking-wide">
          {doc.type}
        </div>
      </div>

      <h3 className="text-xl font-bold text-gray-900 mb-2 line-clamp-2 leading-tight group-hover:text-blue-600 transition-colors">
        {doc.title}
      </h3>

      <div className="mt-auto pt-6 flex items-center justify-between text-sm text-gray-400 border-t border-gray-50">
        <div className="flex items-center space-x-2">
          <User size={14} />
          <span className="truncate max-w-[100px]">{doc.author}</span>
        </div>
        <div className="flex items-center space-x-2">
          <Clock size={14} />
          <span>{new Date(doc.uploadDate).toLocaleDateString()}</span>
        </div>
      </div>
    </div>
  );
};