import React, { useState } from "react";
import {
  BookOpen,
  FileText,
  Clock,
  Pencil,
  Trash2,
  Eye,
  EyeOff,
} from "lucide-react";
import type { SubjectResponse } from "../types";
import { DocType } from "../types";
import { useUpdateSubject, useDeleteSubject } from "../hooks/useAppQueries";
import { Modal } from "./Modal";
import { Button } from "./Button";

interface SubjectCardProps {
  subject: SubjectResponse;
  onClick: (subject: SubjectResponse) => void;
}

export const SubjectCard: React.FC<SubjectCardProps> = ({
  subject,
  onClick,
}) => {
  const [isEditing, setIsEditing] = useState(false);
  const [editedTitle, setEditedTitle] = useState(subject.title);

  const updateSubject = useUpdateSubject();
  const deleteSubject = useDeleteSubject();

  const handleUpdate = () => {
    updateSubject.mutate({ id: subject.id, data: { title: editedTitle } });
    setIsEditing(false);
  };

  const handleDelete = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (window.confirm("Are you sure you want to delete this subject?")) {
      deleteSubject.mutate(subject.id);
    }
  };

  const toggleVisibility = (e: React.MouseEvent) => {
    e.stopPropagation();
    updateSubject.mutate({
      id: subject.id,
      data: { is_public: !subject.is_public },
    });
  };

  return (
    <>
      <div
        onClick={() => onClick(subject)}
        className="group bg-white rounded-3xl p-6 shadow-apple-sm hover:shadow-apple-xl border border-gray-100 transition-all duration-300 cursor-pointer flex flex-col h-full"
      >
        <div className="flex justify-between items-start mb-6">
          <div
            className={`
            w-12 h-12 rounded-2xl flex items-center justify-center transition-colors duration-300
            ${
              subject.type === DocType.BOOK
                ? "bg-blue-50 text-blue-600 group-hover:bg-blue-600 group-hover:text-white"
                : "bg-purple-50 text-purple-600 group-hover:bg-purple-600 group-hover:text-white"
            }
          `}
          >
            {subject.type === DocType.BOOK ? (
              <BookOpen size={24} />
            ) : (
              <FileText size={24} />
            )}
          </div>
          <div className="flex items-center space-x-2">
            <button
              onClick={(e) => {
                e.stopPropagation();
                setIsEditing(true);
              }}
              className="p-2 rounded-full hover:bg-gray-100"
            >
              <Pencil size={16} />
            </button>
            <button
              onClick={handleDelete}
              className="p-2 rounded-full hover:bg-red-300"
            >
              <Trash2 size={16} />
            </button>
            <button
              onClick={toggleVisibility}
              className="p-2 rounded-full hover:bg-gray-100"
            >
              {subject.is_public ? <Eye size={16} /> : <EyeOff size={16} />}
            </button>
          </div>
        </div>
        <h3 className="text-xl font-bold text-gray-900 mb-2 line-clamp-2 leading-tight group-hover:text-blue-600 transition-colors">
          {subject.title}
        </h3>
        <div className="mt-auto pt-6 flex items-center justify-between text-sm text-gray-400 border-t border-gray-50">
          <div className="flex items-center space-x-2">
            <Clock size={14} />
            <span>{new Date(subject.created_at).toLocaleDateString()}</span>
          </div>
        </div>
      </div>
      <Modal isOpen={isEditing} onClose={() => setIsEditing(false)}>
        <div className="p-6">
          <h2 className="text-2xl font-bold mb-4">Edit Subject</h2>
          <input
            type="text"
            value={editedTitle}
            onChange={(e) => setEditedTitle(e.target.value)}
            className="w-full p-2 border rounded"
          />
          <div className="mt-4 flex justify-end space-x-2">
            <Button onClick={() => setIsEditing(false)} variant="secondary">
              Cancel
            </Button>
            <Button onClick={handleUpdate}>Save</Button>
          </div>
        </div>
      </Modal>
    </>
  );
};
